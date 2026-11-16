//! Retain an already admitted observation's work, without inventing admission.
//!
//! Why (ADR0079): dropping a reply waiter cannot drop a permit while a blocking
//! syscall is still running. The existing tracker must own that real closure.
//! Its close flag is not a lifecycle/start fence; the future caller supplies it.
use super::{file, tree};
use std::{error::Error, fmt, path::PathBuf, time::Instant};
use tokio::{
    sync::OwnedSemaphorePermit,
    task::{JoinError, JoinHandle},
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

#[derive(Debug, thiserror::Error)]
enum Failure {
    #[error("plugin artifact observation requires the node runtime")]
    Runtime(#[source] tokio::runtime::TryCurrentError),
    #[error("plugin artifact observation refused")]
    Observation(#[source] tree::Failure),
    #[error("plugin artifact observation worker did not complete")]
    Worker(#[source] PrivateJoin),
}

/// Keep the concrete cancellation/panic cause without reopening panic text.
struct PrivateJoin(JoinError);
impl fmt::Debug for PrivateJoin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PrivateJoin")
            .field("cancelled", &self.0.is_cancelled())
            .field("panic", &self.0.is_panic())
            .finish()
    }
}
impl fmt::Display for PrivateJoin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("private plugin observation worker unavailable")
    }
}
impl Error for PrivateJoin {}

struct Observation {
    task: JoinHandle<Result<tree::Observed, tree::Failure>>,
    cancellation: CancellationToken,
}
impl Observation {
    fn start(
        path: PathBuf,
        permit: OwnedSemaphorePermit,
        tasks: &TaskTracker,
        parent: &CancellationToken,
    ) -> Result<Self, Failure> {
        let deadline =
            Instant::now()
                .checked_add(file::OBSERVATION_BUDGET)
                .ok_or(Failure::Observation(tree::Failure::File(
                    file::Failure::Expired,
                )))?;
        Self::start_with_gate(path, permit, tasks, parent, deadline, || {})
    }

    fn start_with_gate(
        path: PathBuf,
        permit: OwnedSemaphorePermit,
        tasks: &TaskTracker,
        parent: &CancellationToken,
        deadline: Instant,
        before: impl FnOnce() + Send + 'static,
    ) -> Result<Self, Failure> {
        let _runtime = tokio::runtime::Handle::try_current().map_err(Failure::Runtime)?;
        if parent.is_cancelled() {
            return Err(Failure::Observation(tree::Failure::File(
                file::Failure::Cancelled,
            )));
        }
        let cancellation = parent.child_token();
        let worker_cancel = cancellation.clone();
        let task = tasks.spawn_blocking(move || {
            // This guard is deliberately owned by the blocking closure, never
            // by the waiting future. Join completion follows its actual drop.
            let _permit = permit;
            before();
            tree::Pending::begin_until(&path, &worker_cancel, deadline)?.finish()
        });
        Ok(Self { task, cancellation })
    }

    async fn wait(mut self) -> Result<tree::Observed, Failure> {
        (&mut self.task)
            .await
            .map_err(|cause| Failure::Worker(PrivateJoin(cause)))?
            .map_err(Failure::Observation)
    }
}
impl Drop for Observation {
    fn drop(&mut self) {
        self.cancellation.cancel();
        // Dropping a JoinHandle detaches its waiter; the shared tracker and the
        // blocking closure still retain actual work and its admission permit.
    }
}

#[cfg(test)]
mod tests;
