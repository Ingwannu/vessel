//! RAII retains the exact group/job handle through readiness and every error path.

use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, BufReader};
use vessel_proc::{Child, Detached, Spawned, spawn_releasable};
use vessel_proto::local_launch::{MAX_READY_BYTES, Ready};

use super::{Launch, LaunchError};
use crate::logs::LogReader;

pub(crate) struct Owned {
    child: Child,
    logs: LogReader,
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
    ) -> Result<(Self, Ready), LaunchError> {
        let Spawned {
            child,
            stdin,
            stdout,
            stderr,
        } = spawn_releasable(&launch.command).map_err(LaunchError::Spawn)?;
        drop(stdin);
        let owned = Self {
            child,
            logs: LogReader::collect(stderr),
        };
        on_logs(owned.logs());
        // Probe one extra byte, so oversized input cannot look like a valid
        // truncated record. Reader allocation is capped before line decoding.
        let limit = u64::try_from(MAX_READY_BYTES.saturating_add(1))
            .map_err(|_range| LaunchError::ReadLimit)?;
        let mut reader = BufReader::new(stdout.take(limit));
        let mut line = String::new();
        let _read = reader
            .read_line(&mut line)
            .await
            .map_err(LaunchError::Read)?;
        let ready = Ready::decode(&line).map_err(LaunchError::Ready)?;
        Ok((owned, ready))
    }

    pub(crate) fn logs(&self) -> LogReader {
        self.logs.clone()
    }

    pub(crate) async fn wait(&mut self) -> Result<ProcessExit, LaunchError> {
        self.child
            .wait()
            .await
            .map(ProcessExit)
            .map_err(LaunchError::Wait)
    }

    pub(crate) fn keep(self) -> Result<KeptNode, LaunchError> {
        let detached = self.child.detach().map_err(LaunchError::Detach)?;
        let mut reaped = detached.clone();
        let logs = self.logs;
        // Keep draining stderr until direct-child exit even after all UI handles
        // disappear. This task ends with the original launcher runtime.
        drop(tokio::spawn(async move {
            let _observed = reaped.wait().await;
            drop(logs);
        }));
        Ok(KeptNode { detached })
    }
}

/// A finite child startup includes pipe readiness and the authenticated greeting.
pub(crate) const START_DEADLINE: Duration = Duration::from_secs(30);
/// Stop acknowledgement does not substitute for this real-exit wait.
pub(crate) const STOP_DEADLINE: Duration = Duration::from_secs(15);
