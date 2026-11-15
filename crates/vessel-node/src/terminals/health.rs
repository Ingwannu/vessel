//! Service health transitions, staged until the owned route runtime is wired.
//! A clock grants no socket, process, workspace or browser authority.

use std::future::Future;
use std::time::Duration;

use tokio::time::Instant;

/// Grace begins at first eligible route observation, never at process spawn.
const GRACE: Duration = Duration::from_secs(5);
/// One failed probe must not immediately replace the previous health.
const FAILURES_BEFORE_UNHEALTHY: u8 = 2;

/// Health describes probe history, independently of supervised process status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Health {
    Pending,
    Healthy,
    Unhealthy,
}

/// The connector classifies success or failure without exposing a destination.
#[derive(Debug, Clone, Copy)]
pub(super) enum Probe {
    Connected,
    Failed,
}

/// One eligible run's observation history; replacement requires a new clock.
pub(super) struct HealthClock {
    observed_at: Instant,
    health: Health,
    failures: u8,
}

impl HealthClock {
    pub(super) fn new(observed_at: Instant) -> Self {
        Self {
            observed_at,
            health: Health::Pending,
            failures: 0,
        }
    }

    /// Initial publication can show pending before any permitted probe.
    pub(super) const fn state(&self) -> Health {
        self.health
    }

    /// The lazy callback must not open a connection during the grace period.
    /// Cancellation before its completion commits no history. The route owner
    /// must still recheck ownership before publishing a returned change.
    pub(super) async fn poll<F: Future<Output = Probe>>(
        &mut self,
        now: Instant,
        connect: impl FnOnce() -> F,
    ) -> Option<Health> {
        if now.saturating_duration_since(self.observed_at) < GRACE {
            return None;
        }
        let result = connect().await;
        let before = self.health;
        match result {
            Probe::Connected => {
                self.failures = 0;
                self.health = Health::Healthy;
            }
            Probe::Failed => {
                // Only the threshold matters; a long outage cannot overflow.
                self.failures = self
                    .failures
                    .saturating_add(1)
                    .min(FAILURES_BEFORE_UNHEALTHY);
                if self.failures == FAILURES_BEFORE_UNHEALTHY {
                    self.health = Health::Unhealthy;
                }
            }
        }
        (before != self.health).then_some(self.health)
    }
}

#[cfg(test)]
mod tests;
