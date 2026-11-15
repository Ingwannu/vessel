//! One acquisition's two-second normalized-audio budget follows frames through every queue.
//!
//! Raw-device blocks, normalized frames and an upload borrowing those frames
//! retain the same permit. Moving PCM out of a queue cannot release its budget.

#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::sync::Arc;
use tokio::sync::OwnedSemaphorePermit;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use tokio::sync::{Semaphore, TryAcquireError};

/// At most two seconds of retained16k mono PCM16 per acquisition, across stages.
pub const CAPTURE_QUEUE_BYTES: usize = 64_000;
#[cfg(any(target_os = "linux", target_os = "windows"))]
const NORMALIZED_BLOCK_BYTES: u32 = 320;

#[cfg(any(target_os = "linux", target_os = "windows"))]
#[derive(Debug, Clone)]
pub(super) struct Budget(Arc<Semaphore>);
#[cfg(any(target_os = "linux", target_os = "windows"))]
impl Default for Budget {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(CAPTURE_QUEUE_BYTES)))
    }
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
impl Budget {
    #[cfg(test)]
    pub(super) fn available(&self) -> usize {
        self.0.available_permits()
    }
    pub(super) fn reserve(&self) -> Result<Permit, TryAcquireError> {
        Arc::clone(&self.0)
            .try_acquire_many_owned(NORMALIZED_BLOCK_BYTES)
            .map(|held| Permit { _held: held })
    }
}

#[derive(Debug)]
pub(super) struct Permit {
    _held: OwnedSemaphorePermit,
}

#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod tests {
    use super::*;
    #[test]
    fn a_full_acquisition_needs_actual_frame_drop_before_another_reservation() {
        let budget = Budget::default();
        let mut held = Vec::new();
        for _ in 0..200 {
            held.push(budget.reserve().unwrap());
        }
        assert!(budget.reserve().is_err());
        drop(held.pop());
        assert!(budget.reserve().is_ok());
    }
}
