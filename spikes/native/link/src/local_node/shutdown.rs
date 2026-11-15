//! Runtime ownership outlives GPUI's non-vetoable 200ms shutdown observation.
//!
//! Ordinary quit/update already await protected finalization before `App::quit`.
//! The OS backstop cannot do that: dropping a timed-out callback must not drop
//! the runtime and implicitly kill an owned child's live sessions. A supervisor
//! created before launch receives the irreversible policy synchronously. Preserve
//! requests release, retaining the reactor/reaper until direct-child exit while
//! the launcher still lives. Actual process death remains the OS boundary; this
//! is neither a shutdown acknowledgement nor a document recovery journal.

use super::{ManagerError, Snapshot, actor};
use tokio::sync::{mpsc, oneshot, watch};
use vessel_app_view::local_node::Status;
use vessel_local_node::QuitPolicy;

pub(super) const CAPACITY: usize = 1;

#[derive(Debug)]
pub(super) enum Policy {
    Ordinary,
    Preserve,
}

pub(super) fn run(
    runtime: tokio::runtime::Runtime,
    policy: std::sync::mpsc::Receiver<Policy>,
    commands: mpsc::Sender<actor::Command>,
    state: watch::Receiver<Snapshot>,
) {
    if matches!(policy.recv(), Ok(Policy::Preserve)) {
        runtime.block_on(preserve(commands, state));
    }
    runtime.shutdown_background();
}

async fn preserve(commands: mpsc::Sender<actor::Command>, mut state: watch::Receiver<Snapshot>) {
    let (reply, wait) = oneshot::channel::<Result<(), ManagerError>>();
    let request = actor::Command::Finalize {
        policy: QuitPolicy::KeepRunning,
        confirmation: None,
        reply,
    };
    if commands.send(request).await.is_ok() {
        match wait.await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!(%error, "OS exit release was not acknowledged"),
            Err(_closed) => {
                tracing::warn!("OS exit controller stopped before release acknowledgement")
            }
        }
    }
    loop {
        let status = state.borrow_and_update().status;
        if !matches!(status, Status::Owned(_) | Status::Kept(_) | Status::Working) {
            return;
        }
        if state.changed().await.is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests;
