//! One finite lane transfers accepted mutations to the node before lock waiting.
//! The consumer owns no sender, so abandoning all handles still drains the lane.

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tokio_util::task::task_tracker::TaskTrackerToken;
use vessel_proto::settings::{NodeSettings, SetNodeSettings};

use super::{SettingsError, State};

type Reply = oneshot::Receiver<Result<NodeSettings, SettingsError>>;

struct Operation {
    change: SetNodeSettings,
    reply: oneshot::Sender<Result<NodeSettings, SettingsError>>,
    // Registered before admission checks, released only after the reply attempt.
    _accounting: TaskTrackerToken,
}

/// The only settings operation quota; active work belongs to one consumer.
#[derive(Debug)]
pub(super) struct SettingsAdmission {
    sender: mpsc::Sender<Operation>,
    closing: CancellationToken,
    tasks: TaskTracker,
}

impl SettingsAdmission {
    /// Waiting capacity is additional to the single active operation.
    pub(super) const MAX_WAITING: usize = 8;

    pub(super) fn start(state: Arc<State>) -> Self {
        let (sender, receiver) = mpsc::channel(Self::MAX_WAITING);
        let tasks = TaskTracker::new();
        let closing = CancellationToken::new();
        // The tracked consumer owns physical work, not a request's future. Its
        // state has no reference back to this sender or the Settings handle.
        drop(tasks.spawn(consume(state, receiver, closing.clone())));
        Self {
            sender,
            closing,
            tasks,
        }
    }

    /// Non-waiting success is the ownership transfer and execution-order edge.
    pub(super) fn submit(&self, change: SetNodeSettings) -> Result<Reply, SettingsError> {
        let accounting = self.tasks.token();
        if self.tasks.is_closed() {
            return Err(SettingsError::Stopping);
        }
        let (reply, observing) = oneshot::channel();
        let operation = Operation {
            change,
            reply,
            _accounting: accounting,
        };
        self.sender
            .try_send(operation)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => SettingsError::Full,
                mpsc::error::TrySendError::Closed(_) => SettingsError::Stopping,
            })?;
        Ok(observing)
    }

    /// Token-before-close makes a racing accepted producer visible to drain.
    pub(super) async fn shutdown(&self) {
        let _first_close = self.tasks.close();
        self.closing.cancel();
        self.tasks.wait().await;
    }
}

async fn consume(
    state: Arc<State>,
    mut receiver: mpsc::Receiver<Operation>,
    closing: CancellationToken,
) {
    loop {
        tokio::select! {
            biased;
            () = closing.cancelled(), if !receiver.is_closed() => receiver.close(),
            next = receiver.recv() => {
                let Some(operation) = next else { break };
                // This await is deliberately outside cancellation selection: a
                // close seals the queue, never aborts accepted physical work.
                let result = state.change(operation.change).await;
                if let Err(error) = &result {
                    tracing::warn!(%error, "node settings operation failed");
                }
                if operation.reply.send(result).is_err() {
                    tracing::debug!("settings operation settled after its caller left");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
