//! `session.reload` (TRACKER 803, ADR 0110): Paseo's "Reload agent". The
//! session's agent is started again on the same conversation, so that it
//! picks up skills, MCP servers and its login, and the log takes in what
//! the agent's own history has beyond it.
//!
//! [Decision Log]
//! - Purpose: Paseo's `refresh_agent_request` (server `session.ts`,
//!   `handleRefreshAgentRequest`): unarchive, interrupt a running turn, reload
//!   the agent on its persisted session, rehydrate the timeline from the
//!   provider, answer `agent_refreshed` with the timeline's size.
//! - Constraint: Vessel's log is the cursor every client holds (ADR 0004),
//!   so it is never wiped; ADR 0063 had specified a reload that drops the
//!   agent's replay. ADR 0110 supersedes that section.
//! - Choice: one call, `operate`, forwardable, refused to agents (a person
//!   reloads; Paseo has no MCP tool for it). Its reply is the session's
//!   info, whose `next_seq` is Paseo's `timelineSize`. The log gains
//!   [`SessionEvent::Reloaded`](crate::SessionEvent::Reloaded) once the
//!   agent is ready again, saying how many records the agent's history
//!   added (turns made outside Vessel); an older peer reads it as a log line.
//! - Cost: a peer older than [`RELOAD_PROTOCOL`] cannot reload; its clients
//!   do not offer the row.

use serde::{Deserialize, Serialize};

use crate::id::SessionId;
use crate::node::ProtocolVersion;

/// The protocol of `session.reload` and its `reloaded` record. Drafted as
/// 31; the integrator assigns the number at merge.
pub const RELOAD_PROTOCOL: Option<ProtocolVersion> = Some(ProtocolVersion::new(31));

/// `session.reload`: start the session's agent again on the same
/// conversation, answered once it is ready. Needs `operate`; refused to
/// agents.
///
/// An archived session is unarchived first. A live one has its running
/// turn cancelled (refused with `conflict`, the turn kept, when the turn
/// does not end within the grace a forced move waits), its process
/// stopped and started again with `session/load` and a fresh launch. A
/// parked one is started with that launch, and an ended one is started
/// again as `session.resume` does. A terminal session, one with no
/// conversation yet, and one whose agent cannot load a conversation are
/// `unsupported`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReloadSession {
    /// Which session.
    pub session: SessionId,
}

/// The line an older peer reads instead of a `reloaded` record.
#[must_use]
pub fn reloaded_line(appended: u32) -> String {
    match appended {
        0 => "vessel: the agent was reloaded".to_owned(),
        1 => "vessel: the agent was reloaded; 1 record of its own history was added".to_owned(),
        _ => format!(
            "vessel: the agent was reloaded; {appended} records of its own history were added"
        ),
    }
}

#[cfg(test)]
mod tests;
