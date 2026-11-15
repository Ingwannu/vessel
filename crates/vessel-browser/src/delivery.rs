//! One atomic winner between caller cancellation and starting any page effect.
//! Reading a flag after a timeout is insufficient: a queued worker could start
//! just after that read. The caller instead withdraws the pending attempt before
//! reporting `NotDelivered`, and the worker must win the same state first.
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};
use vessel_proto::browser::BrowserEffect;

const PENDING: u8 = 0;
const STARTED: u8 = 1;
const CANCELLED: u8 = 2;

pub(crate) struct Attempt(AtomicU8);
impl Attempt {
    pub(crate) fn new() -> Self {
        Self(AtomicU8::new(PENDING))
    }
    pub(crate) fn start(&self) -> bool {
        self.0
            .compare_exchange(PENDING, STARTED, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
    fn cancel(&self) -> BrowserEffect {
        match self
            .0
            .compare_exchange(PENDING, CANCELLED, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) | Err(CANCELLED) => BrowserEffect::NotDelivered,
            Err(_) => BrowserEffect::Uncertain,
        }
    }
}
/// Also withdraws on a dropped/aborted run future, before its receiver disappears.
pub(crate) struct Caller(Arc<Attempt>);
impl Caller {
    pub(crate) fn new(attempt: Arc<Attempt>) -> Self {
        Self(attempt)
    }
    pub(crate) fn cancel(&self) -> BrowserEffect {
        self.0.cancel()
    }
}
impl Drop for Caller {
    fn drop(&mut self) {
        let _delivery = self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn withdrawn_attempt_cannot_start_but_started_attempt_stays_uncertain() {
        let pending = Arc::new(Attempt::new());
        let caller = Caller::new(Arc::clone(&pending));
        assert_eq!(caller.cancel(), BrowserEffect::NotDelivered);
        assert!(!pending.start());
        drop(caller);
        assert!(!pending.start());
        let started = Attempt::new();
        assert!(started.start());
        assert_eq!(started.cancel(), BrowserEffect::Uncertain);
        assert!(!started.start());
    }
}
