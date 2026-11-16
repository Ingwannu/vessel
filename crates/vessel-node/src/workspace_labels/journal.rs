//! Commit-ordered catalog projection and bounded catch-up history.
//! This owns no storage; the service calls committed only after the writer replies.

use std::collections::{BTreeMap, VecDeque};

use vessel_proto::workspace_labels::{
    CatalogCursor, CatalogDelta, CatalogList, CatalogMode, CatalogUpdate, Label, LabelKey,
};
use vessel_proto::{WorkspaceLabelGeneration, WorkspaceLabelSeq};

/// Reference default. Bounds retained updates, not catalog cardinality.
pub(super) const JOURNAL_ENTRIES: usize = 256;
const FIRST_UPDATE: WorkspaceLabelSeq = WorkspaceLabelSeq::new(1);

pub(super) struct Journal {
    generation: WorkspaceLabelGeneration,
    head: WorkspaceLabelSeq,
    catalog: BTreeMap<LabelKey, Label>,
    retained: VecDeque<CatalogUpdate>,
}

impl Journal {
    #[must_use]
    pub(super) fn new(labels: Vec<Label>) -> Self {
        Self {
            generation: WorkspaceLabelGeneration::random(),
            head: WorkspaceLabelSeq::ZERO,
            catalog: labels
                .into_iter()
                .map(|label| (label.name.key(), label))
                .collect(),
            retained: VecDeque::new(),
        }
    }

    #[must_use]
    pub(super) const fn cursor(&self) -> CatalogCursor {
        CatalogCursor {
            generation: self.generation,
            after: self.head,
        }
    }

    pub(super) fn committed(&mut self, delta: CatalogDelta) -> CatalogUpdate {
        // A durable mutation cannot be rolled back because RAM ran out of
        // sequence ids. Rotate generation and force old cursors to snapshot.
        let next = if let Some(next) = self.head.next() {
            next
        } else {
            self.generation = WorkspaceLabelGeneration::random();
            self.retained.clear();
            FIRST_UPDATE
        };
        match &delta {
            CatalogDelta::Upsert { label, previous } => {
                if let Some(previous) = previous {
                    let _previous_definition = self.catalog.remove(&previous.key());
                }
                let _replaced_definition = self.catalog.insert(label.name.key(), label.clone());
            }
            CatalogDelta::Remove { name } => {
                let _removed_definition = self.catalog.remove(&name.key());
            }
        }
        self.head = next;
        let update = CatalogUpdate {
            cursor: self.cursor(),
            delta,
        };
        self.retained.push_back(update.clone());
        if self.retained.len() > JOURNAL_ENTRIES {
            let _expired_update = self.retained.pop_front();
        }
        update
    }

    #[must_use]
    pub(super) fn synchronize(&self, cursor: Option<CatalogCursor>) -> CatalogList {
        let cursor = cursor.filter(|cursor| self.retains(*cursor));
        let data = match cursor {
            Some(from) => CatalogMode::Changes {
                from,
                updates: self
                    .retained
                    .iter()
                    .filter(|update| update.cursor.after > from.after)
                    .cloned()
                    .collect(),
            },
            None => CatalogMode::Snapshot {
                labels: self.catalog.values().cloned().collect(),
            },
        };
        CatalogList {
            data,
            head: self.cursor(),
        }
    }

    fn retains(&self, cursor: CatalogCursor) -> bool {
        if cursor.generation != self.generation || cursor.after > self.head {
            return false;
        }
        if cursor.after == self.head {
            return true;
        }
        match self.retained.front() {
            Some(first) => cursor
                .after
                .next()
                .is_some_and(|next| next >= first.cursor.after),
            None => cursor.after == self.head,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::workspace_labels::LabelColor;

    fn label(name: &str, color: LabelColor) -> Label {
        Label {
            name: name.parse().unwrap(),
            color,
        }
    }

    #[test]
    fn initial_or_foreign_or_future_cursor_gets_authoritative_snapshot() {
        let journal = Journal::new(vec![label("Ready", LabelColor::Violet)]);
        let snapshot = journal.synchronize(None);
        assert!(matches!(snapshot.data, CatalogMode::Snapshot { labels } if labels.len() == 1));
        for cursor in [
            CatalogCursor {
                generation: WorkspaceLabelGeneration::random(),
                after: WorkspaceLabelSeq::ZERO,
            },
            CatalogCursor {
                after: WorkspaceLabelSeq::new(1),
                ..journal.cursor()
            },
        ] {
            assert!(matches!(
                journal.synchronize(Some(cursor)).data,
                CatalogMode::Snapshot { .. }
            ));
        }
        assert!(matches!(journal.synchronize(Some(journal.cursor())).data,
            CatalogMode::Changes { updates, .. } if updates.is_empty()));
    }

    #[test]
    fn rename_recolor_remove_are_contiguous_and_snapshot_has_only_committed_names() {
        let mut journal = Journal::new(vec![label("Ready", LabelColor::Violet)]);
        let from = journal.cursor();
        let renamed = journal.committed(CatalogDelta::Upsert {
            label: label("Waiting", LabelColor::Sky),
            previous: Some("Ready".parse().unwrap()),
        });
        let recolored = journal.committed(CatalogDelta::Upsert {
            label: label("Waiting", LabelColor::Red),
            previous: None,
        });
        assert_eq!(renamed.cursor.after.next(), Some(recolored.cursor.after));
        let CatalogMode::Changes { updates, .. } = journal.synchronize(Some(from)).data else {
            panic!("expected retained changes");
        };
        assert_eq!(updates, vec![renamed, recolored]);
        assert_eq!(
            journal.synchronize(None).data,
            CatalogMode::Snapshot {
                labels: vec![label("Waiting", LabelColor::Red)]
            }
        );
        let _removed = journal.committed(CatalogDelta::Remove {
            name: "waiting".parse().unwrap(),
        });
        assert_eq!(
            journal.synchronize(None).data,
            CatalogMode::Snapshot { labels: Vec::new() }
        );
    }

    #[test]
    fn retained_boundary_is_exact_and_expired_cursor_refetches_instead_of_skipping() {
        let mut journal = Journal::new(Vec::new());
        let initial = journal.cursor();
        for index in 0..JOURNAL_ENTRIES {
            let _appended = journal.committed(CatalogDelta::Upsert {
                label: label(&format!("Label {index}"), LabelColor::Teal),
                previous: None,
            });
        }
        assert!(
            matches!(journal.synchronize(Some(initial)).data, CatalogMode::Changes { updates, .. } if updates.len() == JOURNAL_ENTRIES)
        );
        let earliest_retained = journal.retained.front().unwrap().cursor;
        let _evicted = journal.committed(CatalogDelta::Remove {
            name: "Label 0".parse().unwrap(),
        });
        assert_eq!(journal.retained.len(), JOURNAL_ENTRIES);
        assert!(matches!(
            journal.synchronize(Some(initial)).data,
            CatalogMode::Snapshot { .. }
        ));
        assert!(
            matches!(journal.synchronize(Some(earliest_retained)).data, CatalogMode::Changes { updates, .. } if updates.len() == JOURNAL_ENTRIES)
        );
    }

    #[test]
    fn sequence_exhaustion_rotates_generation_without_losing_committed_state() {
        let mut journal = Journal::new(Vec::new());
        journal.head = WorkspaceLabelSeq::new(u64::MAX);
        let before = journal.cursor();
        let update = journal.committed(CatalogDelta::Upsert {
            label: label("Ready", LabelColor::Blue),
            previous: None,
        });
        assert_ne!(before.generation, update.cursor.generation);
        assert_eq!(update.cursor.after, FIRST_UPDATE);
        assert_eq!(
            journal.synchronize(Some(before)).data,
            CatalogMode::Snapshot {
                labels: vec![label("Ready", LabelColor::Blue)]
            }
        );
    }
}
