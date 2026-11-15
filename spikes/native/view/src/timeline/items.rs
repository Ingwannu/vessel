//! Canonical item upserts use the existing append/change path and source bounds.

use super::{EntryKey, EntryKind, Timeline, merge_compaction};
use vessel_proto::EventRecord;
use vessel_proto::timeline::{Compaction, CompactionStatus, PluginTimelineItem};

impl Timeline {
    /// Every loading event is a distinct marker; completion upgrades the earliest
    /// outstanding non-rewound loading marker even across intervening rows.
    pub(super) fn compacted(&mut self, record: &EventRecord, item: Compaction) {
        if item.status == CompactionStatus::Completed {
            let loading = self.entries.iter().enumerate().find_map(|(index, entry)| {
                (!self.rewound(index) && matches!(&entry.kind, EntryKind::Compaction(previous) if previous.status == CompactionStatus::Loading))
                    .then_some(index)
            });
            if let Some(index) = loading
                && let Some(entry) = self.change(index)
                && let EntryKind::Compaction(previous) = &mut entry.kind
            {
                *previous = merge_compaction(previous, &item);
                return;
            }
        }
        self.push_keyed(
            record,
            EntryKey::Record(record.seq),
            EntryKind::Compaction(item),
        );
    }

    /// Contribution-local ids do not collide across plugins. The core stores the
    /// validated opaque value without interpreting payload data or version meaning.
    pub(super) fn plugin_item(&mut self, record: &EventRecord, item: PluginTimelineItem) {
        let identity = (item.plugin.clone(), item.id.clone());
        if let Some(index) = self.plugins.get(&identity).copied()
            && let Some(entry) = self.change(index)
        {
            entry.kind = EntryKind::Plugin(item);
            return;
        }
        let _previous = self.plugins.insert(identity, self.entries.len());
        self.push_keyed(
            record,
            EntryKey::Plugin {
                plugin: item.plugin.clone(),
                id: item.id.clone(),
            },
            EntryKind::Plugin(item),
        );
    }
}
