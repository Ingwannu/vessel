//! What a row of the timeline's window is, once "Collapse tool calls"
//! has folded some entries away (the page's `data/rows.ts`).
//!
//! A row is an entry that is drawn: every entry, less the members of a
//! closed run other than its lead, which draws the run's summary (and
//! itself, once the run is opened). The window counts rows; find and the
//! outline name entries; `row_index` turns an entry into the row that
//! shows it (a folded member's is its run's lead).

use std::collections::HashSet;

use super::runs::Folds;

/// The entries drawn as the window's rows, by entry index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rows {
    /// Every entry is a row (nothing folded): `count` of them.
    All(usize),
    /// Only these entries, in order.
    Some(Vec<usize>),
}

impl Rows {
    /// The rows of `count` entries folded by `folds` (`None`: not
    /// collapsing), runs whose lead is in `open` shown whole.
    #[must_use]
    pub fn of(count: usize, folds: Option<&Folds>, open: &HashSet<usize>) -> Self {
        let Some(folds) = folds else {
            return Self::All(count);
        };
        Self::Some(
            (0..count)
                .filter(|index| {
                    folds
                        .run_of(*index)
                        .is_none_or(|run| run.lead == *index || open.contains(&run.lead))
                })
                .collect(),
        )
    }

    /// How many rows there are.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::All(count) => *count,
            Self::Some(rows) => rows.len(),
        }
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The entry row `row` draws.
    #[must_use]
    pub fn entry(&self, row: usize) -> Option<usize> {
        match self {
            Self::All(count) => (row < *count).then_some(row),
            Self::Some(rows) => rows.get(row).copied(),
        }
    }

    /// Which row shows entry `index`: its own, or for an entry folded
    /// away, its run's lead's; `None` when it is not in the log.
    #[must_use]
    pub fn row_index(&self, index: usize, folds: Option<&Folds>) -> Option<usize> {
        match self {
            Self::All(count) => (index < *count).then_some(index),
            Self::Some(rows) => rows.binary_search(&index).ok().or_else(|| {
                let lead = folds?.run_of(index)?.lead;
                rows.binary_search(&lead).ok()
            }),
        }
    }
}
