//! Finite retained jobs for launch metadata/probes and live application.
//!
//! The existing node task tracker drains these before closing the store.
//! Caller cancellation cannot release admission while a writer, probe or
//! provider setter still runs. A separate apply budget keeps a hung provider
//! from exhausting metadata reads. There is no queued work before admission.

use crate::error::ServiceError;
use std::{future::Future, sync::Arc};
use tokio::sync::{Semaphore, oneshot};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Lane {
    Metadata,
    Apply,
}

#[derive(Debug)]
pub(crate) struct LaunchProfileJobs {
    metadata: Arc<Semaphore>,
    apply: Arc<Semaphore>,
    tasks: TaskTracker,
    stopping: CancellationToken,
}
impl LaunchProfileJobs {
    /// One shared definition for both independent finite admission budgets.
    pub(crate) const MAX_ACTIVE: usize = 4;

    pub(crate) fn new(tasks: TaskTracker, stopping: CancellationToken) -> Self {
        Self {
            metadata: Arc::new(Semaphore::new(Self::MAX_ACTIVE)),
            apply: Arc::new(Semaphore::new(Self::MAX_ACTIVE)),
            tasks,
            stopping,
        }
    }

    pub(crate) async fn run<T, F, Work>(&self, lane: Lane, work: Work) -> Result<T, ServiceError>
    where
        T: Send + 'static,
        F: Future<Output = Result<T, ServiceError>> + Send + 'static,
        Work: FnOnce() -> F + Send + 'static,
    {
        if self.stopping.is_cancelled() {
            return Err(ServiceError::Draining);
        }
        let slots = match lane {
            Lane::Metadata => &self.metadata,
            Lane::Apply => &self.apply,
        };
        let permit = Arc::clone(slots)
            .try_acquire_owned()
            .map_err(ServiceError::LaunchBusy)?;
        let (send, receive) = oneshot::channel();
        drop(self.tasks.spawn(async move {
            let result = work().await;
            // The completion channel owns admission too, until consumed or
            // dropped. A departed caller still waits for actual work above.
            let _gone = send.send((result, permit));
        }));
        let (result, _permit) = receive.await.map_err(ServiceError::LaunchWorker)?;
        result
    }
}

#[cfg(test)]
mod tests;
