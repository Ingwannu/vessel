//! Tab stops: every eighth column at first; HTS sets one, TBC clears one
//! or all. At most one stop per column, so the set is bounded by the
//! grid's width.

use std::collections::BTreeSet;

/// The columns that hold a stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TabStops {
    stops: BTreeSet<u16>,
}

/// The default spacing.
const EVERY: u16 = 8;

impl TabStops {
    /// Every eighth column of `cols`.
    pub(super) fn new(cols: u16) -> Self {
        let mut tabs = Self {
            stops: BTreeSet::new(),
        };
        tabs.fill(0, cols);
        tabs
    }

    /// The first stop after `col`, or `last` (the right margin).
    pub(super) fn next(&self, col: u16, last: u16) -> u16 {
        let from = col.saturating_add(1);
        // `BTreeSet::range` panics on a range that starts past its end.
        if from > last {
            return last;
        }
        self.stops
            .range(from..=last)
            .next()
            .copied()
            .unwrap_or(last)
    }

    /// The last stop before `col`, or column 0.
    pub(super) fn previous(&self, col: u16) -> u16 {
        self.stops.range(..col).next_back().copied().unwrap_or(0)
    }

    /// A stop at `col`.
    pub(super) fn set(&mut self, col: u16) {
        let _: bool = self.stops.insert(col);
    }

    /// No stop at `col`.
    pub(super) fn clear(&mut self, col: u16) {
        let _: bool = self.stops.remove(&col);
    }

    /// No stop anywhere.
    pub(super) fn clear_all(&mut self) {
        self.stops.clear();
    }

    /// A grid now `cols` wide (it was `old`): stops past the edge go, and
    /// new columns get the default stops (xterm's rule).
    pub(super) fn resize(&mut self, old: u16, cols: u16) {
        self.stops.retain(|stop| *stop < cols);
        self.fill(old, cols);
    }

    /// Default stops in `from..cols`.
    fn fill(&mut self, from: u16, cols: u16) {
        for col in (EVERY..cols).step_by(usize::from(EVERY)) {
            if col >= from {
                let _: bool = self.stops.insert(col);
            }
        }
    }
}
