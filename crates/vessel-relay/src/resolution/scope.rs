//! One admission/close order and retained joins, independent of result waiters.

use super::{
    Addresses, Failure, FailureKind, HostName, MAX_NATIVE_RESOLUTIONS, ResolutionPort,
    ResolutionQuota, worker,
};
use std::{
    fmt,
    sync::{Arc, Mutex, MutexGuard, Weak},
};
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, oneshot};

/// Stable native capacity owned once by the consumer lifecycle root.
#[derive(Debug, Clone)]
pub struct ResolutionBudget(Arc<Semaphore>);
impl Default for ResolutionBudget {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(MAX_NATIVE_RESOLUTIONS)))
    }
}
impl ResolutionBudget {
    /// Create a generation scope using an original permits-only consumer hold.
    /// Hold must not capture configuration, credentials, body/audio, a registry
    /// or ConnectionLife. Handles cannot mint another budget or scope.
    #[must_use]
    pub fn scope<Hold: Send + Sync + 'static>(
        &self,
        quota: ResolutionQuota,
        hold: Arc<Hold>,
    ) -> ResolutionScope<Hold> {
        ResolutionScope(Arc::new(Inner {
            budget: Arc::clone(&self.0),
            quota,
            hold,
            state: Mutex::new(State {
                closed: false,
                active: 0,
                failed: None,
            }),
            changed: Notify::new(),
        }))
    }
}

/// Observable physical completion, separate from a consumer's logical failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeState {
    /// May admit work, subject to both finite quotas.
    Open,
    /// Admission is closed but real worker completion is still outstanding.
    Draining,
    /// Closed, every join observed, and no physical work lease remains.
    Drained,
    /// Completion ownership failed; no normal drain receipt is available.
    Failed,
}

/// Opaque acknowledgement, obtainable only after actual normal drain.
#[derive(Debug)]
pub struct NormalDrain {
    _private: (),
}

/// Generation owner. Drop closes admission but never acknowledges normal stop.
pub struct ResolutionScope<Hold>(Arc<Inner<Hold>>);
impl<Hold: Send + Sync + 'static> ResolutionScope<Hold> {
    /// A weak resolver capability tied to this one scope.
    #[must_use]
    pub fn handle(&self) -> ResolutionHandle<Hold> {
        ResolutionHandle(Arc::downgrade(&self.0))
    }

    /// Close under the same order as admission and native-start decisions.
    pub fn close(&self) {
        self.0.close();
    }

    /// Inspect physical lifetime without waiting or performing any I/O.
    #[must_use]
    pub fn state(&self) -> ScopeState {
        self.0.state().snapshot()
    }

    /// Close, then wait for all admitted actual blocking joins. Cancelling this
    /// waiter does not cancel their ownership; calling drain again is safe.
    ///
    /// # Errors
    /// A failed completion owner cannot supply a normal-stop receipt.
    pub async fn drain(&self) -> Result<NormalDrain, Failure> {
        self.close();
        loop {
            let changed = self.0.changed.notified();
            tokio::pin!(changed);
            let _registered = changed.as_mut().enable();
            {
                let state = self.0.state();
                if state.active == 0 {
                    return match &state.failed {
                        Some(error) => Err(error.clone()),
                        None => Ok(NormalDrain { _private: () }),
                    };
                }
            }
            changed.await;
        }
    }
}
impl<Hold> Drop for ResolutionScope<Hold> {
    fn drop(&mut self) {
        self.0.close();
    }
}
impl<Hold> fmt::Debug for ResolutionScope<Hold> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ResolutionScope(<opaque>)")
    }
}

/// Cloneable weak capability; it cannot reopen or replace a closed generation.
pub struct ResolutionHandle<Hold>(Weak<Inner<Hold>>);
impl<Hold> Clone for ResolutionHandle<Hold> {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}
impl<Hold> fmt::Debug for ResolutionHandle<Hold> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ResolutionHandle(<opaque>)")
    }
}
impl<Hold: Send + Sync + 'static> ResolutionHandle<Hold> {
    /// Resolve using standard native work; full/closed refuses before copying
    /// worker input or spawning. Dropping this future never releases live work.
    ///
    /// # Errors
    /// Safe typed admission, input, resolver-result or worker failures.
    pub async fn resolve(
        &self,
        host: &HostName,
        port: ResolutionPort,
    ) -> Result<Addresses, Failure> {
        self.start(
            host,
            port,
            #[cfg(test)]
            None,
        )
        .await
    }

    async fn start(
        &self,
        host: &HostName,
        port: ResolutionPort,
        #[cfg(test)] probe: Option<tests::Probe>,
    ) -> Result<Addresses, Failure> {
        let inner = self
            .0
            .upgrade()
            .ok_or_else(|| Failure::new(FailureKind::Closed))?;
        // Runtime availability is established before admission. A shutdown
        // afterwards may prevent normal drain; it must never fabricate an ACK.
        let runtime = tokio::runtime::Handle::try_current().map_err(Failure::runtime)?;
        let lease = inner.admit()?;
        let host = host.clone();
        let running = Arc::clone(&inner);
        let blocking = runtime.spawn_blocking(move || {
            let _physical_lease = lease;
            #[cfg(test)]
            if let Some(probe) = &probe {
                probe.enter(tests::Stage::BeforeStart);
            }
            if running.state().closed {
                return Err(Failure::new(FailureKind::Closed));
            }
            #[cfg(test)]
            if let Some(probe) = &probe {
                probe.enter(tests::Stage::AfterStart);
            }
            worker::resolve(&host, port)
        });
        let (send, receive) = oneshot::channel();
        // This retained completion owns the REAL blocking join. Caller loss
        // only drops receive; normal close never aborts this completion task.
        drop(runtime.spawn(async move {
            let result = match blocking.await {
                Ok(result) => result,
                Err(error) => Err(Failure::worker(error)),
            };
            inner.completed(result.as_ref().err());
            let _gone = send.send(result);
        }));
        receive.await.map_err(Failure::receiver)?
    }
}

#[cfg(test)]
mod tests;

struct State {
    closed: bool,
    active: usize,
    failed: Option<Failure>,
}
impl State {
    fn snapshot(&self) -> ScopeState {
        if self.failed.is_some() {
            ScopeState::Failed
        } else if !self.closed {
            ScopeState::Open
        } else if self.active == 0 {
            ScopeState::Drained
        } else {
            ScopeState::Draining
        }
    }
}
struct Inner<Hold> {
    budget: Arc<Semaphore>,
    quota: ResolutionQuota,
    hold: Arc<Hold>,
    state: Mutex<State>,
    changed: Notify,
}
struct PhysicalLease<Hold> {
    _global: OwnedSemaphorePermit,
    _consumer: Arc<Hold>,
}
impl<Hold> Inner<Hold> {
    fn state(&self) -> MutexGuard<'_, State> {
        match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => {
                let mut state = poisoned.into_inner();
                state.closed = true;
                state.failed = Some(Failure::new(FailureKind::Worker));
                state
            }
        }
    }
    fn close(&self) {
        self.state().closed = true;
        self.changed.notify_waiters();
    }
    fn admit(&self) -> Result<PhysicalLease<Hold>, Failure> {
        let mut state = self.state();
        if state.closed {
            return Err(Failure::new(FailureKind::Closed));
        }
        if state.active >= self.quota.0.get() {
            return Err(Failure::new(FailureKind::Full));
        }
        let global = Arc::clone(&self.budget)
            .try_acquire_owned()
            .map_err(Failure::capacity)?;
        state.active = state
            .active
            .checked_add(1)
            .ok_or_else(|| Failure::new(FailureKind::Full))?;
        Ok(PhysicalLease {
            _global: global,
            _consumer: Arc::clone(&self.hold),
        })
    }
    fn completed(&self, failure: Option<&Failure>) {
        let mut state = self.state();
        match state.active.checked_sub(1) {
            Some(remaining) => state.active = remaining,
            None => {
                // An impossible duplicate completion is not a zero-work proof.
                state.closed = true;
                state.failed = Some(Failure::new(FailureKind::Worker));
            }
        }
        if let Some(error) = failure
            && error.kind() == FailureKind::Worker
        {
            state.closed = true;
            state.failed = Some(error.clone());
        }
        drop(state);
        self.changed.notify_waiters();
    }
}
