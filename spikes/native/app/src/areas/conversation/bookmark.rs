//! A visible entry owns reading position across display/default changes.
//! The kit stores only row/offset/follow state; timeline identity rebases its row.

use super::ConversationArea;
use vessel_app_kit::ListBookmark;
use vessel_app_view::timeline::EntryKey;

pub(super) struct ReadingBookmark {
    list: ListBookmark,
    entry: Option<ReadingEntry>,
}

enum ReadingEntry {
    Session(EntryKey),
    Origin(EntryKey),
}

impl ConversationArea {
    pub(super) fn reading_bookmark(&self) -> ReadingBookmark {
        let list = self.list.bookmark();
        let row = list.row();
        let entry = if let Some(row) = row.checked_sub(self.head()) {
            self.rows
                .entry(row)
                .and_then(|index| self.timeline.entries().get(index))
                .map(|entry| ReadingEntry::Session(entry.key.clone()))
        } else {
            self.origin
                .area()
                .and_then(|origin| {
                    origin
                        .rows
                        .entry(row)
                        .and_then(|index| origin.timeline.entries().get(index))
                })
                .map(|entry| ReadingEntry::Origin(entry.key.clone()))
        };
        ReadingBookmark { list, entry }
    }

    pub(super) fn restore_reading(&self, bookmark: ReadingBookmark) {
        let row = match bookmark.entry {
            Some(ReadingEntry::Session(key)) => self.visible_row(&key).map(|row| self.at(row)),
            Some(ReadingEntry::Origin(key)) => self
                .origin
                .area()
                .and_then(|origin| origin.visible_row(&key)),
            None => None,
        };
        self.list
            .restore(row.map_or(bookmark.list, |row| bookmark.list.at_row(row)));
    }

    fn visible_row(&self, key: &EntryKey) -> Option<usize> {
        let entry = self
            .timeline
            .entries()
            .iter()
            .position(|entry| &entry.key == key)?;
        self.rows
            .row_index(entry, self.folds.as_ref(), &self.logs)
            .or_else(|| {
                let host = self.turns.as_ref()?.folds.containing(entry)?.host;
                self.rows.row_index(host, self.folds.as_ref(), &self.logs)
            })
    }
}
