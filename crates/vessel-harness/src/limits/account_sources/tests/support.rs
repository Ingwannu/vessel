//! Named phase deadlines sit inside the canonical, fake-preparing case deadline.
//!
//! [Decision Log]
//! - Purpose: an assertion or stalled handshake must fail without stranding a
//!   physical credential callback on the blocking pool.
//! - Constraint: aborting its async waiter is not settlement of that callback.
//! - Choice: one gate release owner handles success and unwinding; each phase
//!   is shorter than the canonical within case, whose clock starts after fakes.
//! - Alternative: a naked timeout can drop the only code that would release the
//!   worker. Sleeping, fake settlement or treating timeout as success hides it.
//! - Impact: production deadlines and semantic assertions stay unchanged. The
//!   failure control checks its exact deliberate panic payload, then joins the
//!   actual reader and drains its context; unrelated failures still fail.
use std::{
    future::Future,
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

pub(crate) const CASE_BUDGET: Duration = Duration::from_secs(20);
const PHASE_BUDGET: Duration = Duration::from_secs(10);

pub(crate) trait PhaseWait: Future + Sized {
    fn phase(self, name: &'static str) -> impl Future<Output = Self::Output> {
        // Boxing happens without polling. The timer starts only when the phase
        // is polled inside within, after the canonical fake preparation.
        let operation = Box::pin(self);
        async move {
            tokio::time::timeout(PHASE_BUDGET, operation)
                .await
                .unwrap_or_else(|cause| panic!("{name}: {cause}"))
        }
    }
}
impl<F: Future> PhaseWait for F {}

/// A callback may outlive its async waiter, including a failed test phase.
/// The same owner releases it on both the success path and unwinding.
pub(super) struct Release(pub(super) Arc<(Mutex<bool>, Condvar)>);
impl Release {
    pub(super) fn release(&self) {
        let (lock, changed) = &*self.0;
        *lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        changed.notify_all();
    }
}
impl Drop for Release {
    fn drop(&mut self) {
        self.release();
    }
}
