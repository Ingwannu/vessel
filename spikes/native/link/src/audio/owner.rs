//! One process-wide reservation survives Stop until the device thread destroys its streams.
//!
//! [Decision Log]
//! - Purpose: all windows and dictation/voice share one physical microphone.
//! - Constraint: CPAL streams live on their owning thread; a queued Stop proves no release.
//! - Choice: a synchronous reservation with an independent atomic callback fence.
//!   The private guard stays on that thread through stream destruction. Poisoning
//!   refuses new acquisitions instead of guessing whether ownership is safe.

use std::sync::{Arc, Mutex, OnceLock};

use tokio::sync::watch;
use vessel_app_view::audio::{CaptureCompletion, CaptureLease, CapturePurpose, CaptureSlot};

use super::AudioError;
use super::budget::Budget;
use super::upload_access::Gate;
use crate::cancel::Fence;

static ENGINE: OnceLock<Engine> = OnceLock::new();

#[derive(Debug, Clone, Default)]
pub(super) struct Engine {
    slot: Arc<Mutex<CaptureSlot>>,
}

use super::capture::AcquireError;
pub(super) use super::capture::{CaptureState as State, DeviceFailure};

/// Only the owning thread retires this guard after dropping its streams.
#[derive(Debug)]
pub(super) struct Owner {
    engine: Engine,
    lease: CaptureLease,
    fence: Arc<Fence>,
    status: watch::Sender<State>,
    failure: Option<DeviceFailure>,
    budget: Budget,
    upload: Gate,
}

/// This handle gates callbacks; it cannot release physical ownership.
#[derive(Debug)]
pub(super) struct Handle {
    lease: CaptureLease,
    fence: Arc<Fence>,
    status: watch::Receiver<State>,
}

impl Engine {
    pub(super) fn process() -> &'static Self {
        ENGINE.get_or_init(Self::default)
    }

    pub(super) fn reserve(&self, purpose: CapturePurpose) -> Result<(Owner, Handle), AcquireError> {
        let lease = self
            .slot
            .lock()
            .map_err(|_private| AcquireError::Ownership)?
            .reserve(purpose)?;
        let fence = Arc::new(Fence::default());
        let (status, receiver) = watch::channel(State::Reserved);
        Ok((
            Owner {
                engine: self.clone(),
                lease,
                fence: Arc::clone(&fence),
                status,
                failure: None,
                budget: Budget::default(),
                upload: Gate::new(Arc::clone(&fence)),
            },
            Handle {
                lease,
                fence,
                status: receiver,
            },
        ))
    }
}

impl Owner {
    pub(super) fn upload_gate(&self) -> Gate {
        self.upload.clone()
    }
    pub(super) fn budget(&self) -> Budget {
        self.budget.clone()
    }
    pub(super) const fn lease(&self) -> CaptureLease {
        self.lease
    }

    pub(super) fn fence(&self) -> Arc<Fence> {
        Arc::clone(&self.fence)
    }

    pub(super) fn stopped(&self) -> bool {
        self.fence.is_closed()
    }

    pub(super) fn started(&self) -> Result<(), AudioError> {
        if self.stopped() {
            return Err(AudioError::Stopped);
        }
        let completion = self
            .engine
            .slot
            .lock()
            .map_err(|_private| AudioError::Ownership)?
            .started(self.lease)
            .map_err(AudioError::Reservation)?;
        if completion != CaptureCompletion::Current {
            return Err(AudioError::Stopped);
        }
        let _previous = self.status.send_replace(State::Running);
        Ok(())
    }

    pub(super) fn fail(&mut self, failure: DeviceFailure) {
        self.failure = Some(failure);
        self.fence.close();
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        self.fence.close();
        // Keep a poisoned slot held rather than publish unverified release.
        if let Ok(mut slot) = self.engine.slot.lock()
            && slot.stopping(self.lease).is_ok()
            && slot.released(self.lease).is_ok()
        {
            let state = self.failure.map_or(State::Released, State::Failed);
            let _previous = self.status.send_replace(state);
        }
    }
}

impl Handle {
    pub(super) const fn lease(&self) -> CaptureLease {
        self.lease
    }

    pub(super) fn stop(&self) {
        self.fence.close();
    }

    pub(super) fn status(&self) -> watch::Receiver<State> {
        self.status.clone()
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests;
