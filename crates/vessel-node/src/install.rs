//! `agent.install` and `agent.install_output` (protocol 11, ADR 0032): one
//! of the four's documented installers, run on the node when a person
//! asks, and its output read back.
//!
//! [Decision Log]
//! - Purpose: a person installs Claude Code, Codex, Gemini CLI or `OpenCode`
//!   (or an ACP adapter) on a node from any client, seeing the command
//!   before it runs and its output while it runs.
//! - Constraint: an installer runs a package's scripts as the node's user,
//!   from the network; it must not see the node's credentials, must not
//!   run beside a session of that agent (it replaces the binary the
//!   session runs), and must not run for ever.
//! - Alternatives: agent-orchestrator's list of installers per agent with
//!   their own scripts and brew (more vendors' commands to trust and keep
//!   current); GOAT's installing a registry agent on add (Vessel adds
//!   registry agents as `npx`/`uvx` commands instead, fetched by the
//!   runner at their first start).
//! - Choice: only the command `vessel init` offers (`vessel_harness::
//!   installs`), its program found on the node's search path, in the
//!   user's home, with the environment an added agent gets plus
//!   `npm_config_*` (`vessel_harness::installer`), standard input closed;
//!   one install at a time on a node; 15 minutes, then the whole process
//!   group is stopped; the first 1 MiB of output kept. While it runs,
//!   `session.create`, `session.resume` and `harness.options` for that
//!   agent answer `conflict`.
//! - Kept: the last job only, in memory, until the next or the node stops.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use bytes::Bytes;
use tokio::io::{AsyncRead, AsyncReadExt as _};
use tokio::sync::watch;
use vessel_proc::{CommandSpec, Signal, Spawned};
use vessel_proto::agents::{InstallJob, InstallOutput, InstallState, ReadInstall};
use vessel_proto::{ErrorCode, HarnessKind, TerminalBytes, WireError};

/// How long an install may run.
pub(crate) const INSTALL_LIMIT: Duration = Duration::from_secs(15 * 60);
/// The most output kept.
pub(crate) const MAX_OUTPUT: usize = 1024 * 1024;
/// The most `agent.install_output` answers at once.
const PAGE: usize = 64 * 1024;
/// The longest a read waits.
const MAX_WAIT: Duration = Duration::from_secs(30);

/// The node's last install.
#[derive(Debug, Default)]
pub(crate) struct Installs {
    last: Mutex<Option<Arc<Job>>>,
}

#[derive(Debug)]
struct Job {
    info: InstallJob,
    log: Mutex<Log>,
    /// Bumped on every write and at the end, for waiting readers.
    changed: watch::Sender<u64>,
}

#[derive(Debug)]
struct Log {
    bytes: Vec<u8>,
    truncated: bool,
    state: InstallState,
}

/// Why an install was not started.
#[derive(Debug, thiserror::Error)]
pub(crate) enum InstallError {
    #[error("an install is running on this node; wait for it to end")]
    Running,
}

impl Installs {
    /// Whether an install of `agent` is running.
    pub(crate) fn running(&self, agent: HarnessKind) -> bool {
        self.current()
            .is_some_and(|job| job.info.agent == agent && job.state() == InstallState::Running)
    }

    fn current(&self) -> Option<Arc<Job>> {
        self.last
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Start `command` as `info`, in the background.
    ///
    /// # Errors
    /// [`InstallError::Running`] while another runs.
    pub(crate) fn start(
        &self,
        info: InstallJob,
        command: CommandSpec,
    ) -> Result<InstallJob, InstallError> {
        let job = {
            let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
            if last
                .as_ref()
                .is_some_and(|job| job.state() == InstallState::Running)
            {
                return Err(InstallError::Running);
            }
            let job = Arc::new(Job {
                info: info.clone(),
                log: Mutex::new(Log {
                    bytes: Vec::new(),
                    truncated: false,
                    state: InstallState::Running,
                }),
                changed: watch::Sender::new(0),
            });
            *last = Some(Arc::clone(&job));
            job
        };
        tracing::info!(install = %info.id, agent = %info.agent, command = %info.command, "install started");
        drop(tokio::spawn(run(job, command)));
        Ok(info)
    }

    /// A page of job `request.job`'s output; with `wait`, after more is
    /// written or it ends (30 s at most).
    ///
    /// # Errors
    /// `not_found` for a job that is not the last one.
    pub(crate) async fn read(&self, request: ReadInstall) -> Result<InstallOutput, WireError> {
        let ReadInstall { job, from, wait } = request;
        let current = self
            .current()
            .filter(|current| current.info.id == job)
            .ok_or_else(|| {
                WireError::new(
                    ErrorCode::NotFound,
                    format!(
                        "no install {job} on this node (another replaced it, or the node restarted)"
                    ),
                )
            })?;
        let wait = wait.map_or(Duration::ZERO, |seconds| {
            Duration::from_secs(u64::from(seconds)).min(MAX_WAIT)
        });
        if !wait.is_zero() {
            let mut changed = current.changed.subscribe();
            let more = |job: &Job| {
                let log = job.log.lock().unwrap_or_else(PoisonError::into_inner);
                u64::try_from(log.bytes.len()).unwrap_or(u64::MAX) > from
                    || log.state != InstallState::Running
            };
            if !more(&current) {
                let _woken = tokio::time::timeout(wait, async {
                    while changed.changed().await.is_ok() {
                        if more(&current) {
                            break;
                        }
                    }
                })
                .await;
            }
        }
        Ok(current.page(from))
    }
}

impl Job {
    fn state(&self) -> InstallState {
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .state
    }

    fn page(&self, from: u64) -> InstallOutput {
        let log = self.log.lock().unwrap_or_else(PoisonError::into_inner);
        let start = usize::try_from(from)
            .unwrap_or(usize::MAX)
            .min(log.bytes.len());
        let end = start.saturating_add(PAGE).min(log.bytes.len());
        let bytes = log.bytes.get(start..end).unwrap_or_default();
        InstallOutput {
            from: u64::try_from(start).unwrap_or(u64::MAX),
            bytes: TerminalBytes::from(Bytes::copy_from_slice(bytes)),
            next: u64::try_from(end).unwrap_or(u64::MAX),
            truncated: log.truncated,
            state: log.state,
        }
    }

    /// Keep `chunk`, up to [`MAX_OUTPUT`] in all.
    fn write(&self, chunk: &[u8]) {
        {
            let mut log = self.log.lock().unwrap_or_else(PoisonError::into_inner);
            let room = MAX_OUTPUT.saturating_sub(log.bytes.len());
            let kept = chunk.get(..room.min(chunk.len())).unwrap_or_default();
            log.bytes.extend_from_slice(kept);
            if kept.len() < chunk.len() {
                log.truncated = true;
            }
        }
        self.changed
            .send_modify(|count| *count = count.wrapping_add(1));
    }

    fn end(&self, state: InstallState) {
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .state = state;
        self.changed
            .send_modify(|count| *count = count.wrapping_add(1));
        tracing::info!(install = %self.info.id, ?state, "install ended");
    }
}

/// Run `command` for `job` to its end, or stop it at [`INSTALL_LIMIT`].
async fn run(job: Arc<Job>, command: CommandSpec) {
    let Ok(Spawned {
        mut child,
        stdin,
        stdout,
        stderr,
    }) = vessel_proc::spawn(&command)
    else {
        job.write(b"vessel: the installer did not start\n");
        job.end(InstallState::Failed { code: None });
        return;
    };
    drop(stdin);
    let pumped = async {
        tokio::join!(pump(stdout, &job), pump(stderr, &job));
        child.wait().await
    };
    match tokio::time::timeout(INSTALL_LIMIT, pumped).await {
        Ok(Ok(Some(0))) => job.end(InstallState::Succeeded),
        Ok(Ok(code)) => job.end(InstallState::Failed { code }),
        Ok(Err(_error)) => job.end(InstallState::Failed { code: None }),
        Err(_elapsed) => {
            if let Err(error) = child.signal(Signal::Kill) {
                tracing::debug!(%error, "the installer had already ended");
            }
            let _code = child.wait().await;
            job.write(b"\nvessel: stopped after 15 minutes\n");
            job.end(InstallState::TimedOut);
        }
    }
}

/// Copy `stream` into the job's log until it ends.
async fn pump(mut stream: impl AsyncRead + Unpin, job: &Job) {
    let mut buffer = vec![0_u8; 8 * 1024];
    loop {
        match stream.read(&mut buffer).await {
            Ok(0) | Err(_) => return,
            Ok(read) => job.write(buffer.get(..read).unwrap_or_default()),
        }
    }
}
