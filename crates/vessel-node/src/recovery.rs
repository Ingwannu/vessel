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

/// Shared recovery test safety net, including canonical settings wire cases.
#[cfg(test)]
pub(crate) const CASE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(60);

/// Boot attempts and ordinary operations have separate fixed pools. These maps are
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
    /// Exact run whose prompt enqueue succeeded; its reply is still owned.
    prompt: Option<vessel_session::SessionHandle>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Intervention {
    Wait,
    Stop,
    /// A process kill can join an admitted prompt without awaiting its reply.
    Kill,
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
    kill_run: Option<vessel_session::SessionHandle>,
}
impl Recovery {
    /// Concurrent unresolved boot attempts; the coordinator and arbiter share it.
    pub(crate) const MAX_ATTEMPTS: usize = 2;
    /// Already admitted ordinary operations, including those waiting on boot.
    const MAX_OPERATIONS: usize = 64;
    pub(crate) fn new(policy: Arc<Policy>) -> Self {
        let (_, boot_generation) = policy.snapshot();
        Self {
            boot_generation,
            policy,
            state: Mutex::new(State::default()),
            changed: watch::channel(()).0,
            operations: Arc::new(Semaphore::new(Self::MAX_OPERATIONS)),
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
        if state.attempts.len() >= Self::MAX_ATTEMPTS
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
                prompt: None,
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
        if matches!(intervention, Intervention::Stop | Intervention::Kill)
            && let Some(waiting) = self.lock().attempts.get(&session)
        {
            waiting.cancelled.cancel();
        }
        let slot = Arc::clone(&self.operations)
            .try_acquire_owned()
            .map_err(|error| {
                let message = match error {
                    tokio::sync::TryAcquireError::NoPermits => {
                        format!("the node has {} session operations in flight; retry after one finishes", Self::MAX_OPERATIONS)
                    }
                    tokio::sync::TryAcquireError::Closed => "the node stopped operation admission".to_owned(),
                };
                WireError::new(ErrorCode::Unavailable, message)
            })?;
        loop {
            let admission = self.lock().before_operation(session, intervention);
            match admission {
                OperationGate::Admit(kill_run) => {
                    return Ok(Arc::new(Operation {
                        owner: Arc::clone(self),
                        session,
                        _slot: slot,
                        kill_run,
                    }));
                }
                OperationGate::Wait(mut waiting) => {
                    let _settled = waiting.done.wait_for(|done| *done).await;
                }
            }
        }
    }
}
enum OperationGate {
    Wait(Waiting),
    Admit(Option<vessel_session::SessionHandle>),
}
impl State {
    fn before_operation(
        &mut self,
        session: SessionId,
        intervention: Intervention,
    ) -> OperationGate {
        let waiting = self.attempts.get(&session).cloned();
        if matches!(intervention, Intervention::Stop | Intervention::Kill)
            && let Some(waiting) = &waiting
        {
            waiting.cancelled.cancel();
        }
        let kill_run = if intervention == Intervention::Kill {
            waiting.as_ref().and_then(|waiting| waiting.prompt.clone())
        } else {
            None
        };
        // Joining a person's kill retains the Attempt, its actual reply,
        // session claim and pool occupancy. Only the operation is admitted.
        if kill_run.is_none()
            && let Some(waiting) = waiting
        {
            return OperationGate::Wait(waiting);
        }
        // The fixed operation pool bounds this count; never wrap a counter.
        let count = self.operations.entry(session).or_default();
        *count = count.saturating_add(1);
        OperationGate::Admit(kill_run)
    }
}
impl Operation {
    /// A delayed admitted kill must never look up a replacement by session id.
    pub(crate) fn kill_target(&self) -> Option<vessel_session::SessionHandle> {
        self.kill_run.clone()
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
        self.admit_with(generation, None, effect)
    }
    /// Publish post-prompt ownership under the same arbiter as its enqueue.
    pub(crate) fn admit_prompt<T, F>(
        &self,
        generation: policy::Generation,
        handle: &vessel_session::SessionHandle,
        effect: &mut Option<F>,
    ) -> Result<Option<T>, WireError>
    where
        F: FnOnce(&vessel_proto::restart::RestartRecovery) -> Result<T, WireError>,
    {
        self.admit_with(generation, Some(handle), effect)
    }
    fn admit_with<T, F>(
        &self,
        generation: policy::Generation,
        prompt: Option<&vessel_session::SessionHandle>,
        effect: &mut Option<F>,
    ) -> Result<Option<T>, WireError>
    where
        F: FnOnce(&vessel_proto::restart::RestartRecovery) -> Result<T, WireError>,
    {
        #[cfg(test)]
        self.owner.before_admission();
        let mut state = self.owner.lock();
        if !state.attempts.contains_key(&self.session) || self.cancelled.is_cancelled() {
            return Err(WireError::new(
                ErrorCode::Conflict,
                "a person stopped this startup recovery attempt",
            ));
        }
        let admitted = self.owner.policy.admit(generation, effect)?;
        if admitted.is_some()
            && let Some(handle) = prompt
            && let Some(waiting) = state.attempts.get_mut(&self.session)
        {
            waiting.prompt = Some(handle.clone());
        }
        Ok(admitted)
    }
}
