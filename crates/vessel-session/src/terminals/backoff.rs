//! [`Backoff`]: when a failed service starts again, and when it is given
//! up. Pure, so the policy is tested without a clock.
//!
//! [Decision Log]
//! - Purpose: a dev server that dies on a syntax error should come back
//!   once the file is saved, but one that cannot start at all must not
//!   restart forever (neither agent-orchestrator's preview nor Paseo's
//!   services restart at all).
//! - Choice: 1, 2, 4, 8, then 16 s between starts; a run that lasted
//!   [`STABLE`] resets the count, so a service that fails once a day is
//!   never given up; the sixth failure in a row (five restarts) gives up.
//!   An exit with 0 is not a failure: the service is done, or wrongly
//!   written, and restarting it would hide that.

use std::time::Duration;

/// A run this long counts as having worked: the count starts again.
pub const STABLE: Duration = Duration::from_secs(60);

/// Restarts tried in a row before giving up.
pub const MAX_RESTARTS: u32 = 5;

/// The delay before the `n`th restart in a row (from 1).
const DELAYS: [Duration; 5] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(8),
    Duration::from_secs(16),
];

/// What to do after a service's process ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retry {
    /// Start it again after this long.
    After(Duration),
    /// It ended cleanly: do not start it again.
    Done,
    /// It failed too often in a row: give up after this many restarts.
    GiveUp {
        /// The restarts tried.
        restarts: u32,
    },
}

/// A service's failures in a row.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Backoff {
    failures: u32,
}

impl Backoff {
    /// A run ended after `ran` with `code` (`None`: a signal, or a start
    /// that failed).
    pub fn ended(&mut self, code: Option<i32>, ran: Duration) -> Retry {
        if code == Some(0) {
            self.failures = 0;
            return Retry::Done;
        }
        if ran >= STABLE {
            self.failures = 0;
        }
        self.failures = self.failures.saturating_add(1);
        let restarts = self.failures.saturating_sub(1);
        if restarts >= MAX_RESTARTS {
            return Retry::GiveUp { restarts };
        }
        let index = usize::try_from(restarts).unwrap_or(usize::MAX);
        Retry::After(
            DELAYS
                .get(index)
                .copied()
                .unwrap_or(Duration::from_secs(16)),
        )
    }

    /// A person restarted it: the count starts again.
    pub fn reset(&mut self) {
        self.failures = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUICK: Duration = Duration::from_secs(1);

    #[test]
    fn failures_back_off_then_give_up() {
        let mut backoff = Backoff::default();
        let seconds: Vec<Retry> = (0..6).map(|_| backoff.ended(Some(1), QUICK)).collect();
        assert_eq!(
            seconds,
            [
                Retry::After(Duration::from_secs(1)),
                Retry::After(Duration::from_secs(2)),
                Retry::After(Duration::from_secs(4)),
                Retry::After(Duration::from_secs(8)),
                Retry::After(Duration::from_secs(16)),
                Retry::GiveUp { restarts: 5 },
            ]
        );
    }

    #[test]
    fn a_stable_run_starts_the_count_again() {
        let mut backoff = Backoff::default();
        for _ in 0..4 {
            let _ = backoff.ended(None, QUICK);
        }
        assert_eq!(
            backoff.ended(Some(2), STABLE),
            Retry::After(Duration::from_secs(1))
        );
    }

    #[test]
    fn a_clean_exit_is_not_restarted() {
        let mut backoff = Backoff::default();
        let _ = backoff.ended(Some(1), QUICK);
        assert_eq!(backoff.ended(Some(0), QUICK), Retry::Done);
    }

    #[test]
    fn a_restart_by_a_person_starts_the_count_again() {
        let mut backoff = Backoff::default();
        for _ in 0..5 {
            let _ = backoff.ended(Some(1), QUICK);
        }
        backoff.reset();
        assert_eq!(
            backoff.ended(Some(1), QUICK),
            Retry::After(Duration::from_secs(1))
        );
    }
}
