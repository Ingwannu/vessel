//! Physical preparation ownership survives cancellation of its async waiter.
//! The same owner drains tracked blocking closures before acknowledging stop.

use std::sync::Arc;
use tokio::{sync::OwnedSemaphorePermit, task::JoinHandle};
use tokio_util::task::TaskTracker;

#[derive(Debug)]
struct Held {
    _global: OwnedSemaphorePermit,
    _provider: OwnedSemaphorePermit,
}

#[derive(Debug, Clone)]
pub(super) struct WorkLease {
    held: Arc<Held>,
    workers: TaskTracker,
}

impl WorkLease {
    pub(super) fn new(
        global: OwnedSemaphorePermit,
        provider: OwnedSemaphorePermit,
        workers: TaskTracker,
    ) -> Self {
        Self {
            held: Arc::new(Held {
                _global: global,
                _provider: provider,
            }),
            workers,
        }
    }

    /// The queued/running closure owns both permits and its tracker token.
    /// Dropping a JoinHandle cannot make actual filesystem work disappear.
    pub(super) fn blocking<T, F>(&self, operation: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let held = Arc::clone(&self.held);
        self.workers.spawn_blocking(move || {
            let _held = held;
            operation()
        })
    }
}
