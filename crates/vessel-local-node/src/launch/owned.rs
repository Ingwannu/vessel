//! RAII retains the exact group/job handle through readiness and every error path.

use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, AsyncRead, AsyncReadExt as _, BufReader};
use vessel_proc::{Child, Detached, Spawned, spawn_releasable};
use vessel_proto::local_launch::{MAX_READY_BYTES, Ready};

use super::{Launch, LaunchError};
use crate::logs::LogReader;

#[cfg(windows)]
mod bootstrap;
#[cfg(test)]
mod tests;

pub(crate) struct Owned {
    child: Child,
    logs: LogReader,
}

/// Windows carries the proven preopened connection; Unix qualifies its kernel
/// peer after reading the private home. No optional connection can bypass proof.
pub(crate) struct Started {
    pub(crate) child: Owned,
    pub(crate) ready: Ready,
    #[cfg(unix)]
    pub(crate) reaping: tokio::signal::unix::Signal,
    #[cfg(windows)]
    pub(crate) connection: vessel_client::LocalConnection,
}

/// Observation-only handle after an explicit keep-running choice.
#[derive(Debug, Clone)]
pub struct KeptNode {
    detached: Detached,
}

/// The actual direct-child exit, distinct from an RPC acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessExit(Option<i32>);

impl ProcessExit {
    /// OS exit code, absent when the process ended through a signal.
    #[must_use]
    pub const fn code(self) -> Option<i32> {
        self.0
    }
}

impl KeptNode {
    /// Direct-child reaping remains owned by the launch runtime after release.
    ///
    /// # Errors
    /// The original runtime stopped or the OS could not observe that child.
    pub async fn wait(&mut self) -> Result<ProcessExit, LaunchError> {
        self.detached
            .wait()
            .await
            .map(ProcessExit)
            .map_err(LaunchError::DetachedWait)
    }
}

impl Owned {
    pub(crate) async fn start(
        launch: &Launch,
        on_logs: impl FnOnce(LogReader),
    ) -> Result<Started, LaunchError> {
        #[cfg(unix)]
        let reaping = register_reaping()?;
        #[cfg(windows)]
        let pipe = bootstrap::pipe()?;
        let Spawned {
            child,
            stdin,
            stdout,
            stderr,
        } = spawn_releasable(&launch.command).map_err(LaunchError::Spawn)?;
        let owned = Self {
            child,
            logs: LogReader::collect(stderr),
        };
        on_logs(owned.logs());
        #[cfg(unix)]
        {
            drop(stdin);
            let line = record(stdout, MAX_READY_BYTES).await?;
            let ready = Ready::decode(&line).map_err(LaunchError::Ready)?;
            Ok(Started {
                child: owned,
                ready,
                reaping,
            })
        }
        #[cfg(windows)]
        {
            let (connection, challenge) = bootstrap::connect(pipe, stdin).await?;
            let line = record(
                stdout,
                vessel_proto::local_launch::MAX_BOOTSTRAP_READY_BYTES,
            )
            .await?;
            let ready = bootstrap::verify(&owned, &line, &challenge)?;
            Ok(Started {
                child: owned,
                ready,
                connection,
            })
        }
    }

    /// The observer proves only retained runner authority, not OS liveness.
    /// Its active state excludes a child already reaped or released here; an
    /// exited but unreaped direct child still prevents PID reuse.
    #[cfg(unix)]
    pub(crate) fn qualify_peer(
        &self,
        peer: Option<std::num::NonZeroU32>,
    ) -> Result<(), LaunchError> {
        if self.child.lifetime().is_live()
            && peer.is_some_and(|peer| peer.get() == self.child.pid())
        {
            Ok(())
        } else {
            Err(LaunchError::PeerUnverified)
        }
    }

    pub(crate) fn logs(&self) -> LogReader {
        self.logs.clone()
    }

    pub(crate) async fn wait(&mut self) -> Result<ProcessExit, LaunchError> {
        self.child
            .wait_checked()
            .await
            .map(|receipt| ProcessExit(receipt.code()))
            .map_err(LaunchError::Wait)
    }

    pub(crate) fn keep(self) -> Result<KeptNode, Box<Self>> {
        let Self { child, logs } = self;
        let detached = match child.detach() {
            Ok(detached) => detached,
            Err(refused) => {
                // Refusal is a release-capability policy result. Return the
                // recovered owner, rather than letting an error own and drop it.
                return Err(Box::new(Self {
                    child: refused.into_child(),
                    logs,
                }));
            }
        };
        let mut reaped = detached.clone();
        // Keep draining stderr until direct-child exit even after all UI handles
        // disappear. This task ends with the original launcher runtime.
        drop(tokio::spawn(async move {
            let _observed = reaped.wait().await;
            drop(logs);
        }));
        Ok(KeptNode { detached })
    }
}

#[cfg(unix)]
fn register_reaping() -> Result<tokio::signal::unix::Signal, LaunchError> {
    // A retained Child PID is evidence only while it cannot auto-reap. Install
    // before spawn to replace inherited SIG_IGN/SA_NOCLDWAIT; registration after
    // spawn is too late. Embeddings changing SIGCHLD afterward are unsupported.
    // Keep this launch prerequisite here, not in Proc's shared spawn contract.
    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::child())
        .map_err(LaunchError::Reaping)
}

async fn record(source: impl AsyncRead + Unpin, bound: usize) -> Result<String, LaunchError> {
    // The trusted managed binary emits exactly one stdout record; ordinary
    // logging stays elsewhere. Never await daemon EOF or accept a later proof.
    let limit = u64::try_from(bound.saturating_add(1)).map_err(|_range| LaunchError::ReadLimit)?;
    let mut reader = BufReader::new(source.take(limit));
    let mut line = String::new();
    let _read = reader
        .read_line(&mut line)
        .await
        .map_err(LaunchError::Read)?;
    if !reader.buffer().is_empty() {
        return Err(LaunchError::ExtraReadiness);
    }
    Ok(line)
}

/// A finite child startup includes pipe readiness and the authenticated greeting.
pub(crate) const START_DEADLINE: Duration = Duration::from_secs(30);
/// Stop acknowledgement does not substitute for this real-exit wait.
pub(crate) const STOP_DEADLINE: Duration = Duration::from_secs(15);
