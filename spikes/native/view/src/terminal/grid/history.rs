//! The lines that scrolled off the top of the primary screen.
//!
//! [Decision Log]
//! - Purpose: scrollback the person can read back, as Paseo keeps it
//!   (`terminalScrollbackLines`, default 10,000, up to 1,000,000).
//! - Constraint: a program writes as many lines as it likes, as wide as
//!   the grid; at the setting's maximum and the grid's widest that is a
//!   billion cells, far past any memory the app may take.
//! - Choice: a line that ended hard keeps no trailing unprinted cells (a
//!   soft-wrapped one keeps them for reflow, and is at most as wide as the
//!   grid), and history is bounded twice: by the setting's line count and
//!   by [`CELL_LIMIT`] cells in all. Whichever bound is reached first drops
//!   the oldest line. A cell is 20 bytes (a glyph, two colours and six
//!   attributes; a test pins it), so cells are at most 80 MiB; each line
//!   also costs a 32-byte header, about 32 MiB more at the 1,000,000-line
//!   setting, so about 112 MiB at most.
//! - Alternative: lines at the grid's full width (xterm.js), or the line
//!   count alone; either lets a program fill memory with wide lines.
//! - Cost: with a high setting and long lines the cell bound comes first,
//!   so fewer lines are kept than the setting names. At the default
//!   (10,000 lines) a line would have to average over 419 printed cells
//!   for that to happen.

use std::collections::VecDeque;

use super::{Generation, Scroll, cell::Line};
use crate::terminal::size::ScrollbackLines;

/// The most cells history holds, across all its lines.
pub const CELL_LIMIT: usize = 4 * 1024 * 1024;

/// A physical history row's address survives front eviction. A clear
/// changes the epoch so an old anchor cannot attach to unrelated output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct HistoryRow {
    epoch: Generation,
    line: u64,
}

impl HistoryRow {
    pub(crate) fn after(self, rows: Scroll) -> Self {
        Self {
            line: self.line.saturating_add(u64::from(rows.0)),
            ..self
        }
    }

    pub(crate) fn before(self, rows: Scroll) -> Self {
        Self {
            line: self.line.saturating_sub(u64::from(rows.0)),
            ..self
        }
    }

    pub(crate) fn distance(self, earlier: Self) -> Scroll {
        Scroll(u32::try_from(self.line.saturating_sub(earlier.line)).unwrap_or(u32::MAX))
    }

    pub(crate) fn within(self, first: Self, end: Self) -> Option<Self> {
        (self.epoch == first.epoch).then(|| Self {
            line: self.line.clamp(first.line, end.line),
            ..self
        })
    }
}

/// History, oldest first.
#[derive(Debug)]
pub(super) struct History {
    lines: VecDeque<Line>,
    /// Cells held, the sum of every line's width.
    cells: usize,
    limit: ScrollbackLines,
    first: HistoryRow,
}

impl History {
    pub(super) const fn new(limit: ScrollbackLines) -> Self {
        Self {
            lines: VecDeque::new(),
            cells: 0,
            limit,
            first: HistoryRow {
                epoch: Generation(0),
                line: 0,
            },
        }
    }

    /// The setting's bound.
    pub(super) const fn limit(&self) -> ScrollbackLines {
        self.limit
    }

    /// Lines held.
    pub(super) fn len(&self) -> usize {
        self.lines.len()
    }

    pub(super) const fn first(&self) -> HistoryRow {
        self.first
    }

    /// Line `index` from the oldest.
    pub(super) fn line(&self, index: usize) -> Option<&Line> {
        self.lines.get(index)
    }

    /// Keep `line`, newest, then drop the oldest past either bound.
    pub(super) fn push(&mut self, line: Line) {
        let line = line.trimmed();
        self.cells = self.cells.saturating_add(line.cells().len());
        self.lines.push_back(line);
        self.bound();
    }

    /// The newest line, taken back (a screen that grew taller).
    pub(super) fn pop_newest(&mut self) -> Option<Line> {
        let line = self.lines.pop_back()?;
        self.cells = self.cells.saturating_sub(line.cells().len());
        Some(line)
    }

    /// Every line, oldest first, taken (a reflow rebuilds them).
    pub(super) fn take_all(&mut self) -> Vec<Line> {
        self.cells = 0;
        std::mem::take(&mut self.lines).into()
    }

    /// Forget every line (`CSI 3 J`, a reset).
    pub(super) fn clear(&mut self) {
        self.lines.clear();
        self.lines.shrink_to_fit();
        self.cells = 0;
        self.reset_after(self.first);
    }

    /// A new buffer must not reuse an older buffer's row addresses.
    pub(super) fn reset_after(&mut self, previous: HistoryRow) {
        self.first = HistoryRow {
            epoch: previous.epoch.next(),
            line: 0,
        };
    }

    fn bound(&mut self) {
        let limit = usize::try_from(self.limit.get()).unwrap_or(usize::MAX);
        while self.lines.len() > limit || self.cells > CELL_LIMIT {
            let Some(oldest) = self.lines.pop_front() else {
                break;
            };
            self.cells = self.cells.saturating_sub(oldest.cells().len());
            self.first = self.first.after(Scroll(1));
        }
    }

    #[cfg(test)]
    pub(super) const fn cells(&self) -> usize {
        self.cells
    }
}
