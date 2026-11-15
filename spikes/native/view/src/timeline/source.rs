//! Exact source membership complements conservative first/through bounds.
//! A late update can make spans overlap; a cursor inside a span is not proof
//! that its record contributed to that entry. Most rows need no extra allocation.

use super::Entry;
use vessel_proto::EventSeq;

/// Additional records that changed/invalidated a row after its creating record.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtraSources(Vec<EventSeq>);

impl ExtraSources {
    pub(super) fn record(&mut self, source: EventSeq) {
        if let Err(index) = self.0.binary_search(&source) {
            self.0.insert(index, source);
        }
    }

    /// Exact recorded membership, without interpreting a gap as a contribution.
    #[must_use]
    pub fn contains(&self, source: EventSeq) -> bool {
        self.0.binary_search(&source).is_ok()
    }
}

/// Prefer an entry created by this exact record, otherwise one explicitly changed
/// by it. Multiple activity rows from one record use the first created row.
#[must_use]
pub fn entry_at_source(entries: &[Entry], source: EventSeq) -> Option<usize> {
    entries
        .iter()
        .position(|entry| entry.seq == source)
        .or_else(|| {
            entries
                .iter()
                .rposition(|entry| entry.extra_sources.contains(source))
        })
}
