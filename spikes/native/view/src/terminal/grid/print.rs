//! Printing: a scalar placed at the cursor in one cell, two (a wide
//! scalar and its continuation) or none (a mark composed into the cell
//! before the cursor). See `width` for the rules and their reasons.

use super::Grid;
use super::cell::{Cell, Glyph, Wrap};
use super::modes::{AutoWrap, Insert, Pending};
use super::width::{Width, compose, width};

impl Grid {
    /// Print `c` at the cursor with the pen.
    pub(super) fn print(&mut self, c: char) {
        match width(c) {
            Width::Zero => self.combine(c),
            Width::One => self.place(c, 1),
            Width::Two => self.place(c, 2),
        }
    }

    /// `c`, `cells` wide, at the cursor: wrapping first when a wrap is
    /// pending or a wide scalar does not fit before the edge (with
    /// auto-wrap off, or on a one-column grid, that scalar is dropped, as
    /// xterm.js drops it).
    fn place(&mut self, c: char, cells: u16) {
        let last = self.size.last_col();
        let pending = self.at.pending == Pending::Wrap;
        let past_edge = self.at.col.saturating_add(cells).saturating_sub(1) > last;
        self.at.pending = Pending::Nothing;
        if pending || past_edge {
            match self.modes.auto_wrap {
                AutoWrap::Wrap if cells <= self.size.cols() => self.wrap(),
                AutoWrap::Wrap | AutoWrap::Stay if past_edge => return,
                AutoWrap::Wrap | AutoWrap::Stay => {}
            }
        }
        let (row, col, blank) = (self.at.row, self.at.col, self.blank());
        if self.modes.insert == Insert::Insert {
            self.screen_mut().insert_cells(row, col, cells, blank);
        }
        let pen = self.pen;
        if let Some(line) = self.screen_mut().line_mut(row) {
            let start = usize::from(col);
            let end = start.saturating_add(usize::from(cells));
            if let Some(span) = line.cells_mut().get_mut(start..end) {
                for (at, cell) in span.iter_mut().enumerate() {
                    *cell = if at == 0 {
                        Cell::printed(c, pen)
                    } else {
                        Cell::continuation(pen)
                    };
                }
            }
            line.fix_pairs(start, end, blank);
        }
        let end = col.saturating_add(cells).saturating_sub(1);
        if end < last {
            self.at.col = end.saturating_add(1);
        } else {
            self.at.col = last;
            if self.modes.auto_wrap == AutoWrap::Wrap {
                self.at.pending = Pending::Wrap;
            }
        }
    }

    /// To the start of the next line, the line left marked soft-wrapped.
    fn wrap(&mut self) {
        let row = self.at.row;
        if let Some(line) = self.screen_mut().line_mut(row) {
            line.set_wrap(Wrap::Soft);
        }
        self.at.col = 0;
        self.index();
    }

    /// A zero-width `mark` composed into the cell before the cursor (the
    /// cursor's own cell while a wrap is pending), its continuation's
    /// scalar for a wide one. At column 0, or with nothing to compose
    /// with, it is dropped.
    fn combine(&mut self, mark: char) {
        let row = self.at.row;
        let col = match self.at.pending {
            Pending::Wrap => Some(self.at.col),
            Pending::Nothing => self.at.col.checked_sub(1),
        };
        let Some(col) = col else {
            return;
        };
        let Some(line) = self.screen_mut().line_mut(row) else {
            return;
        };
        let col = match line.cell_mut(col).map(|cell| cell.glyph()) {
            Some(Glyph::Continuation) => col.saturating_sub(1),
            Some(Glyph::Scalar(_) | Glyph::Empty) | None => col,
        };
        if let Some(cell) = line.cell_mut(col)
            && let Glyph::Scalar(base) = cell.glyph()
            && let Some(composed) = compose(base, mark)
        {
            *cell = Cell::printed(composed, cell.pen());
        }
    }
}
