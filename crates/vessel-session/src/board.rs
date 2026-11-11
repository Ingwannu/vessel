//! The board's brain: which lane a session is in and what its card says
//! (protocol 8, ADR 0019).
//!
//! Owns: the facts a placement is derived from, typed, and [`derive`], a
//! pure function of them. Must not: gather facts (the session actor reads
//! its own records; pull requests will come from the node), keep state, or
//! read the clock (the caller passes `now`). None of these types is a wire
//! type: only the result crosses the wire, as `vessel_proto::SessionBoard`.
//!
//! [Decision Log]
//! - Purpose: every client must show the same board, so the node decides
//!   each session's lane and phrase, from facts only it has.
//! - Constraint: the owner asked for agent-orchestrator's board as it is.
//!   Its daemon decides the same two things with one pure reducer,
//!   `DeriveKanbanPresentation` in its `backend/pkg/contract/kanban.go`
//!   (Apache-2.0), covered by a table of cases in `kanban_test.go`.
//! - Alternatives: a reducer of Vessel's own, shaped by what a Vessel
//!   session reports today (no pull requests, so three of the four lanes
//!   would have no rule, and the next milestone would design them again);
//!   reducing in each client (two clients a version apart disagree).
//! - Choice: a port. The lane is chosen first, from lifecycle facts; the
//!   phrase is then derived from the facts that lane cares about, so a
//!   card never shows a phrase from a stage it is not in. With several
//!   pull requests each gets a lane, the most actionable wins (ready, then
//!   in review, then validating; ties by the most recently updated, then
//!   by URL so the board never flickers), and the phrase reads the winner.
//!   The reference's test table is ported case by case (`board/tests.rs`):
//!   the port is checked by it, not by eye.
//! - Differences from the reference, none of which changes one of its
//!   cases:
//!   1. Between turns with nothing delivered, a session is
//!      [`DisplayStatus::AwaitingPr`] only when the node can see pull
//!      requests ([`PullRequests::Seen`]); otherwise it is
//!      [`DisplayStatus::Idle`]. agent-orchestrator always sees them; no
//!      Vessel node does yet, and "awaiting PR" would be a claim about
//!      something it cannot see.
//!   2. Activity is Vessel's four-valued [`Activity`]. The reference also
//!      has `blocked`, which every one of its phrase functions treats
//!      exactly as `waiting_input`; Vessel's only way of waiting on the
//!      owner is a permission request, which is `waiting_input`. The
//!      reference's zero value (no activity reported) reads as idle in all
//!      its functions but one (`interruptedWhileIdle`), and none of its
//!      cases is interrupted without an activity; here it is `idle`.
//!   3. A pull request's lane is its own three-valued type ([`Lane`]), so
//!      the reference's fall-through arms (a priority for "no lane", the
//!      building phrase for a pull-request lane that cannot be building)
//!      do not exist.
//!   4. The phrase is a closed enum, not display text (the contract's
//!      reasons, `vessel_proto::board`).
//! - Must stay true: a change that breaks a ported case is a change of
//!   behaviour, made on purpose and recorded in ADR 0019, not a test to
//!   adjust.

use std::time::Duration;

use chrono::{DateTime, Utc};
use vessel_proto::{Activity, DisplayStatus, KanbanColumn};

mod facts;

pub use facts::{
    CiState, ExternalReviewFacts, Mergeability, Policy, PullRequestFacts, PullRequestUrl,
    PullRequests, ReviewDecision, ReviewRunFacts, SessionFacts, WorkerFacts,
};

/// How long a worker that should be reporting may stay silent, having never
/// reported, before its card says [`DisplayStatus::NoSignal`]. The
/// reference's `noSignalGrace`. No Vessel session expects a signal today
/// (see [`WorkerFacts::signal_expected`]), so this only matters once one
/// does.
pub const NO_SIGNAL_GRACE: Duration = Duration::from_secs(90);

/// A session's lane and the phrase for its card, derived from `session`
/// and its `pull_requests` as of `now`: the reference's
/// `DeriveKanbanPresentation`, with the differences the module docs list.
///
/// An ended session is in the archive whatever else is true. Without pull
/// requests a session is building, or validating if it delivered something
/// else for review. With pull requests the lane is chosen per pull request
/// and the most actionable one speaks for the session; a merged or closed
/// pull request speaks only once none is live, so a landed one cannot hide
/// live work.
#[must_use]
pub fn derive(
    session: &SessionFacts,
    pull_requests: PullRequests<'_>,
    now: DateTime<Utc>,
    no_signal_grace: Duration,
) -> (KanbanColumn, DisplayStatus) {
    if session.worker.is_terminated {
        return (KanbanColumn::Archive, DisplayStatus::Terminated);
    }
    let seen: &[PullRequestFacts] = match pull_requests {
        PullRequests::Unseen => &[],
        PullRequests::Seen(seen) => seen,
    };
    let clock = Clock {
        now,
        grace: no_signal_grace,
    };
    match choose(session, seen) {
        Some((lane, pr)) => (lane.column(), lane.phrase(session, pr, clock)),
        None if session.has_artifact_output => {
            (KanbanColumn::Validating, artifact_phrase(session, clock))
        }
        None => (
            KanbanColumn::Building,
            building_phrase(session, pull_requests, clock),
        ),
    }
}

/// The time the reducer reads, for the silence rule only.
#[derive(Debug, Clone, Copy)]
struct Clock {
    now: DateTime<Utc>,
    grace: Duration,
}

/// The lane one pull request puts its session in. Never building (a pull
/// request exists) and never the archive (the session's end decides that,
/// before any pull request is read).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lane {
    Validating,
    NeedsReview,
    Ready,
}

impl Lane {
    /// The reference's `derivePRKanbanColumn`.
    ///
    /// One early return per case of the reference's `switch`, in its
    /// order: several cases share a lane for different reasons, and an
    /// `else if` chain would have to merge them (clippy's
    /// `if_same_then_else`) and lose which reason is which.
    fn of(session: &SessionFacts, pr: &PullRequestFacts) -> Self {
        if pr.merged || pr.closed {
            return Self::Ready;
        }
        if pr.draft {
            return Self::Validating;
        }
        if externally_approved(pr) {
            return Self::Ready;
        }
        if fleet_turns_next(session, pr) {
            return Self::Validating;
        }
        // Auto review owns this head until its own pass approves it: a head
        // not reviewed yet, a pass that failed or was cancelled, and a pass
        // that asked for changes are all "not approved yet". Without auto
        // review, a changes-requested verdict is as far as the fleet goes,
        // so it releases the pull request (`fleet_turns_next`).
        if session.policy.auto_review && !own_review_approved(pr) {
            return Self::Validating;
        }
        if pr.mergeability == Mergeability::Mergeable {
            return Self::Ready;
        }
        // In its review cycle and no fleet loop is turning it: the next
        // turn is a person's (give the review, answer the feedback, decide
        // about a failing check).
        Self::NeedsReview
    }

    /// Lower is more actionable: the reference's `kanbanPriority`.
    const fn priority(self) -> u8 {
        match self {
            Self::Ready => 0,
            Self::NeedsReview => 1,
            Self::Validating => 2,
        }
    }

    const fn column(self) -> KanbanColumn {
        match self {
            Self::Validating => KanbanColumn::Validating,
            Self::NeedsReview => KanbanColumn::NeedsReview,
            Self::Ready => KanbanColumn::Ready,
        }
    }

    /// The reference's `displayStatusInColumn`, for the lanes a pull
    /// request can be in.
    fn phrase(self, session: &SessionFacts, pr: &PullRequestFacts, clock: Clock) -> DisplayStatus {
        match self {
            Self::Validating => validating_phrase(session, pr, clock),
            Self::NeedsReview => in_review_phrase(session, pr, clock),
            Self::Ready => ready_phrase(pr),
        }
    }
}

/// The pull request that speaks for the session, with its lane; `None`
/// when there are none.
fn choose<'a>(
    session: &SessionFacts,
    pull_requests: &'a [PullRequestFacts],
) -> Option<(Lane, &'a PullRequestFacts)> {
    // A terminal pull request must not hide a live one still moving
    // through either loop; merged and closed ones count only once nothing
    // is live. (The reference copies the live ones out; filtering in place
    // decides the same without allocating.)
    let any_live = pull_requests.iter().any(PullRequestFacts::is_live);
    pull_requests
        .iter()
        .filter(|pr| !any_live || pr.is_live())
        .map(|pr| (Lane::of(session, pr), pr))
        .reduce(|chosen, candidate| {
            if outranks(candidate, chosen) {
                candidate
            } else {
                chosen
            }
        })
}

/// Whether `candidate` is the more actionable placement: by lane, then the
/// most recently updated pull request, then the smaller URL, so the board
/// never flickers between equally ranked ones. The reference's
/// `outranksKanban`.
fn outranks(
    (lane, pr): (Lane, &PullRequestFacts),
    (current, chosen): (Lane, &PullRequestFacts),
) -> bool {
    if lane.priority() != current.priority() {
        return lane.priority() < current.priority();
    }
    if pr.updated_at != chosen.updated_at {
        return pr.updated_at > chosen.updated_at;
    }
    pr.url < chosen.url
}

/// The fleet's own pass approved the current head. A pass that requested
/// changes, one not run yet, and one that failed or was cancelled are all
/// "not approved". The reference's `approvedByAO`.
const fn own_review_approved(pr: &PullRequestFacts) -> bool {
    pr.review_run.outcome && !pr.review_run.changes_requested
}

/// Both the provider's aggregate decision (which honours dismissed
/// reviews) and a standing approval the fleet did not author. The
/// reference's `externallyApproved`.
fn externally_approved(pr: &PullRequestFacts) -> bool {
    pr.review == ReviewDecision::Approved && pr.external_review.approved
}

/// The fleet itself turns the review loop: its pass on the current head is
/// running, it is addressing review feedback, or it is fixing failing
/// checks. Otherwise the same loop goes on with a person taking the next
/// turn. The reference's `aoOwnsNextStep`.
fn fleet_turns_next(session: &SessionFacts, pr: &PullRequestFacts) -> bool {
    pr.review_run.running
        || (session.policy.auto_inject_review && pr.review_run.changes_requested)
        || (session.policy.auto_inject_ci && pr.ci == CiState::Failing)
}

/// Anyone, the fleet's pass or a person, asked for changes. The reference's
/// `changesRequestedOn`.
const fn changes_requested_on(pr: &PullRequestFacts) -> bool {
    pr.review_run.changes_requested || pr.external_review.changes_requested
}

/// The worker should be reporting, never has, and has been quiet past the
/// grace. The reference's `silentPastGrace`; a clock that went backwards
/// is not silence.
fn silent_past_grace(worker: &WorkerFacts, clock: Clock) -> bool {
    worker.signal_expected
        && !worker.has_signal
        && clock
            .now
            .signed_duration_since(worker.last_activity_at)
            .to_std()
            .is_ok_and(|silent| silent > clock.grace)
}

/// A worker that needs a person, has gone, or is silent outranks the loop
/// it was running: what it says in validating and in review, if anything.
fn stuck_worker_phrase(session: &SessionFacts, clock: Clock) -> Option<DisplayStatus> {
    match session.worker.activity {
        Activity::WaitingInput => Some(DisplayStatus::Blocked),
        Activity::Exited => Some(DisplayStatus::Exited),
        Activity::Active | Activity::Idle => {
            silent_past_grace(&session.worker, clock).then_some(DisplayStatus::NoSignal)
        }
    }
}

/// Building: worker progress, since a session with no pull request has no
/// delivery facts to report. The reference's `buildingDisplayStatus`, with
/// `Idle` where it cannot see pull requests (difference 1).
fn building_phrase(
    session: &SessionFacts,
    pull_requests: PullRequests<'_>,
    clock: Clock,
) -> DisplayStatus {
    match session.worker.activity {
        Activity::Active => DisplayStatus::Working,
        Activity::WaitingInput => DisplayStatus::Blocked,
        Activity::Exited => DisplayStatus::Exited,
        // A worker the person stopped mid-turn; without this it would read
        // as if it had finished and were waiting on its own pull request.
        Activity::Idle if session.interrupted => DisplayStatus::Interrupted,
        Activity::Idle if silent_past_grace(&session.worker, clock) => DisplayStatus::NoSignal,
        Activity::Idle => match pull_requests {
            PullRequests::Unseen => DisplayStatus::Idle,
            PullRequests::Seen(_) => DisplayStatus::AwaitingPr,
        },
    }
}

/// Validating with an artifact and no pull request. The reference's
/// `artifactDisplayStatus`.
fn artifact_phrase(session: &SessionFacts, clock: Clock) -> DisplayStatus {
    match session.worker.activity {
        Activity::Active => DisplayStatus::Working,
        Activity::WaitingInput => DisplayStatus::Blocked,
        Activity::Exited => DisplayStatus::Exited,
        Activity::Idle if session.interrupted => DisplayStatus::Interrupted,
        Activity::Idle if silent_past_grace(&session.worker, clock) => DisplayStatus::NoSignal,
        Activity::Idle => DisplayStatus::NeedsReview,
    }
}

/// Validating: the fleet-driven loop turning the pull request. The work the
/// agent is doing outranks the review pass that asked for it, and crediting
/// an auto-fix loop needs the worker active right now: a policy flag on an
/// idle worker falls through to the plain reading of checks and reviews
/// rather than claim work nobody is doing. The reference's
/// `validatingDisplayStatus`.
fn validating_phrase(session: &SessionFacts, pr: &PullRequestFacts, clock: Clock) -> DisplayStatus {
    if let Some(stuck) = stuck_worker_phrase(session, clock) {
        return stuck;
    }
    let active = session.worker.activity == Activity::Active;
    let failing = pr.ci == CiState::Failing;
    if failing && session.policy.auto_inject_ci && active {
        DisplayStatus::FixingCi
    } else if failing {
        DisplayStatus::CiFailing
    } else if changes_requested_on(pr) && session.policy.auto_inject_review && active {
        DisplayStatus::AddressingComments
    } else if pr.review_run.changes_requested {
        DisplayStatus::NeedsReview
    } else if session.policy.auto_review && !pr.review_run.present {
        DisplayStatus::ReviewScheduled
    } else if pr.review_run.running {
        DisplayStatus::Reviewing
    } else if pr.review_run.failed {
        DisplayStatus::ReviewFailed
    } else if pr.review_run.cancelled {
        DisplayStatus::ReviewPending
    } else if pr.draft {
        DisplayStatus::Draft
    } else {
        DisplayStatus::NeedsReview
    }
}

/// In review: the loop seen from the person's side. The lane rule already
/// sends failing checks under auto-fix, and changes the fleet addresses, to
/// validating, so the auto-fix arms change nothing today; they stay, as in
/// the reference, so this function is right on its own rather than relying
/// on a rule in another. The reference's `inReviewDisplayStatus`.
fn in_review_phrase(session: &SessionFacts, pr: &PullRequestFacts, clock: Clock) -> DisplayStatus {
    if let Some(stuck) = stuck_worker_phrase(session, clock) {
        return stuck;
    }
    let active = session.worker.activity == Activity::Active;
    let addressing = session.policy.auto_inject_review && active;
    let external = &pr.external_review;
    if pr.ci == CiState::Failing && session.policy.auto_inject_ci && active {
        DisplayStatus::FixingCi
    } else if pr.ci == CiState::Failing {
        DisplayStatus::CiFailing
    } else if (external.comments || external.changes_requested) && addressing {
        DisplayStatus::AddressingComments
    } else if external.changes_requested {
        DisplayStatus::ChangesRequested
    } else if external.comments {
        DisplayStatus::Commented
    } else {
        DisplayStatus::NeedsHumanReview
    }
}

/// Ready: how the pull request landed, or what is left between an approval
/// and the merge. Merged and closed come first: they are what happened,
/// and no reading of merge readiness overrides them. The reference's
/// `readyDisplayStatus`.
fn ready_phrase(pr: &PullRequestFacts) -> DisplayStatus {
    if pr.merged {
        DisplayStatus::Merged
    } else if pr.closed {
        DisplayStatus::Closed
    } else if pr.mergeability == Mergeability::Mergeable {
        DisplayStatus::Mergeable
    } else if pr.ci == CiState::Failing {
        DisplayStatus::CiFailing
    } else {
        DisplayStatus::Approved
    }
}

#[cfg(test)]
mod tests;
