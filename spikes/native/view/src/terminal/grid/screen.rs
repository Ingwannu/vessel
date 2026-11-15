//! One screen (primary or alternate): its rows of cells and its scroll
//! region. Every edit is bounded by the screen and clamps its counts, so
//! a count from a program (up to 65,535) never reaches an index.

use std::ops::Range;

use super::cell::{Cell, Line, Wrap};
use crate::terminal::size::GridSize;

/// The scroll region: rows `top` through `bottom`, 0-based, inclusive,
/// `top < bottom` (DECSTBM refuses anything else).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Region {
    pub(super) top: u16,
    pub(super) bottom: u16,
}

impl Region {
    /// The whole screen.
    pub(super) const fn whole(size: GridSize) -> Self {
        Self {
            top: 0,
            bottom: size.last_row(),
        }
    }

    pub(super) const fn holds(self, row: u16) -> bool {
        row >= self.top && row <= self.bottom
    }
}

/// A screen's rows, each exactly as wide as the grid.
#[derive(Debug, Clone)]
pub(super) struct Screen {
    lines: Vec<Line>,
    pub(super) region: Region,
}

impl Screen {
    pub(super) fn new(size: GridSize) -> Self {
        Self {
            lines: vec![Line::blank(size.cols(), Cell::BLANK); usize::from(size.rows())],
            region: Region::whole(size),
        }
    }

    pub(super) fn lines(&self) -> &[Line] {
        &self.lines
    }

    pub(super) fn line_mut(&mut self, row: u16) -> Option<&mut Line> {
        self.lines.get_mut(usize::from(row))
    }

    /// Every row blank, the region whole.
    pub(super) fn clear(&mut self, size: GridSize, blank: Cell) {
        for line in &mut self.lines {
            *line = Line::blank(size.cols(), blank);
        }
        self.region = Region::whole(size);
    }

    /// Cells `cols` of `row` blank.
    pub(super) fn erase_cells(&mut self, row: u16, cols: Range<u16>, blank: Cell) {
        let Some(line) = self.line_mut(row) else {
            return;
        };
        let cells = line.cells_mut();
        let end = usize::from(cols.end).min(cells.len());
        let start = usize::from(cols.start).min(end);
        if let Some(span) = cells.get_mut(start..end) {
            span.fill(blank);
        }
        line.fix_pairs(start, end, blank);
    }

    /// Rows `rows` blank, each ending hard.
    pub(super) fn erase_rows(&mut self, rows: Range<u16>, blank: Cell) {
        let end = usize::from(rows.end).min(self.lines.len());
        let start = usize::from(rows.start).min(end);
        if let Some(span) = self.lines.get_mut(start..end) {
            for line in span {
                line.cells_mut().fill(blank);
                line.set_wrap(Wrap::Hard);
            }
        }
    }

    /// `n` blank cells at `col` of `row`; cells pushed past the edge are
    /// lost.
    pub(super) fn insert_cells(&mut self, row: u16, col: u16, n: u16, blank: Cell) {
        let Some(tail) = self.tail(row, col) else {
            return;
        };
        let n = usize::from(n).min(tail.len());
        tail.rotate_right(n);
        if let Some(gap) = tail.get_mut(..n) {
            gap.fill(blank);
        }
        self.fix_line(row, blank);
    }

    /// `n` cells at `col` of `row` removed; blanks fill in at the edge.
    pub(super) fn delete_cells(&mut self, row: u16, col: u16, n: u16, blank: Cell) {
        let Some(tail) = self.tail(row, col) else {
            return;
        };
        let n = usize::from(n).min(tail.len());
        tail.rotate_left(n);
        let keep = tail.len().saturating_sub(n);
        if let Some(gap) = tail.get_mut(keep..) {
            gap.fill(blank);
        }
        self.fix_line(row, blank);
    }

    /// Every half pair on `row` blanked (after a shift, which can split a
    /// pair at the cursor and at the edge).
    fn fix_line(&mut self, row: u16, blank: Cell) {
        if let Some(line) = self.line_mut(row) {
            let width = line.cells().len();
            line.fix_pairs(0, width, blank);
        }
    }

    /// Row `row` from `col` to the edge.
    fn tail(&mut self, row: u16, col: u16) -> Option<&mut [Cell]> {
        self.line_mut(row)?.cells_mut().get_mut(usize::from(col)..)
    }

    /// Scroll rows `top..=region.bottom` up by `n`: the top `n` leave
    /// (returned, oldest first) and blank rows come in at the bottom.
    pub(super) fn scroll_up(&mut self, top: u16, n: u16, blank: Cell) -> Vec<Line> {
        let cols = self.width();
        let Some(span) = self.span(top) else {
            return Vec::new();
        };
        let n = usize::from(n).min(span.len());
        span.rotate_left(n);
        let keep = span.len().saturating_sub(n);
        span.get_mut(keep..)
            .unwrap_or_default()
            .iter_mut()
            .map(|line| std::mem::replace(line, Line::blank(cols, blank)))
            .collect()
    }

    /// Scroll rows `top..=region.bottom` down by `n`: blank rows come in
    /// at `top` and the bottom `n` are lost.
    pub(super) fn scroll_down(&mut self, top: u16, n: u16, blank: Cell) {
        let cols = self.width();
        let Some(span) = self.span(top) else {
            return;
        };
        let n = usize::from(n).min(span.len());
        span.rotate_right(n);
        for line in span.get_mut(..n).unwrap_or_default() {
            *line = Line::blank(cols, blank);
        }
    }

    /// Rows `top` through the region's bottom, when the region holds `top`.
    fn span(&mut self, top: u16) -> Option<&mut [Line]> {
        let region = self.region;
        if !region.holds(top) {
            return None;
        }
        let end = usize::from(region.bottom).saturating_add(1);
        self.lines.get_mut(usize::from(top)..end)
    }

    /// The grid's width, from the first row (every row is as wide).
    fn width(&self) -> u16 {
        self.lines
            .first()
            .and_then(|line| u16::try_from(line.cells().len()).ok())
            .unwrap_or(0)
    }

    /// Every row cut or padded to `cols`.
    pub(super) fn fit_cols(&mut self, cols: u16, blank: Cell) {
        for line in &mut self.lines {
            line.fit(cols, blank);
        }
    }

    /// The top row taken off (a screen that got shorter).
    pub(super) fn take_top(&mut self) -> Option<Line> {
        (!self.lines.is_empty()).then(|| self.lines.remove(0))
    }

    /// The bottom row taken off, if it is blank (a shorter screen keeps
    /// what is printed).
    pub(super) fn take_blank_bottom(&mut self) -> bool {
        let blank = self
            .lines
            .last()
            .is_some_and(|line| line.cells().iter().all(|cell| *cell == Cell::BLANK));
        if blank {
            drop(self.lines.pop());
        }
        blank
    }

    /// `line` as the new top row (a taller screen takes history back).
    pub(super) fn push_top(&mut self, line: Line) {
        self.lines.insert(0, line);
    }

    /// A blank row at the bottom.
    pub(super) fn push_bottom(&mut self, cols: u16, blank: Cell) {
        self.lines.push(Line::blank(cols, blank));
    }

    /// Every row, taken (a reflow rebuilds them).
    pub(super) fn take_lines(&mut self) -> Vec<Line> {
        std::mem::take(&mut self.lines)
    }

    /// `lines` as the rows; the caller gives exactly the grid's rows.
    pub(super) fn set_lines(&mut self, lines: Vec<Line>) {
        self.lines = lines;
    }

    /// Rows held.
    pub(super) fn rows(&self) -> usize {
        self.lines.len()
    }
}
