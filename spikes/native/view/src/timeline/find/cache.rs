//! Cached folding, separate from matching and UI scheduling.

use super::{Entry, EntryKind, Match, fold, occurrences, readable_text, searchable};

#[cfg(test)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Work {
    visits: usize,
    folds: usize,
}

/// Each entry's folded readable text, kept by index and revision: a
/// second search folds only the entries that changed since the first.
#[derive(Debug, Default)]
pub struct FindCache {
    folded: Vec<Option<(u32, Option<String>)>>,
    #[cfg(test)]
    work: Work,
}

impl FindCache {
    /// Entry `index`'s folded text (`None`: not searched).
    fn folded(&mut self, index: usize, entry: &Entry) -> Option<&str> {
        if self.folded.len() <= index {
            self.folded.resize(index.saturating_add(1), None);
        }
        let slot = self.folded.get_mut(index)?;
        let fresh = slot.as_ref().is_none_or(|(rev, _)| *rev != entry.rev);
        if fresh {
            #[cfg(test)]
            {
                self.work.folds = self.work.folds.saturating_add(1);
            }
            let text = searchable(entry).map(|text| match &entry.kind {
                EntryKind::Message { .. } => fold(&readable_text(text)),
                EntryKind::Ready { .. }
                | EntryKind::Prompt(_)
                | EntryKind::Steer { .. }
                | EntryKind::Question { .. }
                | EntryKind::Thought { .. }
                | EntryKind::Tool(_)
                | EntryKind::Plan { .. }
                | EntryKind::Permission(_)
                | EntryKind::TurnEnded { .. }
                | EntryKind::TurnFailed { .. }
                | EntryKind::Log { .. }
                | EntryKind::Status { .. }
                | EntryKind::Parking(_)
                | EntryKind::Forked { .. }
                | EntryKind::Profile { .. }
                | EntryKind::Model(_) => fold(text),
            });
            *slot = Some((entry.rev, text));
        }
        slot.as_ref().and_then(|(_, text)| text.as_deref())
    }

    pub(super) fn collect(
        &mut self,
        index: usize,
        entry: &Entry,
        needle: &str,
        into: &mut Vec<Match>,
    ) {
        #[cfg(test)]
        {
            self.work.visits = self.work.visits.saturating_add(1);
        }
        let Some(text) = self.folded(index, entry) else {
            return;
        };
        let count = occurrences(text, needle);
        for nth in 0..count {
            into.push(Match {
                entry: index,
                nth: u32::try_from(nth).unwrap_or(u32::MAX),
            });
        }
    }
}

#[cfg(test)]
mod tests;
