//! One registration gate makes close/drain linearizable with new attempts.

use std::sync::{Arc, Weak};

use tokio::sync::{Mutex, OwnedSemaphorePermit};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use super::{
    PreviewBudget, PreviewContext, PreviewError, PreviewOrigin, PreviewService,
    budget::{self, FRESH_FOR, PREVIEW_OPERATION_TIMEOUT},
    source::{Binding, Source},
};

/// Consumers can observe or revoke this authority, never construct it.
#[derive(Clone)]
pub(crate) struct PreviewLease(pub(super) Arc<Inner>);

pub(super) struct Inner {
    pub(super) context: PreviewContext,
    pub(super) binding: Binding,
    pub(super) source: Source,
    pub(super) cancelled: CancellationToken,
    pub(super) attempts: TaskTracker,
    pub(super) gate: Mutex<()>,
    _lease: OwnedSemaphorePermit,
}

/// Exact-lease drain association; it is not a process-exit or OS ownership proof.
#[must_use = "profile reuse requires the matching lease's completed drain"]
pub(crate) struct PreviewDrain(Weak<Inner>);

impl PreviewDrain {
    /// A receipt from another profile's lease cannot discharge this drain.
    pub(crate) fn belongs_to(&self, lease: &PreviewLease) -> bool {
        Weak::ptr_eq(&self.0, &Arc::downgrade(&lease.0))
    }
}

impl PreviewLease {
    pub(super) fn new(
        service: &PreviewService,
        context: PreviewContext,
        binding: Binding,
    ) -> Result<Self, PreviewError> {
        let permit = budget::take(&service.runtime.leases, PreviewBudget::Leases)?;
        let inner = Arc::new(Inner {
            context,
            binding,
            source: service.source.clone(),
            cancelled: service.runtime.shutdown.child_token(),
            attempts: TaskTracker::new(),
            gate: Mutex::new(()),
            _lease: permit,
        });
        let _task = service.runtime.tasks.spawn(observe(Arc::downgrade(&inner)));
        Ok(Self(inner))
    }

    /// Required before each browser command, independently of connection checks.
    pub(crate) async fn check(
        &self,
        context: &PreviewContext,
        origin: PreviewOrigin,
    ) -> Result<(), PreviewError> {
        tokio::time::timeout(PREVIEW_OPERATION_TIMEOUT, async {
            let _gate = self.0.gate.lock().await;
            self.0.check_context(context, origin)?;
            self.0.validate()
        })
        .await
        .map_err(PreviewError::Deadline)?
    }

    /// Browser targets and refs observe closure without retaining a context guard.
    pub(crate) async fn revoked(&self) {
        self.0.cancelled.cancelled().await;
    }

    /// Logical cancellation is immediate; receipt waits for actual resource owners.
    pub(crate) async fn revoke_and_drain(&self) -> PreviewDrain {
        self.0.close().await;
        self.0.attempts.wait().await;
        PreviewDrain(Arc::downgrade(&self.0))
    }
}

impl Inner {
    pub(super) fn check_context(
        &self,
        context: &PreviewContext,
        origin: PreviewOrigin,
    ) -> Result<(), PreviewError> {
        if context.scope != self.context.scope {
            return Err(PreviewError::Scope);
        }
        if context.owner != self.context.owner {
            return Err(PreviewError::Connection);
        }
        if origin != self.binding.origin {
            return Err(PreviewError::Origin);
        }
        if context.ended() {
            return Err(PreviewError::Revoked);
        }
        Ok(())
    }

    pub(super) fn validate(&self) -> Result<(), PreviewError> {
        let result = self.current();
        if result.is_err() {
            self.cancelled.cancel();
        }
        result.map(|_binding| ())
    }

    fn current(&self) -> Result<Binding, PreviewError> {
        if self.cancelled.is_cancelled() || self.context.ended() {
            return Err(PreviewError::Revoked);
        }
        let current = self.source.current()?;
        if !self.binding.same_binding(&current) {
            return Err(PreviewError::Changed);
        }
        current.validate()?;
        Ok(current)
    }

    async fn close(&self) {
        self.cancelled.cancel();
        // Registration uses this gate too; no new token can follow close.
        let _gate = self.gate.lock().await;
        let _first_close = self.attempts.close();
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.cancelled.cancel();
        let _first_close = self.attempts.close();
    }
}

async fn observe(weak: Weak<Inner>) {
    let Some(inner) = weak.upgrade() else { return };
    let mut source = inner.source.clone();
    let run = inner.binding.run.clone();
    let cancelled = inner.cancelled.clone();
    let closed = inner.context.closed.clone();
    let transport_closed = inner.context.transport_closed.clone();
    drop(inner);
    loop {
        let Some(inner) = weak.upgrade() else { return };
        let Ok(current) = inner.current() else {
            inner.close().await;
            return;
        };
        let Some(expiry) = current.observed.checked_add(FRESH_FOR) else {
            inner.close().await;
            return;
        };
        drop(inner);
        tokio::select! {
            biased;
            () = source.changed() => continue,
            () = cancelled.cancelled() => {},
            () = closed.cancelled() => {},
            () = transport_closed.cancelled() => {},
            () = run.ended() => {},
            () = tokio::time::sleep_until(expiry) => {},
        }
        if let Some(inner) = weak.upgrade() {
            inner.close().await;
        }
        return;
    }
}

impl std::fmt::Debug for PreviewLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreviewLease").finish_non_exhaustive()
    }
}

impl std::fmt::Debug for PreviewDrain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreviewDrain").finish_non_exhaustive()
    }
}
