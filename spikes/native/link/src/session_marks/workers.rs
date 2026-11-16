//! Test-only observation of the SAME actor hub; no client, State or guard escape.
use crate::Link;
use tokio::sync::OwnedSemaphorePermit;

/// A local fixture wait failed; no result certifies a node-side physical effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DrainProblem {
    /// The synchronous facade must not enter another Tokio runtime.
    #[error("mark worker observation cannot run inside a Tokio runtime")]
    NestedRuntime,
    /// No original actor hub is retained (including detached/expired fixtures).
    #[error("the original mark worker hub is unavailable")]
    Unavailable,
    /// Actual workers remain owned and unfinished; no success is fabricated.
    #[error("the original mark workers did not drain before the deadline")]
    TimedOut,
    /// The actual hub closed without granting its full permit set.
    #[error("the original mark worker hub closed")]
    Closed,
}
/// Hold through Scenario close: queued/new marks must fail admission while held.
/// This is local worker quiescence only, not a remote mutation receipt, physical
/// cleanup certificate, resident operation or permission to alter input/tabs.
#[derive(Debug)]
#[must_use = "retain the original permits until the fixture has closed"]
pub struct WorkerDrainGuard {
    _permit: OwnedSemaphorePermit,
}
impl Link {
    /// Stop test ingress, settle its original replies, then await this exact hub.
    /// Retain the returned guard through fixture close to reject later admission.
    /// Timeout cancels only the observation; actual workers and UI remain owned.
    ///
    /// # Errors
    /// Nested runtime, absent/expired original hub, deadline or semaphore closure.
    pub fn drain_session_marks(&self) -> Result<WorkerDrainGuard, DrainProblem> {
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(DrainProblem::NestedRuntime);
        }
        let hub = self
            .session_mark_workers
            .upgrade()
            .ok_or(DrainProblem::Unavailable)?;
        self._runtime.block_on(async {
            let permit = tokio::time::timeout(
                crate::testing::DEADLINE,
                hub.acquire_many_owned(u32::from(super::native::MAX_CALLS)),
            )
            .await
            .map_err(|_deadline| DrainProblem::TimedOut)?
            .map_err(|_closed| DrainProblem::Closed)?;
            Ok(WorkerDrainGuard { _permit: permit })
        })
    }
}
