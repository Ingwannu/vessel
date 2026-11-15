//! A parked session as the screens say it (protocol 12, ADR 0038; Paseo's
//! "Release idle agents", `idle-runtime-cleanup`): a live session whose
//! agent's process the node stopped while it sat idle, its conversation
//! kept; the next prompt starts the agent again.
//!
//! [Decision Log]
//! - Purpose: a parked session stays `running` on the wire (ADR 0038), so
//!   a screen reading its status alone would call it running though no
//!   agent runs; the conversation bar, the sidebar row and the board card
//!   must each say it was released.
//! - The reference: Paseo releases an idle agent after an hour ("Stops
//!   the agent's process while keeping its conversation. Send a message
//!   to resume."), files it with the finished agents, and a failed resume
//!   leaves it closed and to be tried again.
//! - Choice: Paseo's word, "released" ("해제됨"), wherever a session is
//!   shown, quiet as an idle session is; a failed wake is "resume failed",
//!   which asks for the person (it waits on them: a send or Try again).
//! - Cost: the node's own board phrase for the session (idle, or awaiting
//!   a pull request) gives way to it while the agent is released.

use vessel_proto::SessionInfo;
use vessel_proto::parking::ParkReason;

/// How a session's agent is parked, as the screens tell it apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParkState {
    /// Released while idle, or because someone asked: the next prompt
    /// resumes it.
    Released,
    /// Starting it again failed: the node's words.
    ResumeFailed(String),
}

/// How `session`'s agent is parked; `None` when it is not (or the session
/// has ended: an ended session is not parked).
#[must_use]
pub fn park_state(session: &SessionInfo) -> Option<ParkState> {
    if !session.status.is_live() {
        return None;
    }
    let parked = session.parked.as_ref()?;
    Some(match &parked.reason {
        ParkReason::WakeFailed { message, .. } => ParkState::ResumeFailed(message.clone()),
        ParkReason::Idle | ParkReason::Asked | ParkReason::Unknown => ParkState::Released,
    })
}
