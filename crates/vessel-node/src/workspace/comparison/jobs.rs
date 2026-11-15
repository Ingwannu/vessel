//! A comparison outlives its reply receiver so its index cleanup cannot be
//! abandoned. Shutdown cancels subprocesses, not this owned cleanup future.

use std::future::Future;
use std::sync::Arc;

use tokio::sync::{Semaphore, oneshot};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

/// Includes waiting and executing operations, independent of the process pool.
const MAX_ADMITTED: usize = 64;
/// Paseo's snapshot concurrency, not its separate observation setup limit.
const MAX_EXECUTING: usize = 4;

/// Refusals before a result can be delivered; none represents an empty comparison.
#[derive(Debug, thiserror::Error)]
pub(crate) enum JobError {
    /// All comparison reservations are occupied.
    #[error("too many comparison reads are active; try again")]
    Busy,
    /// The node is stopping; no queued work is started.
    #[error("the node is stopping")]
    Closed,
    /// An owned worker ended without settling its reply.
    #[error("the comparison worker stopped")]
    Worker(#[from] oneshot::error::RecvError),
}

/// One node's bounded admission and execution, with no per-connection owner.
#[derive(Debug)]
pub(in crate::workspace) struct Jobs {
    admitted: Arc<Semaphore>,
    executing: Arc<Semaphore>,
    #[cfg(test)]
    submitted: tokio::sync::Notify,
}

impl Default for Jobs {
    fn default() -> Self {
        Self {
            admitted: Arc::new(Semaphore::new(MAX_ADMITTED)),
            executing: Arc::new(Semaphore::new(MAX_EXECUTING)),
            #[cfg(test)]
            submitted: tokio::sync::Notify::new(),
        }
    }
}

/// Dropping this receiver abandons only delivery, never operation ownership.
#[derive(Debug)]
pub(super) struct Pending<T>(oneshot::Receiver<Result<T, JobError>>);

impl<T> Pending<T> {
    pub(super) async fn result(self) -> Result<T, JobError> {
        self.0.await?
    }
}

impl Jobs {
    /// Reserve before spawning: a disconnected caller cannot create an
    /// unbounded queue. Once execution starts, only the operation's bound git
    /// cancellation token stops commands; the future is awaited through cleanup.
    pub(super) fn submit<T, F>(
        &self,
        tasks: &TaskTracker,
        stop: CancellationToken,
        work: F,
    ) -> Result<Pending<T>, JobError>
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        if stop.is_cancelled() {
            return Err(JobError::Closed);
        }
        let admitted = Arc::clone(&self.admitted)
            .try_acquire_owned()
            .map_err(|_full| JobError::Busy)?;
        #[cfg(test)]
        self.submitted.notify_waiters();
        let executing = Arc::clone(&self.executing);
        let (reply, answer) = oneshot::channel();
        let _task = tasks.spawn(async move {
            let _admitted = admitted;
            let result = tokio::select! {
                biased;
                () = stop.cancelled() => Err(JobError::Closed),
                slot = executing.acquire_owned() => match slot {
                    Ok(_executing) => Ok(work.await),
                    Err(_closed) => Err(JobError::Closed),
                },
            };
            let _delivered = reply.send(result);
        });
        Ok(Pending(answer))
    }
}

#[cfg(test)]
impl Jobs {
    pub(super) fn hold_execution_for_test(&self) -> tokio::sync::OwnedSemaphorePermit {
        Arc::clone(&self.executing)
            .try_acquire_many_owned(u32::try_from(MAX_EXECUTING).unwrap())
            .unwrap()
    }
    pub(super) fn submitted_for_test(&self) -> &tokio::sync::Notify {
        &self.submitted
    }
}

#[cfg(test)]
mod tests;
