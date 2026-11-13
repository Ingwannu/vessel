//! Parking an idle agent and waking it (protocol 12, milestone 11's track
//! A2, ADR 0038): a structured session whose agent can resume has its agent
//! process stopped after a time without activity, and started again with
//! its conversation (`session/load`) by whatever next needs it.
//!
//! [Decision Log]
//! - Purpose: an idle agent process holds hundreds of MiB for nothing; all
//!   three references release it (agent-orchestrator after 10 minutes,
//!   GOAT after 30, Paseo after an hour) and bring it back on the next
//!   prompt.
//! - Alternatives: a new `SessionStatus` variant (the contract's first
//!   form). Its review found it reaching protocol 10 and 11 peers through
//!   replies passed on unchanged (a mark's answer, an export, an import)
//!   and through status records, whose decoders would fail; and every
//!   `is_live` and every client's "running" check would have to learn it.
//! - Choice: the status stays `running`; `SessionInfo.parked` says the
//!   agent is parked and why, and the log has a `parked` and a `woken`
//!   record (both `since 12`, a `vessel:` line for older peers). An older
//!   client sees a running session whose prompt takes a moment longer.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::id::SessionId;

/// Why a session's agent is parked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum ParkReason {
    /// Nothing happened in it for the node's `park_after`.
    Idle,
    /// Someone asked (`session.park`).
    Asked,
    /// It was parked, and starting its agent again failed; the next prompt
    /// of a person, `session.wake` or the queue's `resume` tries again.
    WakeFailed {
        /// Why, as the agent or the node said it (at most
        /// [`ParkReason::MESSAGE_BYTES`]).
        message: String,
    },
    /// A reason a newer node gives.
    #[serde(other)]
    Unknown,
}

impl ParkReason {
    /// The longest `wake_failed` message kept.
    pub const MESSAGE_BYTES: usize = 1024;

    /// A failed wake, its message cut to [`Self::MESSAGE_BYTES`] on a
    /// character's boundary.
    #[must_use]
    pub fn wake_failed(message: &str) -> Self {
        let end = (0..=Self::MESSAGE_BYTES.min(message.len()))
            .rev()
            .find(|&at| message.is_char_boundary(at))
            .unwrap_or(0);
        Self::WakeFailed {
            message: message.get(..end).unwrap_or_default().to_owned(),
        }
    }

    /// The reason in a few words, for a log line.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Idle => "idle".to_owned(),
            Self::Asked => "asked".to_owned(),
            Self::WakeFailed { message } => format!("waking it failed: {message}"),
            Self::Unknown => "a newer node's reason".to_owned(),
        }
    }
}

/// A parked session's agent: since when, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Parked {
    /// When it was parked (or its last wake failed).
    pub since: DateTime<Utc>,
    /// Why.
    pub reason: ParkReason,
}

/// `session.park`: stop an idle session's agent now, keeping the session
/// and its conversation. Needs `operate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ParkSession {
    /// Which session.
    pub session: SessionId,
}

/// `session.wake`: start a parked session's agent again with its
/// conversation, answered once it is ready. Needs `operate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WakeSession {
    /// Which session.
    pub session: SessionId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wake_failure_is_cut_on_a_character() {
        let long = "가".repeat(600);
        let ParkReason::WakeFailed { message } = ParkReason::wake_failed(&long) else {
            panic!("a wake failure");
        };
        assert!(message.len() <= ParkReason::MESSAGE_BYTES);
        assert!(long.starts_with(&message));
    }

    #[test]
    fn older_peers_read_the_records_as_log_lines() {
        use crate::agent::AgentEvent;
        use crate::event::SessionEvent;
        use crate::node::ProtocolVersion;

        let parked = SessionEvent::Parked {
            reason: ParkReason::Idle,
        };
        assert_eq!(parked.for_protocol(ProtocolVersion::new(12)), None);
        let Some(SessionEvent::Agent {
            event: AgentEvent::HarnessLog { line },
        }) = parked.for_protocol(ProtocolVersion::new(11))
        else {
            panic!("a log line");
        };
        assert!(line.contains("stopped (idle)"), "{line}");
        assert!(
            SessionEvent::Woken
                .for_protocol(ProtocolVersion::new(2))
                .is_some()
        );
    }

    #[test]
    fn reasons_round_trip_and_a_newer_one_is_unknown() {
        let reason = ParkReason::wake_failed("no transcript");
        let text = serde_json::to_string(&reason).unwrap();
        assert_eq!(
            text,
            r#"{"reason":"wake_failed","message":"no transcript"}"#
        );
        assert_eq!(serde_json::from_str::<ParkReason>(&text).unwrap(), reason);
        assert_eq!(
            serde_json::from_str::<ParkReason>(r#"{"reason":"low_memory"}"#).unwrap(),
            ParkReason::Unknown
        );
    }
}
