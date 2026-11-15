//! A moved read-only projection reuses the normal rows, disclosures and scroll.
//! Source/query/parent retirement admission stays with the owning resident. This
//! module never reads, follows, submits, or creates a session identity.

use super::ConversationArea;
use std::collections::HashSet;
use vessel_app_view::timeline::{Entry, FindCache, Folds, Logs, Timeline, outline};

/// Refusal returns ownership; a caller must not lose an accepted page projection.
#[cfg(test)]
pub(crate) struct UnappliedProjection {
    /// Original projection, untouched when the receiver is a managed conversation.
    pub(crate) timeline: Box<Timeline>,
}

impl ConversationArea {
    /// Install a moved provider-child projection without a fabricated SessionId.
    /// The owner first checks its parent binding/read generation and UI holds; a
    /// later query must never target this receiver through an old keyed Seat.
    ///
    /// # Errors
    /// A managed identity/description refuses replacement and returns the projection.
    /// Staged until W2's first real caller; remove this cfg with that integration.
    #[cfg(test)]
    pub(crate) fn replace_readonly_projection(
        &mut self,
        timeline: Timeline,
    ) -> Result<(), UnappliedProjection> {
        if self.shown.is_some() || self.info.is_some() {
            return Err(UnappliedProjection {
                timeline: Box::new(timeline),
            });
        }
        self.install_projection(timeline);
        Ok(())
    }

    /// The existing fork-origin reader is the first production caller. A rebuilt
    /// prefix can shift every numeric row; choices and anchors follow stable keys.
    pub(super) fn install_projection(&mut self, timeline: Timeline) {
        let bookmark = self.reading_bookmark();
        let previous = self.timeline.entries();
        self.disclosures.rebase(previous, &timeline);
        rebase_open(&mut self.open_runs, previous, &timeline);
        if let Some(turns) = &mut self.turns {
            rebase_open(&mut turns.open, previous, &timeline);
        }
        // UI holds are checked by the source owner. Keep the generation counter
        // so an already queued dismissal cannot match a future sheet lifetime.
        self.tool_sheet.clear();
        self.timeline = timeline;
        self.folds = self
            .collapse
            .then(|| Folds::with_reasoning(self.timeline.entries(), self.reasoning.expanded()));
        self.logs = Logs::of(self.timeline.entries());
        self.outline = outline(self.timeline.entries());
        let timeline = &self.timeline;
        self.outline.retain(|item| !timeline.rewound(item.entry));
        self.find_cache = FindCache::default();
        self.fresh = None;
        self.rows_again();
        self.list.reset(self.at(self.rows.len()));
        self.restore_reading(bookmark);
    }
}

fn rebase_open(open: &mut HashSet<usize>, previous: &[Entry], next: &Timeline) {
    *open = std::mem::take(open)
        .into_iter()
        .filter_map(|index| next.index_of(&previous.get(index)?.key))
        .collect();
}

#[cfg(test)]
mod tests;
