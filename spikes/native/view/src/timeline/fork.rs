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

use vessel_proto::EventSeq;

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
