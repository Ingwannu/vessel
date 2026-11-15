//! Admission policy shared by settings and the bounded startup owner.
//!
//! Settings invalidation and the final effect enqueue hold the same short
//! lock. Preparation may await without holding authority; it must return
//! through this gate before it can create a process or submit a prompt.
use std::sync::{Mutex, MutexGuard, PoisonError};
use tokio::sync::watch;
use vessel_proto::restart::RestartRecovery;
use vessel_proto::{ErrorCode, WireError};

#[derive(Debug)]
pub(crate) struct Policy {
    state: Mutex<State>,
    changed: watch::Sender<()>,
}
#[derive(Debug)]
struct State {
    value: RestartRecovery,
    off_saving: bool,
    pending: Pending,
    generation: Generation,
    admission: Admission,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Admission {
    Open,
    Sealed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pending {
    Clean,
    OffCleanup,
    OnApply,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Generation(u64);
impl Policy {
    pub(crate) fn new(value: RestartRecovery) -> Self {
        Self {
            state: Mutex::new(State {
                value,
                off_saving: false,
                pending: Pending::Clean,
                generation: Generation(0),
                admission: Admission::Open,
            }),
            changed: watch::channel(()).0,
        }
    }
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
    pub(crate) fn watch(&self) -> watch::Receiver<()> {
        self.changed.subscribe()
    }
    pub(crate) fn snapshot(&self) -> (RestartRecovery, Generation) {
        let state = self.lock();
        (state.value.clone(), state.generation)
    }
    /// Shutdown shares the final effect mutex and is irreversible. A queued
    /// settings On may still settle on disk but cannot reopen this owner.
    pub(crate) fn seal_effect_admission(&self) {
        self.lock().admission = Admission::Sealed;
        self.changed.send_replace(());
    }
    pub(crate) fn begin_write(&self, next: &RestartRecovery) {
        self.lock().off_saving = !next.enabled;
        self.changed.send_replace(());
    }
    pub(crate) fn write_failed(&self) {
        self.lock().off_saving = false;
        self.changed.send_replace(());
    }
    pub(crate) fn apply(&self, next: RestartRecovery) {
        let mut state = self.lock();
        if next.enabled {
            state.pending = Pending::OnApply;
        } else {
            // Saturation closes admission permanently instead of allowing
            // an old generation to match after wrapping.
            state.generation = Generation(state.generation.0.saturating_add(1));
            state.pending = Pending::OffCleanup;
        }
        state.value = next;
        state.off_saving = false;
        drop(state);
        self.changed.send_replace(());
    }
    pub(crate) fn needs_cleanup(&self) -> bool {
        self.lock().pending == Pending::OffCleanup
    }
    pub(crate) fn cleaned(&self) {
        self.lock().pending = Pending::Clean;
        self.changed.send_replace(());
    }
    pub(crate) fn needs_sync(&self) -> bool {
        self.lock().pending != Pending::Clean
    }
    pub(crate) fn check(&self, generation: Generation) -> Result<bool, WireError> {
        let state = self.lock();
        Self::check_state(&state, generation)
    }
    fn check_state(state: &State, generation: Generation) -> Result<bool, WireError> {
        if state.admission == Admission::Sealed {
            return Err(WireError::new(
                ErrorCode::Unavailable,
                "the node is stopping; startup recovery admission is closed",
            ));
        }
        if !state.value.enabled || state.generation.0 == u64::MAX || generation != state.generation
        {
            return Err(WireError::new(
                ErrorCode::Conflict,
                "startup recovery was disabled or superseded before admission",
            ));
        }
        Ok(!state.off_saving && state.pending == Pending::Clean)
    }
    pub(crate) fn admit<T, F>(
        &self,
        generation: Generation,
        effect: &mut Option<F>,
    ) -> Result<Option<T>, WireError>
    where
        F: FnOnce(&RestartRecovery) -> Result<T, WireError>,
    {
        let state = self.lock();
        if !Self::check_state(&state, generation)? {
            return Ok(None);
        }
        let effect = effect.take().ok_or_else(|| {
            WireError::new(
                ErrorCode::Internal,
                "startup recovery effect was already consumed",
            )
        })?;
        effect(&state.value).map(Some)
    }
}
