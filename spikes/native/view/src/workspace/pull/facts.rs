//! A pull request's facts as Paseo's pull request pane reads them
//! (`git/pull-request-panel/pane.tsx`: the state under the title;
//! `checks-summary.ts`: the checks' headline and their counts in the order
//! a person acts on them; the activity summary's approval and change
//! request pills), from what the node sends (`vessel_proto::forge`).
//!
//! [Decision Log]
//! - Purpose: one reading of the facts for the Changes header's link, the
//!   pull request view and the board card, tested once.
//! - Constraints: the node sends counts (passed, failed, pending) and the
//!   failing checks only (at most twenty), not every check: Paseo's groups
//!   of successful and skipped checks cannot be listed by name.
//! - Alternatives: draw from the wire's enums in each screen.
//! - Choice: closed enums with Paseo's order; counts as numbers.
//! - Cost: a newer node's unknown values read as "not known", never as
//!   passing or mergeable.

use vessel_proto::forge::{
    Checks, ChecksState, Mergeable, PullState, PullSummary, ReviewDecision, Reviews,
};

/// Where it is in its life, as the state line under its title names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullPhase {
    /// Open and ready for review.
    Open,
    /// Open as a draft.
    Draft,
    /// Merged.
    Merged,
    /// Closed without merging.
    Closed,
    /// A state this app does not know.
    Unknown,
}

impl PullPhase {
    /// The phase of `summary`.
    #[must_use]
    pub const fn of(summary: &PullSummary) -> Self {
        match (summary.state, summary.draft) {
            (PullState::Open, true) => Self::Draft,
            (PullState::Open, false) => Self::Open,
            (PullState::Merged, _) => Self::Merged,
            (PullState::Closed, _) => Self::Closed,
            (PullState::Unknown, _) => Self::Unknown,
        }
    }
}

/// The worst thing happening in the checks: Paseo's `ChecksOutcome`, the
/// headline and the ring's colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksOutcome {
    /// One failed or was cancelled.
    Failure,
    /// One still runs and none failed.
    Pending,
    /// Every one passed.
    Success,
    /// None was reported.
    None,
    /// Not known (not read yet, or more than one read takes).
    Unknown,
}

impl ChecksOutcome {
    /// The outcome the node's summary names.
    #[must_use]
    pub const fn of(state: ChecksState) -> Self {
        match state {
            ChecksState::Failing | ChecksState::Cancelled => Self::Failure,
            ChecksState::Pending => Self::Pending,
            ChecksState::Passing => Self::Success,
            ChecksState::None => Self::None,
            ChecksState::Unknown => Self::Unknown,
        }
    }
}

/// One count of checks, in Paseo's order (what to act on, then what runs,
/// then what passed); only counts above zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckCount {
    /// Failing.
    Failing(u32),
    /// In progress.
    Pending(u32),
    /// Successful.
    Passed(u32),
}

/// The counts to show, in order.
#[must_use]
pub fn check_counts(checks: &Checks) -> Vec<CheckCount> {
    [
        CheckCount::Failing(checks.failed),
        CheckCount::Pending(checks.pending),
        CheckCount::Passed(checks.passed),
    ]
    .into_iter()
    .filter(|count| match count {
        CheckCount::Failing(n) | CheckCount::Pending(n) | CheckCount::Passed(n) => *n > 0,
    })
    .collect()
}

/// The review verdict as a person reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Approved.
    Approved,
    /// Changes were requested.
    ChangesRequested,
    /// A review is needed before it can merge.
    ReviewRequired,
    /// No verdict yet.
    None,
    /// Not known.
    Unknown,
}

impl Verdict {
    /// The forge's decision.
    #[must_use]
    pub const fn of(decision: ReviewDecision) -> Self {
        match decision {
            ReviewDecision::Approved => Self::Approved,
            ReviewDecision::ChangesRequested => Self::ChangesRequested,
            ReviewDecision::ReviewRequired => Self::ReviewRequired,
            ReviewDecision::None => Self::None,
            ReviewDecision::Unknown => Self::Unknown,
        }
    }
}

/// Paseo's activity pills: approvals and change requests that stand, and
/// unresolved threads (the node counts threads, not comments).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewPills {
    /// A person's approval stands.
    pub approved: bool,
    /// A person asked for changes.
    pub changes_requested: bool,
    /// Unresolved review threads.
    pub unresolved: u32,
}

impl ReviewPills {
    /// The pills of `reviews`.
    #[must_use]
    pub const fn of(reviews: &Reviews) -> Self {
        Self {
            approved: reviews.approved,
            changes_requested: reviews.changes_requested,
            unresolved: reviews.unresolved,
        }
    }
}

/// Whether it can merge now, as a person reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Merge {
    /// It can.
    Ready,
    /// It conflicts with its base.
    Conflicts,
    /// A rule blocks it (a review, a check, a draft).
    Blocked,
    /// It can, but a check is not green.
    Unstable,
    /// Its base moved on; it needs updating first.
    Behind,
    /// Not known yet (the forge computes it).
    Unknown,
}

impl Merge {
    /// The forge's mergeability.
    #[must_use]
    pub const fn of(mergeable: Mergeable) -> Self {
        match mergeable {
            Mergeable::Mergeable => Self::Ready,
            Mergeable::Conflicting => Self::Conflicts,
            Mergeable::Blocked => Self::Blocked,
            Mergeable::Unstable => Self::Unstable,
            Mergeable::Behind => Self::Behind,
            Mergeable::Unknown => Self::Unknown,
        }
    }
}

#[cfg(test)]
#[path = "facts_tests.rs"]
mod tests;
