//! Preserve the first owned cleanup result without retrying a reused group id.

use std::sync::Arc;

use crate::{ProcessLifetime, SignalError, WaitError};

/// Receipt minted only by an owned [`crate::Child`] after its leader was reaped and its
/// original group/job cleanup request succeeded (including already absent).
///
/// This is not an OS-death certificate for every descendant or an escaped
/// process. [`crate::ProcessLifetime`] revocation, a durable PID snapshot and a root-only
/// pidfd signal cannot mint it. Callers must keep those evidence scopes distinct.
#[derive(Debug, Clone)]
pub struct OwnedGroupExit {
    code: Option<i32>,
    run: ProcessLifetime,
}

impl OwnedGroupExit {
    /// The owned leader's actual exit code, or None after a signal.
    #[must_use]
    pub const fn code(&self) -> Option<i32> {
        self.code
    }

    /// Original runner observation for exact-run association, never authority
    /// from numeric PID equality. The observation has already been revoked.
    #[must_use]
    pub const fn run(&self) -> &ProcessLifetime {
        &self.run
    }
}

impl PartialEq for OwnedGroupExit {
    fn eq(&self, other: &Self) -> bool {
        self.code == other.code && self.run.same_run(&other.run)
    }
}

impl Eq for OwnedGroupExit {}

/// A legacy leader wait success cannot flatten these cleanup uncertainties.
#[derive(Debug, thiserror::Error)]
pub enum CheckedWaitError {
    /// Original concrete failure to reap the owned leader.
    #[error("cannot wait for owned child")]
    Wait(#[source] WaitError),
    /// The very first cleanup sweep refused; subsequent checks return this same
    /// cause without signalling a numeric group id that could have been reused.
    #[error("owned child exited but its group cleanup request failed")]
    Sweep(#[source] Arc<SignalError>),
    /// No attempt was recorded; refuse proof rather than inventing success or
    /// performing a delayed signal. Normal owned transitions always record one.
    #[error("owned child cleanup outcome was not recorded")]
    Unrecorded,
}

#[derive(Debug, Default)]
pub(crate) struct FirstSweep(Option<Result<(), Arc<SignalError>>>);

impl FirstSweep {
    /// The closure runs at most once, while the caller still owns the original
    /// cleanup transition. Later reads never invoke another OS operation.
    pub(crate) fn record(
        &mut self,
        sweep: impl FnOnce() -> Result<(), SignalError>,
    ) -> Result<(), Arc<SignalError>> {
        self.0
            .get_or_insert_with(|| sweep().map_err(Arc::new))
            .clone()
    }

    pub(crate) fn receipt(
        &self,
        code: Option<i32>,
        run: ProcessLifetime,
    ) -> Result<OwnedGroupExit, CheckedWaitError> {
        match &self.0 {
            Some(Ok(())) => Ok(OwnedGroupExit { code, run }),
            Some(Err(cause)) => Err(CheckedWaitError::Sweep(Arc::clone(cause))),
            None => Err(CheckedWaitError::Unrecorded),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed_run() -> ProcessLifetime {
        let guard =
            crate::ProcessLifetimeGuard::begin(crate::ProcessId::from(std::num::NonZeroU32::MIN));
        guard.revoke();
        guard.observer()
    }

    #[test]
    fn failed_first_sweep_keeps_its_cause_and_never_runs_another_signal() {
        let mut observed = FirstSweep::default();
        let first = observed
            .record(|| {
                Err(SignalError::from(std::io::Error::from(
                    std::io::ErrorKind::PermissionDenied,
                )))
            })
            .expect_err("refused");
        let next = observed
            .record(|| panic!("a delayed signal must never run"))
            .expect_err("same refusal");
        assert!(Arc::ptr_eq(&first, &next));
        let CheckedWaitError::Sweep(receipt) = observed
            .receipt(Some(0), observed_run())
            .expect_err("cannot certify")
        else {
            panic!("sweep cause");
        };
        assert!(Arc::ptr_eq(&first, &receipt));
        assert!(receipt.is_refused());
    }

    #[test]
    fn successful_first_sweep_is_retained_and_missing_observation_never_mints_receipt() {
        let mut observed = FirstSweep::default();
        assert!(matches!(
            observed.receipt(Some(0), observed_run()),
            Err(CheckedWaitError::Unrecorded)
        ));
        observed.record(|| Ok(())).expect("accepted");
        observed
            .record(|| panic!("no delayed retry"))
            .expect("cached");
        assert_eq!(
            observed
                .receipt(Some(7), observed_run())
                .expect("receipt")
                .code(),
            Some(7)
        );
    }

    #[test]
    fn signal_error_clones_share_the_original_concrete_os_cause() {
        use std::error::Error;
        let signal = SignalError::from(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
        let clone = signal.clone();
        let original = signal
            .source()
            .expect("cause")
            .downcast_ref::<std::io::Error>()
            .expect("OS cause");
        let repeated = clone
            .source()
            .expect("cause")
            .downcast_ref::<std::io::Error>()
            .expect("OS cause");
        assert!(std::ptr::eq(original, repeated));
        assert!(clone.is_refused());
    }
}
