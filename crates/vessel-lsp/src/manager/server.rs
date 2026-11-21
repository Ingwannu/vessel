//! One language server: start it, keep a query that was already written
//! running to its timeout, and shut the process down.

use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use vessel_proto::lsp::Language;

use crate::error::LspError as Error;
use crate::process::{Launch, Process};
use crate::protocol::initialize as init;
use crate::sync::Watcher;

use super::{Config, Key, Tools, Work};

/// The project set and configuration identity a server generation uses.
#[derive(Debug, Default, PartialEq, Eq)]
struct Generation {
    projects: Vec<PathBuf>,
    stamp: Option<crate::policy::Stamp>,
}

/// Recheck the policy and identify the generation the server must run.
fn plan(key: &Key, home: Option<&Path>) -> Result<Generation, Error> {
    let projects = crate::policy::check(&key.workspace, key.language, key.full, home)?;
    let stamp = crate::policy::stamp(&key.workspace, &projects)?;
    Ok(Generation {
        projects,
        stamp: Some(stamp),
    })
}

pub(super) async fn run(
    key: Key,
    config: Config,
    shutdown: CancellationToken,
    retire: CancellationToken,
    mut inbox: mpsc::Receiver<Work>,
    tools: Tools,
) {
    let Ok((watcher, mut changes)) = Watcher::new().await else {
        return;
    };
    let mut process: Option<Process> = None;
    let mut generation = Generation::default();
    let mut retry_at = Instant::now();
    let mut backoff = config.restart_backoff.min(Duration::from_secs(30));
    let mut idle_at = Instant::now()
        .checked_add(config.idle_timeout)
        .unwrap_or_else(Instant::now);
    loop {
        tokio::select! {
            () = shutdown.cancelled() => break,
            () = retire.cancelled() => break,
            () = tokio::time::sleep_until(idle_at) => break,
            event = changes.changed(), if process.is_some() => {
                if event.is_err() { break; }
                drop(changes.borrow_and_update());
                if let Some(server) = &mut process {
                    let result = tokio::time::timeout(config.request_timeout, server.refresh_all(&watcher)).await;
                    if !matches!(result, Ok(Ok(()))) { process = None; }
                }
            }
            message = receive_event(&mut process) => {
                if let Some(server) = &mut process {
                    let result = match message { Some(Ok(message)) => tokio::time::timeout(config.request_timeout, server.event(message)).await.ok(), Some(Err(_)) | None => None };
                    if !matches!(result, Some(Ok(()))) {
                        process = None;
                        retry_at = Instant::now().checked_add(backoff).unwrap_or_else(Instant::now);
                        backoff = backoff.saturating_mul(2).min(Duration::from_secs(30));
                    }
                }
            }
            work = inbox.recv() => {
                let Some(mut work) = work else { break; };
                if work.reply.is_closed() { continue; }
                if Instant::now() < retry_at { drop(work.reply.send(Err(Error::Unavailable))); continue; }
                let policy_key = key.clone();
                let policy_home = config.registry.home.clone();
                let planned = tokio::task::spawn_blocking(move || {
                    plan(&policy_key, policy_home.as_deref())
                }).await;
                let next = match planned {
                    Ok(Ok(next)) => next,
                    Ok(Err(error)) => { process = None; drop(work.reply.send(Err(error))); continue; }
                    Err(error) => { process = None; drop(work.reply.send(Err(error.into()))); continue; }
                };
                if next != generation { process = None; }
                generation = next;
                if process.is_none() {
                    let started = tokio::select! {
                        () = shutdown.cancelled() => break,
                        // The caller gave up before the server was up. Drop
                        // the start without a backoff: a dropped query is
                        // not a crash, and the next caller should still
                        // find this workspace ready.
                        _abandoned = work.reply.closed() => continue,
                        result = launch(&key, &config, &tools, &generation.projects, &watcher) => result,
                    };
                    match started {
                        Ok(started) => process = Some(started),
                        Err(error) => {
                            retry_at = Instant::now().checked_add(backoff).unwrap_or_else(Instant::now);
                            backoff = backoff.saturating_mul(2).min(Duration::from_secs(30));
                            drop(work.reply.send(Err(error))); continue;
                        }
                    }
                }
                let Some(server) = &mut process else { break; };
                // Run a query that was already written to its timeout, even
                // if the caller has gone. Dropping it midway can leave half
                // a frame, or an answer the next query would read as its
                // own; killing the server for that made one cancellation
                // everyone's outage. A closed reply just drops the answer.
                let reply = work.reply;
                let result = tokio::select! {
                    () = shutdown.cancelled() => break,
                    result = tokio::time::timeout(config.request_timeout, server.query(work.query, &watcher)) => result.unwrap_or(Err(Error::Timeout)),
                };
                let failed = matches!(&result, Err(Error::Protocol | Error::Timeout | Error::Frame(_) | Error::Json(_) | Error::Unavailable));
                if failed {
                    process = None;
                    retry_at = Instant::now().checked_add(backoff).unwrap_or_else(Instant::now);
                    backoff = backoff.saturating_mul(2).min(Duration::from_secs(30));
                } else { backoff = config.restart_backoff.min(Duration::from_secs(30)); }
                drop(reply.send(result));
                idle_at = Instant::now().checked_add(config.idle_timeout).unwrap_or_else(Instant::now);
            }
        }
    }
    if let Some(mut process) = process {
        process.shutdown().await;
    }
}

/// Start a new server generation: resolve the toolchain once, prepare the
/// launch off the async workers and initialize the process.
async fn launch(
    key: &Key,
    config: &Config,
    tools: &Tools,
    projects: &[PathBuf],
    watcher: &Watcher,
) -> Result<Process, Error> {
    // The previous generation's documents are gone with its process.
    watcher.reset().await?;
    // Conservative Rust only: full analysis keeps rustup's own toolchain
    // selection, project files included.
    let rust = if key.language == Language::Rust && !key.full {
        let registry = config.registry.clone();
        let deadline = config.request_timeout;
        let resolved = tools
            .get_or_try_init(|| async move {
                let state = registry.clone();
                let (cwd, _tmp) = tokio::task::spawn_blocking(move || state.state()).await??;
                crate::toolchain::resolve(&registry, &cwd, deadline).await
            })
            .await?;
        Some(resolved.clone())
    } else {
        None
    };
    let registry = config.registry.clone();
    let copy = key.clone();
    let prepared = tokio::task::spawn_blocking(move || {
        registry.prepare(copy.language, &copy.workspace, copy.full, rust.as_ref())
    })
    .await??;
    Process::start(
        &prepared.command,
        Launch {
            root: key.workspace.clone(),
            language: key.language,
            settings: init::Settings::new(key.full, projects),
            request_timeout: config.request_timeout,
            typescript_engine: prepared.typescript_engine,
            python_interpreter: prepared.python_interpreter,
        },
    )
    .await
}

async fn receive_event(
    process: &mut Option<Process>,
) -> Option<Result<crate::protocol::Incoming, Error>> {
    match process {
        Some(process) => process.inbox.recv().await,
        None => std::future::pending().await,
    }
}
