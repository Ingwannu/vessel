//! A requested tail window, bounded by actual matching records rather than reserved capacity.

use std::collections::VecDeque;
use std::num::ParseIntError;
use std::str::FromStr;
use vessel_proto::EventRecord;

/// A record count parsed before connecting; zero deliberately selects no history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tail(usize);

impl Tail {
    /// Paseo's omitted follow history window, shared by parser and reader.
    pub(crate) const FOLLOW_DEFAULT: Self = Self(10);

    /// An empty requested window differs from an omitted limit.
    pub(crate) const ZERO: Self = Self(0);

    /// Expose the count only where retaining records needs it.
    const fn get(self) -> usize {
        self.0
    }
}

impl std::fmt::Display for Tail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl FromStr for Tail {
    type Err = ParseIntError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse().map(Self)
    }
}

/// Holds only the requested last matches, including the explicit empty window.
pub(crate) struct Window {
    limit: Tail,
    records: VecDeque<EventRecord>,
}

impl Window {
    /// Allocate only for actual matches, never from the requested count alone.
    pub(crate) fn new(limit: Tail) -> Self {
        Self {
            limit,
            records: VecDeque::new(),
        }
    }

    /// Evict before admission so the window never exceeds its selected count.
    pub(crate) fn push(&mut self, record: EventRecord) {
        if self.limit == Tail::ZERO {
            return;
        }
        if self.records.len() == self.limit.get() {
            drop(self.records.pop_front());
        }
        self.records.push_back(record);
    }

    /// Transfer the retained records to rendering without cloning their payloads.
    pub(crate) fn into_records(self) -> VecDeque<EventRecord> {
        self.records
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;
    use vessel_proto::{EventSeq, SessionEvent, SessionStatus};

    fn record(event: SessionEvent) -> EventRecord {
        EventRecord {
            seq: EventSeq::ZERO,
            at: DateTime::UNIX_EPOCH,
            event,
        }
    }
    #[test]
    fn the_tail_window_evicts_across_batches_and_zero_never_holds_a_record() {
        for (limit, expected) in [("0", Vec::new()), ("2", vec![3, 4])] {
            let mut window = Window::new(limit.parse().unwrap());
            for batch in [[0, 1], [2, 3], [4, 5]] {
                for seq in batch {
                    let mut record = record(SessionEvent::Status {
                        status: SessionStatus::Running,
                    });
                    record.seq = EventSeq::new(seq);
                    // Selection precedes tail: the last raw item is not a match.
                    if seq != 5 {
                        window.push(record);
                    }
                }
            }
            assert_eq!(
                window
                    .into_records()
                    .into_iter()
                    .map(|record| record.seq.get())
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}
