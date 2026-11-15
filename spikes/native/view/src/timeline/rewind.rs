//! Rewinding a conversation (protocol 14, ADR 0052; Paseo's rewind,
//! `components/rewind/rewind-menu.tsx`): where a person's message can be
//! rewound to, and what a rewind leaves behind.
//!
//! [Decision Log]
//! - Purpose: a person takes the conversation back to before one of
//!   their messages, as Paseo's "Rewind conversation" does; what came
//!   after it is no longer shown.
//! - Constraint: the node rewinds only to a turn's end the agent named (a
//!   point, the `turn_ended` record's `point`), between turns; its
//!   `rewound` record names that turn's end. The fold's entries are never
//!   removed (indices are kept by id, `fold.rs`).
//! - Alternatives: remove the entries (breaks every index the fold keeps);
//!   keep showing them (Paseo removes them; the agent no longer has them,
//!   so a reply that seems to remember them would confuse).
//! - Choice: the entries after the turn's end are marked rewound
//!   ([`Timeline::rewound`]): the screen draws them no more and they stay
//!   in the log. A message can be rewound to the turn end just before it
//!   when that end is a point ([`rewind_point`]).
//! - Cost: rewound entries still take a place in the list (drawn empty).

use vessel_proto::EventSeq;

use super::entry::EntryKind;
use super::fold::Timeline;

impl Timeline {
    /// The conversation went back to the turn end `after` (its `rewound`
    /// record): the entries after it are left behind, and redrawn.
    pub(super) fn rewound_to(&mut self, after: EventSeq) {
        let start = self.entries.partition_point(|entry| entry.seq <= after);
        let end = self.entries.len();
        if start >= end {
            return;
        }
        let already = self.batch.added_from;
        self.batch
            .changed
            .extend((start..end).filter(|index| *index < already));
        self.rewound.push(start..end);
    }

    /// Whether entry `index` was left behind by a rewind: it is no longer
    /// the agent's conversation and is not drawn.
    #[must_use]
    pub fn rewound(&self, index: usize) -> bool {
        self.rewound.iter().any(|range| range.contains(&index))
    }
}

/// Where the person's message at `index` can be rewound to: the turn end
/// just before it, when the agent named it (a point), skipping what an
/// earlier rewind left behind. `None` for a message that is not a prompt,
/// the first one, or one after a turn that failed or carried no point.
#[must_use]
pub fn rewind_point(timeline: &Timeline, index: usize) -> Option<EventSeq> {
    let entries = timeline.entries();
    if !matches!(entries.get(index)?.kind, EntryKind::Prompt(_)) {
        return None;
    }
    // The last turn boundary before it that is still the conversation's:
    // a turn's end (a point or not), a failed turn, or another prompt (a
    // turn that never ended).
    let (_, entry) = entries
        .get(..index)?
        .iter()
        .enumerate()
        .rev()
        .filter(|(at, _)| !timeline.rewound(*at))
        .find(|(_, entry)| {
            matches!(
                entry.kind,
                EntryKind::TurnEnded { .. } | EntryKind::TurnFailed { .. } | EntryKind::Prompt(_)
            )
        })?;
    if let EntryKind::TurnEnded { point: true, .. } = entry.kind {
        Some(entry.seq)
    } else {
        None
    }
}
