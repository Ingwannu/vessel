//! Internal typed persistence barriers shared by runtime and Store (ADR 0098).
//! These acquire no public event sequence or RPC registration. A commit reply
//! means the ordinary node writer durably applied the preceding records first.
use serde::{Deserialize, Serialize};

/// A stop/admission generation, bounded by the store's integer range.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct RestartGeneration(u64);
/// A generation that cannot be represented by the durable projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("restart generation exceeds the durable counter range")]
pub struct InvalidRestartGeneration;
impl RestartGeneration {
    /// First generation of a session.
    pub const ZERO: Self = Self(0);
    /// The durable counter value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
    /// The next generation, refusing exhaustion rather than wrapping.
    #[must_use]
    pub fn next(self) -> Option<Self> {
        self.0
            .checked_add(1)
            .and_then(|value| Self::try_from(value).ok())
    }
}
impl TryFrom<u64> for RestartGeneration {
    type Error = InvalidRestartGeneration;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if i64::try_from(value).is_ok() {
            Ok(Self(value))
        } else {
            Err(InvalidRestartGeneration)
        }
    }
}
impl From<RestartGeneration> for u64 {
    fn from(value: RestartGeneration) -> Self {
        value.0
    }
}

/// An internal ordered persistence checkpoint, never a public timeline event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartCheckpoint {
    /// A newly accepted prompt's actual `TurnStarted` record permits recovery.
    Arm {
        /// Generation captured at this input's admission.
        generation: RestartGeneration,
        /// The exact turn, not an observed process status.
        turn: crate::EventSeq,
    },
    /// A person's stop intent must commit before its effect/acknowledgement.
    Suppress {
        /// New stop generation.
        generation: RestartGeneration,
        /// Only this turn when named; a stale turn is a no-op.
        turn: Option<crate::EventSeq>,
    },
    /// Capture the actor before shutdown signals its provider.
    Shutdown {
        /// Current stop/admission generation.
        generation: RestartGeneration,
        /// Actual active turn; None for idle or still starting.
        turn: Option<crate::EventSeq>,
        /// A parked agent is never reopened by startup recovery.
        parked: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generations_use_the_durable_range_and_never_wrap() {
        let last = RestartGeneration::try_from(u64::try_from(i64::MAX).unwrap()).unwrap();
        assert!(last.next().is_none());
        assert!(RestartGeneration::try_from(u64::MAX).is_err());
        assert!(serde_json::from_str::<RestartGeneration>(&u64::MAX.to_string()).is_err());
        assert_eq!(RestartGeneration::ZERO.next().unwrap().get(), 1);
    }
}
