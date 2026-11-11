//! What a person marks on a session (protocol 10, ADR 0026): pinned,
//! archived, unread. Kept by the node, so every client agrees.
//!
//! [Decision Log]
//! - Purpose: the sidebar and the board are shared views; a session one
//!   device pinned or archived is pinned or archived on every device.
//!   GOAT and Paseo keep these on the server; agent-orchestrator keeps
//!   unread in each browser, so two devices disagree.
//! - Unread without a race: a session is unread when it was marked so, or
//!   when a record asking for attention (a turn's end or failure, the
//!   session's end) is at or after the point the last `read` named. The
//!   attention point is a projection of the log and the read point a
//!   mark, so a `read` and a turn ending at the same moment agree in any
//!   order (GOAT compares a visit's time with a run's completion the same
//!   way).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::id::{EventSeq, SessionId};

/// A session's marks, as `SessionInfo.marks`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SessionMarks {
    /// Kept at the top of lists.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pinned: bool,
    /// When it was put away (the board's archive lane). An archived
    /// session has ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<DateTime<Utc>>,
    /// Something happened that nobody has marked seen.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unread: bool,
}

impl SessionMarks {
    /// Whether nothing is marked (the field is then left out).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        !self.pinned && self.archived_at.is_none() && !self.unread
    }
}

/// A mark to set (`session.mark`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "mark", rename_all = "snake_case")]
pub enum Mark {
    /// Keep it at the top.
    Pin,
    /// No longer.
    Unpin,
    /// Put it away; a live session is stopped first.
    Archive,
    /// Bring it back (it stays ended until started again).
    Unarchive,
    /// Every record before `through` has been seen. Required: "the end
    /// of the log now" could mark read a turn's end the reader never
    /// showed. A `through` behind an earlier one changes nothing.
    Read {
        /// The next record the reader has not seen.
        through: EventSeq,
    },
    /// Unread until the next `read`.
    Unread,
}

/// `session.mark`: set a mark. Needs `operate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MarkSession {
    /// Which session.
    pub session: SessionId,
    /// The mark.
    #[serde(flatten)]
    pub mark: Mark,
}

/// `session.delete`: remove an ended session, its log, its marks, its
/// queue and its attachments. Needs `operate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DeleteSession {
    /// Which session.
    pub session: SessionId,
}

/// `session.resume`: start an ended structured session again with its
/// conversation. Needs `operate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ResumeSession {
    /// Which session.
    pub session: SessionId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_have_their_wire_shape() {
        assert_eq!(
            serde_json::to_string(&SessionMarks::default()).unwrap(),
            "{}"
        );
        let session = SessionId::random();
        let read = MarkSession {
            session,
            mark: Mark::Read {
                through: EventSeq::new(12),
            },
        };
        let json = serde_json::to_string(&read).unwrap();
        assert_eq!(
            json,
            format!(r#"{{"session":"{session}","mark":"read","through":12}}"#)
        );
        assert_eq!(serde_json::from_str::<MarkSession>(&json).unwrap(), read);
        assert!(SessionMarks::default().is_empty());
    }
}
