//! Borrowed rail projection: metadata can navigate before transcript bodies arrive.
use super::PromptLocation;
use crate::timeline::{OutlineItem, Timeline};
use vessel_proto::EventSeq;

/// One tick's stable address and bounded preview, without cloning the whole index.
#[derive(Debug, Clone, Copy)]
pub struct PromptRow<'a> {
    /// First retained record of this prompt.
    pub seq: EventSeq,
    /// Text already prepared by its authoritative source.
    pub preview: &'a str,
}

/// A ready server index, or the complete followed log's compatibility projection.
#[derive(Debug, Clone, Copy)]
pub struct PromptRows<'a> {
    index: Option<&'a [PromptLocation]>,
    timeline: &'a Timeline,
    followed: &'a [OutlineItem],
}

impl<'a> PromptRows<'a> {
    /// Prefer a complete metadata snapshot; incomplete sets are supplied as None.
    #[must_use]
    pub const fn new(
        index: Option<&'a [PromptLocation]>,
        timeline: &'a Timeline,
        followed: &'a [OutlineItem],
    ) -> Self {
        Self {
            index,
            timeline,
            followed,
        }
    }

    /// Every prompt in the chosen source, including unloaded metadata addresses.
    #[must_use]
    pub fn len(self) -> usize {
        self.index
            .map_or(self.followed.len(), <[PromptLocation]>::len)
    }

    /// Whether the chosen source has no prompts.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Read a tick by its slot, borrowing the preview.
    #[must_use]
    pub fn get(self, index: usize) -> Option<PromptRow<'a>> {
        if let Some(entries) = self.index {
            let item = entries.get(index)?;
            return Some(PromptRow {
                seq: item.seq,
                preview: item.preview.as_str(),
            });
        }
        let item = self.followed.get(index)?;
        Some(PromptRow {
            seq: self.timeline.entries().get(item.entry)?.seq,
            preview: &item.prompt,
        })
    }

    /// The last prompt at or before the reading position, never a later prompt.
    #[must_use]
    pub fn active(self, seq: EventSeq) -> Option<usize> {
        let count = if let Some(entries) = self.index {
            entries.partition_point(|entry| entry.seq <= seq)
        } else {
            self.followed.partition_point(|item| {
                self.timeline
                    .entries()
                    .get(item.entry)
                    .is_some_and(|entry| entry.seq <= seq)
            })
        };
        count.checked_sub(1)
    }

    /// Stable slot order with no transcript copies.
    pub fn iter(self) -> impl Iterator<Item = (usize, PromptRow<'a>)> {
        (0..self.len()).filter_map(move |index| self.get(index).map(|row| (index, row)))
    }
}
