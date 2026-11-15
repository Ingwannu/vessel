//! Effective recovery settings, unresolved Store debt and irreversible stop seal.
//!
//! This producer owns one real policy state used by the canonical settings
//! operation. Engine-only watching and effect admission extend this same type
//! in the later consumer; no launch, prompt, task or fallback lives here.
use std::sync::{Mutex, MutexGuard, PoisonError};
use vessel_proto::restart::RestartRecovery;

#[derive(Debug)]
pub(crate) struct Policy {
    state: Mutex<State>,
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
        }
    }
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
    /// The same successfully saved value used by the real settings response.
    pub(crate) fn current(&self) -> RestartRecovery {
        self.lock().value.clone()
    }
    /// Shutdown shares the final effect mutex and is irreversible. A queued
    /// settings On may still settle on disk but cannot reopen this owner.
    pub(crate) fn seal_effect_admission(&self) {
        let mut state = self.lock();
        if state.admission == Admission::Sealed {
            return;
        }
        state.admission = Admission::Sealed;
    }
    pub(crate) fn begin_write(&self, next: &RestartRecovery) {
        let mut state = self.lock();
        let off_saving = !next.enabled;
        if state.off_saving == off_saving {
            return;
        }
        state.off_saving = off_saving;
    }
    pub(crate) fn write_failed(&self) {
        let mut state = self.lock();
        if !state.off_saving {
            return;
        }
        state.off_saving = false;
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
    }
    pub(crate) fn needs_cleanup(&self) -> bool {
        self.lock().pending == Pending::OffCleanup
    }
    pub(crate) fn cleaned(&self) {
        self.lock().pending = Pending::Clean;
    }
    pub(crate) fn needs_sync(&self) -> bool {
        self.lock().pending != Pending::Clean
    }
}

#[cfg(test)]
mod tests;
