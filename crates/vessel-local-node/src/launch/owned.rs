//! RAII retains the exact group/job handle through readiness and every error path.

use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, AsyncRead, AsyncReadExt as _, BufReader};
use vessel_proc::{Child, Detached, Spawned, spawn_releasable};
#[cfg(any(unix, test))]
use vessel_proto::local_launch::MAX_READY_BYTES;
use vessel_proto::local_launch::Ready;

use super::{Launch, LaunchError};
use crate::logs::LogReader;

#[cfg(windows)]
mod bootstrap;

pub(crate) struct Owned {
    child: Child,
    logs: LogReader,
    #[cfg(test)]
    witness: cleanup_witness::Dropped,
}

/// The caller owns the child outside the cancelable readiness future.
pub(crate) struct Started {
    pub(crate) child: Owned,
    pub(crate) reader: Readiness,
    #[cfg(unix)]
    pub(crate) reaping: tokio::signal::unix::Signal,
}

pub(crate) struct Readiness {
    stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
    #[cfg(windows)]
    pipe: vessel_proto::local_launch::BootstrapPipe,
}

pub(crate) struct Proof {
    pub(crate) ready: Ready,
    #[cfg(windows)]
    pub(crate) connection: vessel_client::LocalConnection,
}

impl Readiness {
    #[cfg(unix)]
    pub(crate) async fn read(self, _child: &Owned) -> Result<Proof, LaunchError> {
        drop(self.stdin);
        let line = record(self.stdout, MAX_READY_BYTES).await?;
        Ok(Proof {
            ready: Ready::decode(&line).map_err(LaunchError::Ready)?,
        })
    }

    #[cfg(windows)]
    pub(crate) async fn read(self, child: &Owned) -> Result<Proof, LaunchError> {
        let (connection, challenge) = bootstrap::connect(self.pipe, self.stdin).await?;
        let line = record(
            self.stdout,
            vessel_proto::local_launch::MAX_BOOTSTRAP_READY_BYTES,
        )
        .await?;
        Ok(Proof {
            ready: bootstrap::verify(child, &line, &challenge)?,
            connection,
        })
    }
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
    pub(crate) fn start(launch: &Launch) -> Result<Started, LaunchError> {
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
        Ok(Started {
            child: Self {
                child,
                logs: LogReader::collect(stderr),
                #[cfg(test)]
                witness: cleanup_witness::dropped(),
            },
            reader: Readiness {
                stdin,
                stdout,
                #[cfg(windows)]
                pipe,
            },
            #[cfg(unix)]
            reaping,
        })
    }

    /// Failed launch cleanup must have an owner-observed checked settlement.
    /// Fire-and-forget Drop cannot provide that evidence to the original caller.
    /// Cancellation or an unconfirmed deadline still drops this exact owner,
    /// requesting the ordinary hard-kill fallback, never a replacement-peer RPC.
    pub(crate) async fn cleanup(&mut self) -> Result<ProcessExit, LaunchError> {
        let result = match self.child.signal(vessel_proc::Signal::Terminate) {
            Ok(()) => tokio::time::timeout(CLEANUP_DEADLINE, self.wait())
                .await
                .map_err(LaunchError::CleanupDeadline)
                .and_then(std::convert::identity),
            Err(error) => Err(LaunchError::CleanupRequest(error)),
        };
        #[cfg(test)]
        cleanup_witness::settled(&result);
        result
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
        let child = self.child;
        let logs = self.logs;
        #[cfg(test)]
        let witness = self.witness;
        let detached = match child.detach() {
            Ok(detached) => detached,
            Err(refused) => {
                // Refusal is a release-capability policy result. Return the
                // recovered owner, rather than letting an error own and drop it.
                return Err(Box::new(Self {
                    child: refused.into_child(),
                    logs,
                    #[cfg(test)]
                    witness,
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

/// Failed startup gets its own bounded original-child settlement phase.
const CLEANUP_DEADLINE: Duration = Duration::from_secs(5);

#[cfg(test)]
pub(crate) mod cleanup_witness;
#[cfg(test)]
mod tests;
