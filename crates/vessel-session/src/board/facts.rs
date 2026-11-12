//! The facts a board placement is derived from: the session's, and each
//! pull request's, typed as agent-orchestrator's `contract` package has
//! them (`KanbanSessionFacts`, `KanbanPRFacts` and what they hold).
//!
//! Plain data with public fields: whoever knows a fact sets it (the
//! session actor its worker's activity; the node, later, its pull
//! requests). Not wire types.

use chrono::{DateTime, Utc};
use vessel_proto::Activity;

/// What a session's worker (its agent or terminal program) is doing: the
/// reference's `SessionFacts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerFacts {
    /// What it was last seen doing.
    pub activity: Activity,
    /// When it was last seen doing something.
    pub last_activity_at: DateTime<Utc>,
    /// Whether it has reported anything since it started.
    pub has_signal: bool,
    /// Whether it is expected to report. In agent-orchestrator a worker
    /// whose hooks should fire and never do is a broken setup worth a
    /// phrase. False for every Vessel session: a structured session that
    /// never reports fails at its handshake timeout, and a terminal
    /// program owes no report.
    pub signal_expected: bool,
    /// Whether the session is archived (ADR 0026; until protocol 10,
    /// whether it had ended). agent-orchestrator's `terminated`, which its
    /// archive action sets: a session whose process is gone and that
    /// nobody archived has `activity` exited and stays in its lane.
    pub is_terminated: bool,
}

/// Which follow-up loops the fleet drives on a session's behalf: whether
/// its review-feedback loop is turned by the fleet or by a person. No
/// Vessel session has any yet; all are false.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Policy {
    /// The fleet reviews each new head itself and holds the pull request
    /// until its own pass approves.
    pub auto_review: bool,
    /// Review feedback is handed to the agent to address.
    pub auto_inject_review: bool,
    /// Failing checks are handed to the agent to fix.
    pub auto_inject_ci: bool,
}

/// Everything about the session itself the reducer reads: the reference's
/// `KanbanSessionFacts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionFacts {
    /// What its worker is doing.
    pub worker: WorkerFacts,
    /// Its latest turn was stopped before it finished, and none has started
    /// since. Speaks only while the worker is idle.
    pub interrupted: bool,
    /// It delivered something other than a pull request (a document, a
    /// report) for a person to look at.
    pub has_artifact_output: bool,
    /// The loops the fleet drives for it.
    pub policy: Policy,
}

/// The aggregate state of a pull request's checks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum CiState {
    /// Not known (no checks, or not read yet).
    #[default]
    Unknown,
    /// Some checks are still running.
    Pending,
    /// Every check passed.
    Passing,
    /// A check failed.
    Failing,
}

/// The provider's aggregate review verdict on a pull request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ReviewDecision {
    /// No verdict.
    #[default]
    None,
    /// Approved.
    Approved,
    /// Changes were requested.
    ChangesRequested,
    /// A review is required before it can merge.
    ReviewRequired,
}

/// Whether a pull request can be merged now.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Mergeability {
    /// Not known.
    #[default]
    Unknown,
    /// It can be merged.
    Mergeable,
    /// It conflicts with its base.
    Conflicting,
    /// A rule blocks it (a required review or check).
    Blocked,
    /// It can be merged, but a check is not green.
    Unstable,
}

/// The fleet's own review passes against a pull request's current head.
/// Passes recorded for an earlier head are left out before this is built,
/// so a stale pass can never decide the lane.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag aggregates every pass on the head (any running, any \
              asking for changes, ...); they are independent, as the \
              reference has them, and no enum of fewer states holds them"
)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReviewRunFacts {
    /// At least one pass was recorded for the current head.
    pub present: bool,
    /// A pass for the current head is still running.
    pub running: bool,
    /// A pass for the current head asked for changes.
    pub changes_requested: bool,
    /// A pass for the current head returned a verdict. Present without an
    /// outcome is a head the fleet tried and failed to review, which still
    /// owes the pull request its pass.
    pub outcome: bool,
    /// A pass for the current head ended without a verdict.
    pub failed: bool,
    /// A pass for the current head was cancelled.
    pub cancelled: bool,
}

/// Review verdicts on a pull request that the fleet did not author. Its own
/// reviews are left out, because the aggregate [`ReviewDecision`] mixes
/// both and cannot say whose turn the review loop is on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExternalReviewFacts {
    /// A person's approval stands.
    pub approved: bool,
    /// A person requested changes.
    pub changes_requested: bool,
    /// A person commented without requesting changes.
    pub comments: bool,
}

/// A pull request's address, which identifies it and breaks the last tie
/// between equally ranked pull requests.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PullRequestUrl(String);

impl PullRequestUrl {
    /// The pull request at `url`.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self(url.into())
    }

    /// The address as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One pull request of a session: the reference's `KanbanPRFacts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestFacts {
    /// Its address.
    pub url: PullRequestUrl,
    /// It is a draft.
    pub draft: bool,
    /// It was merged.
    pub merged: bool,
    /// It was closed (merged or not; the reference keeps both flags).
    pub closed: bool,
    /// Its checks.
    pub ci: CiState,
    /// The provider's aggregate review verdict.
    pub review: ReviewDecision,
    /// Whether it can be merged.
    pub mergeability: Mergeability,
    /// When it last changed.
    pub updated_at: DateTime<Utc>,
    /// The fleet's own review passes on its current head.
    pub review_run: ReviewRunFacts,
    /// Reviews by people.
    pub external_review: ExternalReviewFacts,
}

impl PullRequestFacts {
    /// An open, ready pull request with nothing known about it yet.
    #[must_use]
    pub fn new(url: PullRequestUrl, updated_at: DateTime<Utc>) -> Self {
        Self {
            url,
            draft: false,
            merged: false,
            closed: false,
            ci: CiState::default(),
            review: ReviewDecision::default(),
            mergeability: Mergeability::default(),
            updated_at,
            review_run: ReviewRunFacts::default(),
            external_review: ExternalReviewFacts::default(),
        }
    }

    /// Still moving through a loop: neither merged nor closed.
    pub(super) const fn is_live(&self) -> bool {
        !self.merged && !self.closed
    }
}

/// What the node knows about a session's pull requests.
#[derive(Debug, Clone, Copy)]
pub enum PullRequests<'a> {
    /// It cannot see them (no Vessel node can yet). Unlike an empty
    /// [`Seen`](Self::Seen), this makes no claim that there are none.
    Unseen,
    /// These, all of them; empty when the session has none.
    Seen(&'a [PullRequestFacts]),
}
