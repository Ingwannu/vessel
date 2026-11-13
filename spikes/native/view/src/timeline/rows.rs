//! What a row of the timeline's window is, once "Collapse tool calls"
//! has folded some entries away (the page's `data/rows.ts`).
//!
//! A row is an entry that is drawn: every entry, less the members of a
//! closed run other than its lead, which draws the run's summary (and
//! itself, once the run is opened). The window counts rows; find and the
//! outline name entries; `row_index` turns an entry into the row that
//! shows it (a folded member's is its run's lead).
//!
//! An agent's log lines (`logs`) are never rows of their own: a run of
//! them is one row at its first line while tool calls are not collapsed,
//! and none at all while they are.

use std::collections::HashSet;

use super::logs::Logs;
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
    /// collapsing), runs whose lead is in `open` shown whole; `logs`'
    /// runs one row each when not collapsing, none when collapsing.
    #[must_use]
    pub fn of(count: usize, folds: Option<&Folds>, logs: &Logs, open: &HashSet<usize>) -> Self {
        let Some(folds) = folds else {
            if logs.is_empty() {
                return Self::All(count);
            }
            return Self::Some(
                (0..count)
                    .filter(|index| logs.run_of(*index).is_none_or(|run| run.lead == *index))
                    .collect(),
            );
        };
        Self::Some(
            (0..count)
                .filter(|index| {
                    logs.run_of(*index).is_none()
                        && folds
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
    /// away, its run's lead's (a tool run's or a log run's); `None` when
    /// it is not in the log or not shown at all (a log line while tool
    /// calls are collapsed).
    #[must_use]
    pub fn row_index(&self, index: usize, folds: Option<&Folds>, logs: &Logs) -> Option<usize> {
        match self {
            Self::All(count) => (index < *count).then_some(index),
            Self::Some(rows) => rows.binary_search(&index).ok().or_else(|| {
                let lead = folds
                    .and_then(|folds| folds.run_of(index))
                    .map(|run| run.lead)
                    .or_else(|| logs.run_of(index).map(|run| run.lead))?;
                rows.binary_search(&lead).ok()
            }),
        }
    }
}
