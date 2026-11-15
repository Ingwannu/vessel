//! Finite retained jobs for launch metadata/probes and live application.
//!
//! The existing node task tracker drains these before closing the store.
//! Caller cancellation cannot release admission while a writer, probe or
//! provider setter still runs. A separate apply budget keeps a hung provider
//! from exhausting metadata reads. There is no queued work before admission.

use crate::error::ServiceError;
use std::{future::Future, sync::Arc};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, oneshot};
use tokio_util::{
    sync::CancellationToken,
    task::{TaskTracker, task_tracker::TaskTrackerToken},
};

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
// Admission must already be tracked before checking shutdown: otherwise a
// preempted request can spawn its job after teardown observed an empty tracker.
struct Admission {
    permit: OwnedSemaphorePermit,
    tracked: TaskTrackerToken,
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
        let Admission { permit, tracked } = self.admit(lane)?;
        let (send, receive) = oneshot::channel();
        drop(self.tasks.spawn(async move {
            let _tracked = tracked;
            let result = work().await;
            // The completion channel owns admission too, until consumed or
            // dropped. A departed caller still waits for actual work above.
            let _gone = send.send((result, permit));
        }));
        let (result, _permit) = receive.await.map_err(ServiceError::LaunchWorker)?;
        result
    }

    fn admit(&self, lane: Lane) -> Result<Admission, ServiceError> {
        let tracked = self.tasks.token();
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
        Ok(Admission { permit, tracked })
    }
}

#[cfg(test)]
mod tests;
