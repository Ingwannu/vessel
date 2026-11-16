//! Preserve the last confirmed value while a read or mutation is in flight.
use super::{Problem, Task};

/// All loading and failure states are explicit and retain their attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Load<T> {
    /// No request in this visit.
    Empty,
    /// In flight; prior values are informational, never acknowledgement.
    Pending {
        /// Last confirmed data, not optimistic state.
        previous: Option<T>,
        /// Exact outstanding attempt.
        task: Task,
    },
    /// Last confirmed node result.
    Ready(T),
    /// Last confirmed value plus safe recovery cause.
    Failed {
        /// Last confirmed data, if a read already succeeded.
        previous: Option<T>,
        /// Safe recovery category.
        problem: Problem,
    },
}
impl<T> Load<T> {
    /// The last confirmed value, if any.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Empty => None,
            Self::Ready(value) => Some(value),
            Self::Pending { previous, .. } | Self::Failed { previous, .. } => previous.as_ref(),
        }
    }
    /// Whether controls must wait for acknowledgement.
    #[must_use]
    pub const fn pending(&self) -> bool {
        matches!(self, Self::Pending { .. })
    }
    /// An unknown mutation outcome requires a fresh read before another write.
    #[must_use]
    pub const fn requires_read(&self) -> bool {
        matches!(
            self.problem(),
            Some(Problem::TimedOut | Problem::Uncertain | Problem::InvalidReply)
        )
    }
    /// Controls may not replay a mutation based on an unconfirmed prior value.
    #[must_use]
    pub const fn writes_blocked(&self) -> bool {
        self.pending() || self.requires_read()
    }
    /// Safe recovery cause.
    #[must_use]
    pub const fn problem(&self) -> Option<Problem> {
        match self {
            Self::Failed { problem, .. } => Some(*problem),
            Self::Empty | Self::Pending { .. } | Self::Ready(_) => None,
        }
    }
    pub(super) fn accepts(&self, task: &Task) -> bool {
        matches!(self, Self::Pending { task: expected, .. } if task == expected)
    }
}
impl<T: Clone> Load<T> {
    pub(super) fn begin(&mut self, task: Task) {
        *self = Self::Pending {
            previous: self.value().cloned(),
            task,
        };
    }
    pub(super) fn finish(&mut self, result: Result<T, Problem>) {
        *self = match result {
            Ok(value) => Self::Ready(value),
            Err(problem) => Self::Failed {
                previous: self.value().cloned(),
                problem,
            },
        };
    }
}
