//! A cell and a line of cells.
//!
//! A cell is plain data the kit can paint: a glyph and the pen it was
//! printed or erased with. A wide scalar takes its cell and the next, a
//! continuation (`width`). Every erase takes the blank to write
//! (`Grid::blank`, the current background).

use super::pen::Pen;

/// What occupies a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// Nothing printed.
    Empty,
    /// A scalar the program printed. It is two cells wide exactly when the
    /// next cell is a `Continuation`.
    Scalar(char),
    /// The second cell of a wide scalar: not a character of its own (find
    /// and selection skip it; the kit draws the scalar across both).
    Continuation,
}

/// One cell of the grid: 20 bytes (a test pins it; history's bound is
/// counted in cells).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    glyph: Glyph,
    pen: Pen,
}

impl Cell {
    /// An unprinted cell in the default attributes.
    pub const BLANK: Self = Self {
        glyph: Glyph::Empty,
        pen: Pen::PLAIN,
    };

    /// `c` printed with `pen`.
    #[must_use]
    pub const fn printed(c: char, pen: Pen) -> Self {
        Self {
            glyph: Glyph::Scalar(c),
            pen,
        }
    }

    /// The second cell of a wide scalar printed with `pen`.
    #[must_use]
    pub const fn continuation(pen: Pen) -> Self {
        Self {
            glyph: Glyph::Continuation,
            pen,
        }
    }

    /// An unprinted cell erased with `pen` (its background).
    #[must_use]
    pub const fn blank(pen: Pen) -> Self {
        Self {
            glyph: Glyph::Empty,
            pen,
        }
    }

    /// The glyph.
    #[must_use]
    pub const fn glyph(self) -> Glyph {
        self.glyph
    }

    /// The pen.
    #[must_use]
    pub const fn pen(self) -> Pen {
        self.pen
    }
}

/// How a line ends: whether the program's text ran on past the last
/// column into the next line (the reflow slice joins soft-wrapped lines
/// again on a resize; find and copy read them as one line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrap {
    /// The line ended here (a line feed, or nothing printed past it).
    Hard,
    /// The cursor wrapped off the last column onto the next line.
    Soft,
}

/// One row of cells. On a screen every line is exactly as wide as the
/// grid; in history a line keeps no trailing blank cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    cells: Vec<Cell>,
    wrap: Wrap,
}

impl Line {
    /// `cols` cells of `blank`.
    pub(super) fn blank(cols: u16, blank: Cell) -> Self {
        Self {
            cells: vec![blank; usize::from(cols)],
            wrap: Wrap::Hard,
        }
    }

    /// A line of `cells` ending as `wrap`.
    pub(super) const fn from_cells(cells: Vec<Cell>, wrap: Wrap) -> Self {
        Self { cells, wrap }
    }

    /// Its cells, taken.
    pub(super) fn into_cells(self) -> Vec<Cell> {
        self.cells
    }

    /// Nothing printed and nothing erased in a colour, ending hard.
    pub(super) fn is_blank(&self) -> bool {
        self.wrap == Wrap::Hard && self.cells.iter().all(|cell| *cell == Cell::BLANK)
    }

    /// The cells.
    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// How the line ends.
    #[must_use]
    pub const fn wrap(&self) -> Wrap {
        self.wrap
    }

    pub(super) const fn set_wrap(&mut self, wrap: Wrap) {
        self.wrap = wrap;
    }

    pub(super) const fn cells_mut(&mut self) -> &mut Vec<Cell> {
        &mut self.cells
    }

    /// The cell at `col`, if the line is that wide.
    pub(super) fn cell_mut(&mut self, col: u16) -> Option<&mut Cell> {
        self.cells.get_mut(usize::from(col))
    }

    /// Exactly `cols` wide: cut, or padded with `blank`; a wide scalar cut
    /// in half at the edge is blanked.
    pub(super) fn fit(&mut self, cols: u16, blank: Cell) {
        self.cells.resize(usize::from(cols), blank);
        let last = self.cells.len();
        self.fix_pairs(last.saturating_sub(1), last, blank);
    }

    /// At most `cols` wide: cut, a wide scalar cut in half at the edge
    /// blanked; a shorter line is left as it is (only rows shown are
    /// padded).
    pub(super) fn cut(mut self, cols: u16, blank: Cell) -> Self {
        let cols = usize::from(cols);
        if self.cells.len() > cols {
            self.cells.truncate(cols);
            self.fix_pairs(cols.saturating_sub(1), cols, blank);
        }
        self
    }

    /// Blank the half pairs in cells `from..to` and on either side of them.
    pub(super) fn fix_pairs(&mut self, from: usize, to: usize, blank: Cell) {
        super::width::fix_pairs(&mut self.cells, from, to, blank);
    }

    /// Without its trailing unprinted cells when it ended hard (a line
    /// kept in history). A soft-wrapped line keeps them: a gap erased at its
    /// end is part of the line reflow joins (xterm.js keeps it too).
    pub(super) fn trimmed(mut self) -> Self {
        if self.wrap == Wrap::Soft {
            return self;
        }
        let kept = self
            .cells
            .iter()
            .rposition(|cell| *cell != Cell::BLANK)
            .map_or(0, |last| last.saturating_add(1));
        self.cells.truncate(kept);
        self.cells.shrink_to_fit();
        self
    }
}
