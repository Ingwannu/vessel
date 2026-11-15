//! Forking a conversation (protocol 14, ADR 0052; Paseo's fork,
//! `components/assistant-fork-menu.tsx` in the turn footer): where a
//! turn's footer offers a fork, and where a forked session came from.
//!
//! [Decision Log]
//! - Purpose: a person continues a conversation from one of its turns in
//!   a new session beside it, as Paseo's "Fork chat from here" does.
//! - Constraint: the node forks at a turn's end the agent named (the
//!   `turn_ended` record's `point`), live or ended; without a point it
//!   forks only the whole conversation, refused while a turn runs. A
//!   fork's log opens with its `forked` record (`from`, `after`).
//! - Alternatives: offer the fork on every turn (the node refuses one
//!   without a point, so the person learns it only by asking); Paseo's
//!   fork from the working turn (the whole conversation as it streams:
//!   our node refuses that while the turn runs).
//! - Choice: a turn's footer offers the fork where its end carries a
//!   point and a rewind has not left it behind ([`fork_point`]); a forked
//!   session's first line names its source (its `forked` entry, the
//!   title from the fleet, `Fleet::title_of`).
//! - Cost: an agent that names no points (an older one, a turn that ended
//!   on a tool call) offers no fork; no fork from a running turn.

use vessel_proto::{EventSeq, SessionStatus};

use super::entry::EntryKind;
use super::fold::Timeline;

/// Where the turn whose footer is entry `index` can be forked from: its
/// end, when the agent named it (a point) and no rewind left it behind.
/// `None` for an entry that is not a turn's end.
#[must_use]
pub fn fork_point(timeline: &Timeline, index: usize) -> Option<EventSeq> {
    let entry = timeline.entries().get(index)?;
    if timeline.rewound(index) {
        return None;
    }
    if let EntryKind::TurnEnded { point: true, .. } = entry.kind {
        Some(entry.seq)
    } else {
        None
    }
}

/// Why a fork never held a conversation (protocol 14: a `failed` status
/// with `empty_fork`, ADR 0052's addendum), in the person's terms. Nothing
/// can be resumed; the screen offers to remove it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyFork {
    /// The agent did not fork at the turn chosen (it ignored the point).
    NotAtPoint,
    /// The agent cannot fork a conversation.
    CannotFork,
    /// It did not start for another reason; the session retains its diagnostic.
    Other,
}

/// The node's sentences for a refused fork (`vessel-session`'s
/// `fork_refusal`), read as the person's words above.
const NOT_AT_POINT: &str = "the agent did not fork at the chosen point";
const CANNOT_FORK: &str = "the agent cannot fork a conversation";

/// Whether `status` is an empty fork's, and why.
#[must_use]
pub fn empty_fork(status: &SessionStatus) -> Option<EmptyFork> {
    let SessionStatus::Failed {
        message,
        empty_fork: true,
    } = status
    else {
        return None;
    };
    Some(match message.trim() {
        NOT_AT_POINT => EmptyFork::NotAtPoint,
        CANNOT_FORK => EmptyFork::CannotFork,
        _ => EmptyFork::Other,
    })
}

impl EmptyFork {
    /// Borrow a safe whole sentence; no diagnostic or allocation enters a frame.
    #[must_use]
    pub fn words(self, catalog: &crate::localization::Translator) -> &str {
        let words = catalog.fork();
        match self {
            Self::NotAtPoint => words.empty_not_at_point(),
            Self::CannotFork => words.empty_cannot_fork(),
            Self::Other => words.empty_other(),
        }
    }
}
