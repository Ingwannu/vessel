//! Validated user content remains redacted in diagnostics and is never truncated.

use crate::key::serde_via_text;
use std::{fmt, str::FromStr};

/// Bound the extra intent/fingerprint copy independently of the transport frame.
pub const MAX_GOAL_OBJECTIVE_BYTES: usize = 64 * 1024;

/// Invalid content is refused before any command reservation or provider call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GoalObjectiveError {
    /// An objective must contain meaningful text.
    #[error("goal objective is empty")]
    Empty,
    /// The objective exceeds its documented byte bound.
    #[error("goal objective exceeds {MAX_GOAL_OBJECTIVE_BYTES} bytes")]
    TooLong,
    /// Only text, tabs and line breaks may reach a provider field.
    #[error("goal objective contains unsupported control characters")]
    Control,
}

/// A trimmed, bounded UTF8 objective. Debug never prints submitted content.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct GoalObjective(String);
impl GoalObjective {
    /// Read only at the provider boundary or an explicitly authorized editor.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(super) fn trimmed(value: &str) -> Result<Self, GoalObjectiveError> {
        if value.is_empty() {
            return Err(GoalObjectiveError::Empty);
        }
        if value.len() > MAX_GOAL_OBJECTIVE_BYTES {
            return Err(GoalObjectiveError::TooLong);
        }
        if value
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\t' | '\n' | '\r'))
        {
            return Err(GoalObjectiveError::Control);
        }
        Ok(Self(value.to_owned()))
    }
}
impl FromStr for GoalObjective {
    type Err = GoalObjectiveError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::trimmed(super::parse::trim(value))
    }
}
impl fmt::Debug for GoalObjective {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GoalObjective")
            .field("bytes", &self.0.len())
            .finish_non_exhaustive()
    }
}
impl fmt::Display for GoalObjective {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
serde_via_text!(GoalObjective);
