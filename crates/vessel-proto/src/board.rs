//! Where a session sits on the board (protocol 8, milestone 9, ADR 0019).
//!
//! [Decision Log]
//! - Purpose: a client showing sessions as a board needs to know, for each
//!   one, which lane it is in and what to say on its card. Both are derived
//!   from facts only the node has (whether a turn is running, whether a
//!   permission waits, and later the state of its pull requests), so the
//!   node derives them and every client shows the same board.
//! - Constraint: milestone 8's board had to place cards by
//!   [`SessionStatus`](crate::SessionStatus) alone, because that was all a
//!   node reported: it could not tell an agent at work from one waiting on
//!   its owner. Deriving that in a client means replaying each session's
//!   event log, once per client, and still guessing for terminal sessions.
//! - Alternatives: send the raw facts and let each client reduce them (two
//!   clients then disagree as soon as one is a version behind); send the
//!   phrase as text, as agent-orchestrator's daemon does (a string that
//!   holds one of a known set of values, which AGENTS.md section 2
//!   forbids, and which a client could not match exhaustively).
//! - Choice: the model is agent-orchestrator's (its `contract/kanban.go`),
//!   ported: a [`KanbanColumn`] chosen first from lifecycle facts, then a
//!   [`DisplayStatus`] derived inside that column, so a session never shows
//!   a phrase belonging to a stage it is not in. Both are closed enums. The
//!   reducer itself lives in `vessel-session`; only its result crosses the
//!   wire, with the [`Activity`] it was derived from.
//! - Must stay true: these are derived, never stored and never sent by a
//!   client. A node that predates protocol 8 sends no [`SessionBoard`], and
//!   a client then keeps the placement the session's status implies.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The lane a session is in: where it sits between first prompt and merge,
/// and which loop is turning it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum KanbanColumn {
    /// A live session with no pull request yet.
    Building,
    /// A pull request inside a loop the fleet drives itself: a draft, a
    /// review pass running on the current head, feedback being addressed,
    /// failing checks being fixed.
    Validating,
    /// A pull request in its review cycle whose next turn is a person's:
    /// give the review, answer the feedback, or decide about a failing
    /// check.
    NeedsReview,
    /// A pull request merged, closed, mergeable, or approved by a person.
    Ready,
    /// A session that has ended. Not a lane: clients list these apart.
    Archive,
}

/// What is happening right now in the lane a session already sits in. It
/// changes with turns, permissions, checks and reviews while the lane stays
/// put.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DisplayStatus {
    // Building.
    /// The agent is in a turn.
    Working,
    /// The agent waits on its owner: a permission to answer.
    Blocked,
    /// The agent's process is gone though the session is not archived.
    Exited,
    /// The agent should be reporting and has been silent past the grace.
    NoSignal,
    /// The last turn was stopped before it finished, and nothing has been
    /// asked since.
    Interrupted,
    /// Between turns, with nothing delivered yet, on a node that can see
    /// pull requests.
    AwaitingPr,
    /// Between turns, on a node that cannot see pull requests. (Not in
    /// agent-orchestrator's set: there every session has a repository.)
    Idle,
    // Validating.
    /// Failing checks are being fixed by the agent.
    FixingCi,
    /// Review feedback is being addressed by the agent.
    AddressingComments,
    /// The fleet's own review pass asked for changes nobody is addressing.
    NeedsReview,
    /// A review pass is owed to the current head and has not started.
    ReviewScheduled,
    /// A review pass is running on the current head.
    Reviewing,
    /// A review pass ended without a verdict.
    ReviewFailed,
    /// A review pass was cancelled.
    ReviewPending,
    /// The pull request is a draft.
    Draft,
    // In review.
    /// Checks fail and nobody is fixing them.
    CiFailing,
    /// A person commented without requesting changes.
    Commented,
    /// A person requested changes.
    ChangesRequested,
    /// The pull request waits for a person's review.
    NeedsHumanReview,
    // Ready.
    /// The pull request can be merged.
    Mergeable,
    /// A person approved it.
    Approved,
    /// It was merged.
    Merged,
    /// It was closed without merging.
    Closed,
    // Archive.
    /// The session has ended.
    Terminated,
}

/// The latest state of a session's agent, as the node observed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    /// In a turn (structured), or its terminal is producing output.
    Active,
    /// Between turns, or its terminal is quiet.
    Idle,
    /// Waiting on its owner: a permission request is unanswered.
    WaitingInput,
    /// Its process has ended.
    Exited,
}

/// A session's place on the board, derived by the node (protocol 8).
///
/// `#[non_exhaustive]`: pull-request evidence joins it when a node can see
/// pull requests; build one with [`SessionBoard::new`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[non_exhaustive]
pub struct SessionBoard {
    /// The lane.
    pub column: KanbanColumn,
    /// The phrase for its card, derived inside `column`.
    pub display_status: DisplayStatus,
    /// What the agent was last seen doing.
    pub activity: Activity,
    /// When the node last saw the agent do something: a turn start or end,
    /// a permission asked or answered, terminal output.
    pub last_activity_at: DateTime<Utc>,
}

impl SessionBoard {
    /// A placement derived at `last_activity_at`.
    #[must_use]
    pub const fn new(
        column: KanbanColumn,
        display_status: DisplayStatus,
        activity: Activity,
        last_activity_at: DateTime<Utc>,
    ) -> Self {
        Self {
            column,
            display_status,
            activity,
            last_activity_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_values_cross_the_wire_in_snake_case() {
        let board = SessionBoard::new(
            KanbanColumn::NeedsReview,
            DisplayStatus::AwaitingPr,
            Activity::WaitingInput,
            DateTime::<Utc>::UNIX_EPOCH,
        );
        let json = serde_json::to_string(&board).unwrap();
        assert_eq!(
            json,
            r#"{"column":"needs_review","display_status":"awaiting_pr","activity":"waiting_input","last_activity_at":"1970-01-01T00:00:00Z"}"#
        );
        assert_eq!(serde_json::from_str::<SessionBoard>(&json).unwrap(), board);
        assert_eq!(
            serde_json::to_string(&DisplayStatus::FixingCi).unwrap(),
            r#""fixing_ci""#
        );
        assert_eq!(
            serde_json::to_string(&DisplayStatus::CiFailing).unwrap(),
            r#""ci_failing""#
        );
    }
}
