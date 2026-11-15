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

use super::cell::Line;
use crate::terminal::size::ScrollbackLines;

/// The most cells history holds, across all its lines.
pub const CELL_LIMIT: usize = 4 * 1024 * 1024;

/// History, oldest first.
#[derive(Debug)]
pub(super) struct History {
    lines: VecDeque<Line>,
    /// Cells held, the sum of every line's width.
    cells: usize,
    limit: ScrollbackLines,
}

impl History {
    pub(super) const fn new(limit: ScrollbackLines) -> Self {
        Self {
            lines: VecDeque::new(),
            cells: 0,
            limit,
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
    }

    fn bound(&mut self) {
        let limit = usize::try_from(self.limit.get()).unwrap_or(usize::MAX);
        while self.lines.len() > limit || self.cells > CELL_LIMIT {
            let Some(oldest) = self.lines.pop_front() else {
                break;
            };
            self.cells = self.cells.saturating_sub(oldest.cells().len());
        }
    }

    #[cfg(test)]
    pub(super) const fn cells(&self) -> usize {
        self.cells
    }
}
