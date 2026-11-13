//! Forking a conversation and rewinding one (protocol 12, milestone 11's
//! track A2, ADR 0052): only where the agent can, at the end of a turn.
//!
//! [Decision Log]
//! - Purpose: a person goes back to a point of a conversation, in place
//!   (rewind) or in a new session beside it (fork), as GOAT, Paseo and
//!   agent-orchestrator offer.
//! - Constraint: over ACP no adapter rewinds in place; both installed ones
//!   fork (`session/fork`), at a point when the request names an agent
//!   message (`_meta.jetbrains.air.fork`). The anchor is advertised
//!   nowhere, so an agent that ignores it would fork the whole
//!   conversation.
//! - Choice: a rewind is a fork at the point that the session's agent then
//!   continues; a point is a turn's end the agent named (`TurnPoint`);
//!   every fork at a point is checked by loading it and reading where its
//!   replay ends, and refused when it is not there. What a rewind leaves
//!   behind stays in the log and in the agent's history.
//! - Cost: files are not put back (no ACP surface); a rewind says how many
//!   tool calls changed them.

use serde::{Deserialize, Serialize};

use crate::id::{EventSeq, SessionId};

/// `session.fork`: a new session that continues `session`'s conversation
/// as it was after the turn whose end is record `after`, or as it is now.
/// Needs `operate`; answers the new session, as `session.create` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ForkSession {
    /// The session forked.
    pub session: SessionId,
    /// A `turn_ended` record of it that carries a point; absent for the
    /// whole conversation (refused while a turn runs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<EventSeq>,
}

/// `session.rewind`: the session goes back to the end of the turn whose
/// end is record `after`; its agent continues from there. Between turns
/// only. Needs `operate`; answers the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RewindSession {
    /// Which session.
    pub session: SessionId,
    /// A `turn_ended` record of it that carries a point.
    pub after: EventSeq,
}
