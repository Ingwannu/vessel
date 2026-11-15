//! A one-way cancellation fence wakes control independently of full audio queues.

use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Notify;

#[derive(Debug, Default)]
pub(super) struct Fence {
    closed: AtomicBool,
    changed: Notify,
}

impl Fence {
    pub(super) fn close(&self) {
        self.closed.store(true, Ordering::Release);
        self.changed.notify_waiters();
    }

    pub(super) fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    pub(super) async fn closed(&self) {
        // Register before checking the flag: Stop cannot fall between the
        // state check and waiter registration and strand a saturated actor.
        let notified = self.changed.notified();
        tokio::pin!(notified);
        let _registered = notified.as_mut().enable();
        if !self.is_closed() {
            notified.await;
        }
    }
}
