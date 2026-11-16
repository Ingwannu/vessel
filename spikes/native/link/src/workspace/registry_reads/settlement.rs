//! Test-only acknowledgement of an actual registry read task returning.
//!
//! It proves neither current admission nor backing/process state. Keeping the
//! signalling owner outside the read future makes the normal acknowledgement
//! follow destruction of that future's selected connection and sender handles.

use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, sync_channel};
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
enum Completion {
    Returned,
    Abandoned,
}

/// A receiver minted only while the real read producer publishes its response.
/// Wait outside a Tokio worker, after consuming the owner's actual event.
#[derive(Debug)]
pub struct Settlement(Receiver<Completion>);

/// A task that did not return normally never satisfies normal-close proof.
#[derive(Debug, thiserror::Error)]
pub enum SettlementError {
    /// The bounded fixture wait expired, or the signalling owner disappeared.
    #[error("registry read settlement was not observed")]
    Receive(#[source] RecvTimeoutError),
    /// Cancellation or panic dropped the real task's signalling owner.
    #[error("registry read was abandoned before normal settlement")]
    Abandoned,
}

impl Settlement {
    /// Await actual task return within the caller's named fixture bound.
    /// This is a blocking fixture operation, never a UI/product wait or an
    /// authority credential. Queued response capacity is owned independently.
    pub fn wait(self, limit: Duration) -> Result<(), SettlementError> {
        match self
            .0
            .recv_timeout(limit)
            .map_err(SettlementError::Receive)?
        {
            Completion::Returned => Ok(()),
            Completion::Abandoned => Err(SettlementError::Abandoned),
        }
    }
}

/// At most one guarded response and one correlated fallback are published by a
/// read. Separate bounded one-message channels also acknowledge the rare case
/// where publication raced retirement: observing either is never task return.
#[derive(Default)]
pub(super) struct Pending {
    guarded: Option<SyncSender<Completion>>,
    refused: Option<SyncSender<Completion>>,
}

impl Pending {
    pub(super) fn guarded(&mut self) -> Settlement {
        let (sender, receiver) = sync_channel(1);
        self.guarded = Some(sender);
        Settlement(receiver)
    }

    pub(super) fn refused(&mut self) -> Settlement {
        let (sender, receiver) = sync_channel(1);
        self.refused = Some(sender);
        Settlement(receiver)
    }

    pub(super) fn complete(mut self) {
        self.signal(Completion::Returned);
    }

    fn signal(&mut self, completion: Completion) {
        for sender in [self.guarded.take(), self.refused.take()]
            .into_iter()
            .flatten()
        {
            // Each sender is signalled once and has one slot. A discarded
            // response may drop its receiver; it cannot block task shutdown.
            let _receiver_gone = sender.try_send(completion);
        }
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.signal(Completion::Abandoned);
    }
}
