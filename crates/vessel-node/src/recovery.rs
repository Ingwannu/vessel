//! Boot ownership and ordinary-operation admission share one short arbiter.
#[cfg(test)]
mod gate;
pub(crate) mod policy;
use policy::Policy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, watch};
use tokio_util::sync::CancellationToken;
use vessel_proto::{ErrorCode, SessionId, WireError};

/// At most two boot attempts and 64 ordinary operations. These maps are
/// transient admission counters, never another source of session state.
#[derive(Debug)]
pub(crate) struct Recovery {
    pub(crate) policy: Arc<Policy>,
    /// Off/On must not rebase a queued or not-yet-polled boot task. Capture
    /// once before RPC serving; every attempt belongs to this same batch.
    pub(crate) boot_generation: policy::Generation,
    state: Mutex<State>,
    changed: watch::Sender<()>,
    operations: Arc<Semaphore>,
    #[cfg(test)]
    admission_gate: Mutex<Option<gate::Admission>>,
    #[cfg(test)]
    finished: watch::Sender<bool>,
}
#[derive(Debug, Default)]
struct State {
    attempts: HashMap<SessionId, Waiting>,
    operations: HashMap<SessionId, usize>,
}
#[derive(Debug, Clone)]
struct Waiting {
    cancelled: CancellationToken,
    done: watch::Receiver<bool>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Intervention {
    Wait,
    Stop,
}
#[derive(Debug)]
pub(crate) enum Claim {
    Owned(Attempt),
    Wait(watch::Receiver<()>),
}
#[derive(Debug)]
pub(crate) struct Attempt {
    owner: Arc<Recovery>,
    session: SessionId,
    pub(crate) cancelled: CancellationToken,
    done: watch::Sender<bool>,
}
#[derive(Debug)]
pub(crate) struct Operation {
    owner: Arc<Recovery>,
    session: SessionId,
    _slot: OwnedSemaphorePermit,
}
impl Recovery {
    pub(crate) fn new(policy: Arc<Policy>) -> Self {
        let (_, boot_generation) = policy.snapshot();
        Self {
            boot_generation,
            policy,
            state: Mutex::new(State::default()),
            changed: watch::channel(()).0,
            operations: Arc::new(Semaphore::new(64)),
            #[cfg(test)]
            admission_gate: Mutex::new(None),
            #[cfg(test)]
            finished: watch::channel(false).0,
        }
    }
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
    pub(crate) fn claim(self: &Arc<Self>, session: SessionId) -> Claim {
        let changed = self.changed.subscribe();
        let mut state = self.lock();
        if state.attempts.len() >= 2
            || state.attempts.contains_key(&session)
            || state.operations.contains_key(&session)
        {
            return Claim::Wait(changed);
        }
        let cancelled = CancellationToken::new();
        let (done, waiting) = watch::channel(false);
        let _old = state.attempts.insert(
            session,
            Waiting {
                cancelled: cancelled.clone(),
                done: waiting,
            },
        );
        Claim::Owned(Attempt {
            owner: Arc::clone(self),
            session,
            cancelled,
            done,
        })
    }
    /// Hold the returned lease through the actual ordinary operation, not
    /// just its initial check. This closes the case where a request passed
    /// a check before boot claimed its session, then resumed after an await.
    pub(crate) async fn before_operation(
        self: &Arc<Self>,
        session: SessionId,
        intervention: Intervention,
    ) -> Result<Arc<Operation>, WireError> {
        // A stop invalidates the attempt before any capacity refusal. No
        // new semaphore waiters are created by this admission component.
        if intervention == Intervention::Stop
            && let Some(waiting) = self.lock().attempts.get(&session)
        {
            waiting.cancelled.cancel();
        }
        let slot = Arc::clone(&self.operations)
            .try_acquire_owned()
            .map_err(|error| {
                let message = match error {
                    tokio::sync::TryAcquireError::NoPermits => {
                        "the node has 64 session operations in flight; retry after one finishes"
                    }
                    tokio::sync::TryAcquireError::Closed => "the node stopped operation admission",
                };
                WireError::new(ErrorCode::Unavailable, message)
            })?;
        loop {
            let waiting = {
                let mut state = self.lock();
                if let Some(waiting) = state.attempts.get(&session).cloned() {
                    if intervention == Intervention::Stop {
                        waiting.cancelled.cancel();
                    }
                    Some(waiting)
                } else {
                    // The fixed pool bounds this count; defend against
                    // wrap even though no more than 64 leases can exist.
                    let count = state.operations.entry(session).or_default();
                    *count = count.saturating_add(1);
                    None
                }
            };
            match waiting {
                None => {
                    return Ok(Arc::new(Operation {
                        owner: Arc::clone(self),
                        session,
                        _slot: slot,
                    }));
                }
                Some(mut waiting) => {
                    let _settled = waiting.done.wait_for(|done| *done).await;
                }
            }
        }
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        let mut state = self.owner.lock();
        if let Some(count) = state.operations.get_mut(&self.session) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                let _removed = state.operations.remove(&self.session);
            }
        }
        drop(state);
        self.owner.changed.send_replace(());
    }
}
impl Drop for Attempt {
    fn drop(&mut self) {
        let _removed = self.owner.lock().attempts.remove(&self.session);
        let _old = self.done.send_replace(true);
        self.owner.changed.send_replace(());
    }
}
impl Attempt {
    pub(crate) fn admit<T, F>(
        &self,
        generation: policy::Generation,
        effect: &mut Option<F>,
    ) -> Result<Option<T>, WireError>
    where
        F: FnOnce(&vessel_proto::restart::RestartRecovery) -> Result<T, WireError>,
    {
        #[cfg(test)]
        self.owner.before_admission();
        let state = self.owner.lock();
        if !state.attempts.contains_key(&self.session) || self.cancelled.is_cancelled() {
            return Err(WireError::new(
                ErrorCode::Conflict,
                "a person stopped this startup recovery attempt",
            ));
        }
        self.owner.policy.admit(generation, effect)
    }
}
