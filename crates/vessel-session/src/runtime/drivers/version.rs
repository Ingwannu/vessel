//! The existing finite version operation, owned until actual root settlement.
//! A cleanup request is not descendant-death evidence. Root-wait uncertainty
//! retains the exact child and caller ownership rather than acknowledging drain.

use std::time::Duration;

use tokio::io::AsyncReadExt as _;
use tokio::task::JoinSet;
#[cfg(test)]
use vessel_proc::CommandSpec;
use vessel_proc::{CheckedWaitError, Child, OwnedGroupExit, Signal, Spawned};

const VERSION_TIMEOUT: Duration = Duration::from_secs(5);
const VERSION_BYTES: u64 = 4096;

#[cfg(test)]
pub(super) async fn read_version(command: CommandSpec) -> Result<Option<String>, CheckedWaitError> {
    let Ok(spawned) = vessel_proc::spawn(&command) else {
        return Ok(None);
    };
    observe_version(spawned, tokio::time::Instant::now(), drop).await
}

pub(super) async fn observe_version(
    spawned: Spawned,
    started: tokio::time::Instant,
    stdin_owner: impl FnOnce(tokio::process::ChildStdin) + Send,
) -> Result<Option<String>, CheckedWaitError> {
    let Spawned {
        mut child,
        stdin,
        stdout,
        mut stderr,
    } = spawned;
    stdin_owner(stdin);
    let mut readers = JoinSet::new();
    let _stderr = readers.spawn(async move {
        let _discarded = tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await;
    });
    let read = async {
        let mut text = String::new();
        let mut limited = stdout.take(VERSION_BYTES);
        let readable = limited.read_to_string(&mut text).await.is_ok();
        let exit = child.wait_checked().await?;
        Ok(if readable && exit.code() == Some(0) {
            text.lines()
                .next()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
        } else {
            None
        })
    };
    let observed =
        match tokio::time::timeout(VERSION_TIMEOUT.saturating_sub(started.elapsed()), read).await {
            Ok(Err(CheckedWaitError::Wait(error))) => unobserved(&mut child, error).await,
            Ok(observed) => observed,
            Err(_elapsed) => {
                let _kill_requested = child.signal(Signal::Kill);
                settled(&mut child).await.map(|_exit| None)
            }
        };
    readers.shutdown().await;
    observed
}

async fn settled(child: &mut Child) -> Result<OwnedGroupExit, CheckedWaitError> {
    match child.wait_checked().await {
        Ok(exit) => Ok(exit),
        Err(CheckedWaitError::Wait(error)) => unobserved(child, error).await,
        Err(error @ (CheckedWaitError::Sweep(_) | CheckedWaitError::Unrecorded)) => Err(error),
    }
}

async fn unobserved<T>(
    _child: &mut Child,
    error: vessel_proc::WaitError,
) -> Result<T, CheckedWaitError> {
    // Outside the version timeout: uncertainty must never become permission
    // to signal a delayed numeric id or to release the original physical owner.
    let _private_evidence = error;
    tracing::error!(
        class = "root_wait",
        "native version exit unobserved; preparation ownership retained"
    );
    std::future::pending().await
}

#[cfg(test)]
#[derive(Debug, Default)]
pub(in crate::runtime) struct VersionControl {
    armed: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<tokio::process::ChildStdin>>>,
    started: std::sync::atomic::AtomicBool,
}

#[cfg(test)]
impl VersionControl {
    pub(in crate::runtime) fn arm(
        &self,
    ) -> tokio::sync::oneshot::Receiver<tokio::process::ChildStdin> {
        let (send, receive) = tokio::sync::oneshot::channel();
        let previous = self
            .armed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .replace(send);
        assert!(
            previous.is_none(),
            "one physical version process control at a time"
        );
        receive
    }
    pub(in crate::runtime) fn has_started(&self) -> bool {
        self.started.load(std::sync::atomic::Ordering::Acquire)
    }
    pub(in crate::runtime) fn publish(&self, stdin: tokio::process::ChildStdin) {
        self.started
            .store(true, std::sync::atomic::Ordering::Release);
        let armed = self
            .armed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(send) = armed {
            let _gone = send.send(stdin);
        }
        // Otherwise stdin drops, preserving the old null-input behavior.
    }
}
