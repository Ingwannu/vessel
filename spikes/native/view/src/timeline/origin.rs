//! The conversation a fork came from (slice F1b): which session to read,
//! and where reading its log stops, so the fork shows what its agent
//! remembers above its own first line.
//!
//! [Decision Log]
//! - Purpose: a fork opened empty and hid what its agent carries over
//!   (the coordinator on F1: the origin's conversation up to the fork
//!   point shows above the "Forked from" line, read-only).
//! - The reference: Paseo shows no earlier turns in a fork; its draft
//!   carries the conversation as one composer attachment, "Chat history"
//!   / "Previous conversation" (`activity-curator.ts`,
//!   `attachment-pill-content.tsx`). Our node forks the agent's own
//!   history instead, so the turns are real records of the origin's log.
//! - Constraint: the fork's log names its origin and the turn's end it
//!   continues from (`forked`: `from`, `after`); a whole-conversation
//!   fork names no record, only when it was made (the record's time).
//!   `session.read` reads a log page by page from its start.
//! - Choice: read the origin's log from its start and keep its records
//!   up to the fork's record (or, for a whole-conversation fork, those
//!   made by the time the fork was), [`Until::keeps`]; fold them as their
//!   own timeline, so the origin's rewinds before the point still hold.
//! - Cost: a fork of a fork shows its origin's "Forked from" line, not
//!   the origin's own origin; the whole origin is read before it shows.

use chrono::{DateTime, Utc};
use vessel_proto::{EventRecord, EventSeq, SessionId};

use super::entry::{Entry, EntryKind};

/// Where a read of the origin's log stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Until {
    /// After this record: the turn's end the fork continues from.
    Record(EventSeq),
    /// After the records made by then: when a whole-conversation fork
    /// was made.
    Time(DateTime<Utc>),
}

impl Until {
    /// Whether `record` of the origin's log is part of what the fork
    /// carries. The log is in order, so the first record it does not keep
    /// ends the read.
    #[must_use]
    pub fn keeps(&self, record: &EventRecord) -> bool {
        match self {
            Self::Record(after) => record.seq <= *after,
            Self::Time(made) => record.at <= *made,
        }
    }
}

/// The session this conversation was forked from and where its part
/// ends, when `entries` hold a `forked` line (a fork's log opens with
/// it).
#[must_use]
pub fn origin_of(entries: &[Entry]) -> Option<(SessionId, Until)> {
    entries.iter().find_map(|entry| {
        if let EntryKind::Forked { from, after } = entry.kind {
            Some((from, after.map_or(Until::Time(entry.at), Until::Record)))
        } else {
            None
        }
    })
}
