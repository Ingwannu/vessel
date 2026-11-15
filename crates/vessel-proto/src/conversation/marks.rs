//! What a person marks on a session (protocol 10, ADR 0026): pinned,
//! archived, unread. Kept by the node, so every client agrees.
//!
//! [Decision Log]
//! - Purpose: the sidebar and the board are shared views; a session one
//!   device pinned or archived is pinned or archived on every device.
//!   GOAT and Paseo keep these on the server; agent-orchestrator keeps
//!   unread in each browser, so two devices disagree.
//! - Unread without a race: a session is unread when it was marked so, or
//!   when a record asking for attention (a turn's end or failure, a
//!   permission or a question asked, the session's end) is at or after the
//!   point the last `read` named. The
//!   attention point is a projection of the log and the read point a
//!   mark, so a `read` and a turn ending at the same moment agree in any
//!   order (GOAT compares a visit's time with a run's completion the same
//!   way).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agent::AgentEvent;
use crate::event::SessionEvent;
use crate::id::{EventSeq, SessionId, WorktreeId};
use crate::workspace::RemoveWorktreeOutcome;

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

/// A session's marks as the node keeps them, before `unread` is derived
/// from the log: what `session.mark` sets. Not a wire type: the store
/// keeps it in the session's row, and the session runtime holds the
/// store's copy while the session is in it, so both derive
/// [`SessionMarks`] with [`marks`](Self::marks) and agree.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeptMarks {
    /// Pinned.
    pub pinned: bool,
    /// When it was archived; the first time, if archived twice.
    pub archived_at: Option<DateTime<Utc>>,
    /// The furthest `read`'s `through`; `None` before the first `read`.
    pub read: Option<EventSeq>,
    /// Marked `unread` since the last `read`.
    pub marked_unread: bool,
}

impl KeptMarks {
    /// The marks a client sees, given `attention`, the `seq` of the last
    /// record that asks for attention ([`SessionEvent::asks_for_attention`]).
    ///
    /// Unread when marked so, or when that record is at or after the
    /// point the last `read` named. No `read` yet reads as a `read`
    /// through 0: every record asking for attention is unread until the
    /// session is first read, as a new message is. A session with no such
    /// record (one stored before protocol 10 included) is not unread.
    #[must_use]
    pub fn marks(&self, attention: Option<EventSeq>) -> SessionMarks {
        let read = self.read.unwrap_or(EventSeq::ZERO);
        SessionMarks {
            pinned: self.pinned,
            archived_at: self.archived_at,
            unread: self.marked_unread || attention.is_some_and(|seq| seq >= read),
        }
    }

    /// Whether it is archived.
    #[must_use]
    pub const fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}

impl SessionEvent {
    /// Whether a record of this event asks a person for attention, so the
    /// session is unread from it on until a `read` covers it (ADR 0026): a
    /// turn's end or failure, a permission or a question asked, and the
    /// session's end. The one definition the store's projection and the
    /// session runtime both use.
    #[must_use]
    pub const fn asks_for_attention(&self) -> bool {
        match self {
            Self::Status { status } => !status.is_live(),
            Self::Agent { event } => match event {
                AgentEvent::TurnEnded { .. }
                | AgentEvent::TurnFailed { .. }
                | AgentEvent::PermissionRequested(_) => true,
                AgentEvent::QuestionAsked(question) => question.delivery_state.is_pending(),
                AgentEvent::Ready { .. }
                | AgentEvent::TurnStarted { .. }
                | AgentEvent::UserMessageChunk { .. }
                | AgentEvent::MessageChunk { .. }
                | AgentEvent::ThoughtChunk { .. }
                | AgentEvent::ToolCall(_)
                | AgentEvent::ToolCallUpdate(_)
                | AgentEvent::Plan { .. }
                | AgentEvent::PermissionResolved { .. }
                | AgentEvent::Notification(_)
| AgentEvent::Compaction(_)
| AgentEvent::Plugin(_)
            | AgentEvent::CommandAccepted { .. }
| AgentEvent::Error { .. }
| AgentEvent::HarnessLog { .. }
                | AgentEvent::Steered { .. }
                | AgentEvent::QuestionResolved { .. } => false,
            },
            Self::Terminal(_)
            | Self::ProfileChanged { .. }
            | Self::ModelCall(_)
            | Self::OptionsChanged { .. }
            | Self::Renamed { .. }
            | Self::QueueChanged { .. }
            | Self::CommandsChanged { .. }
            | Self::UsageChanged { .. }
            // A parked agent waits on nobody; a failed wake answered the
            // call that asked for it.
            | Self::Parked { .. }
            | Self::Woken
            | Self::Forked { .. }
            | Self::Rewound { .. } => false,
        }
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

/// `session.delete`'s answer: the session is gone; what became of the
/// worktree made for it.
///
/// [Decision Log] (milestone 11, where tracks A and B meet)
/// - Purpose: deleting a session tidies the worktree made for it (ADR
///   0027's rule), and that removal may keep the worktree: uncommitted
///   work, commits no other branch has, another session in it. A delete
///   must never lose that work, nor keep it without saying so.
/// - Alternatives: refuse the delete while the worktree would lose work
///   (two calls for a person, and retention, which deletes on its own,
///   needs the keeping rule anyway); snapshot it unasked (a decision about
///   a person's work made for them); answer `Ack` and leave the kept
///   worktree to be noticed in `worktree.list` (silent).
/// - Choice: the session is deleted; the worktree goes only by the rule a
///   person's `worktree.remove { if_lost: keep }` follows, and this answer
///   says which it was, so a client can offer `snapshot` or `discard`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SessionDeleted {
    /// The worktree made for the session and what tidying did with it;
    /// absent when the session had none of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<TidiedWorktree>,
}

/// A session's worktree, tidied with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TidiedWorktree {
    /// Which worktree.
    pub worktree: WorktreeId,
    /// `removed`, or `kept { reasons }` (nothing of it was removed).
    #[serde(flatten)]
    pub outcome: RemoveWorktreeOutcome,
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

    /// Unread compares positions, so a `read` and a turn's end agree in
    /// either order; no `read` yet leaves every such record unread.
    #[test]
    fn unread_is_the_last_attention_against_the_last_read() {
        let seq = EventSeq::new;
        let unread = |read: Option<u64>, attention: Option<u64>| {
            KeptMarks {
                read: read.map(seq),
                ..KeptMarks::default()
            }
            .marks(attention.map(seq))
            .unread
        };
        assert!(!unread(None, None), "nothing asked");
        assert!(unread(None, Some(0)), "never read");
        assert!(unread(Some(4), Some(4)), "the record the read stopped at");
        assert!(!unread(Some(5), Some(4)), "read past it");
        assert!(unread(Some(5), Some(7)), "a later one");
        let marked = KeptMarks {
            marked_unread: true,
            read: Some(seq(9)),
            ..KeptMarks::default()
        };
        assert!(marked.marks(Some(seq(1))).unread, "marked unread");
    }

    #[test]
    fn what_asks_for_attention() {
        use crate::agent::StopReason;
        use crate::session::SessionStatus;
        let ended = SessionEvent::Agent {
            event: AgentEvent::TurnEnded {
                reason: StopReason::EndTurn,
                usage: None,
                point: None,
            },
        };
        assert!(ended.asks_for_attention());
        let exited = SessionEvent::Status {
            status: SessionStatus::Exited { code: Some(0) },
        };
        assert!(exited.asks_for_attention());
        let running = SessionEvent::Status {
            status: SessionStatus::Running,
        };
        assert!(!running.asks_for_attention());
        let started = SessionEvent::Agent {
            event: AgentEvent::TurnStarted {
                content: Vec::new(),

                client_message_id: None,
            },
        };
        assert!(!started.asks_for_attention());
    }
}
