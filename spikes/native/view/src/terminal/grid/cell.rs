//! A cell and a line of cells.
//!
//! A cell is plain data the kit can paint. The attributes slice adds the
//! colours and the pen; the wide-cell slice adds the continuation of a
//! wide scalar. Every erase takes the blank to write (`Grid::blank`), so
//! erasing in the current background is a change in one place.

/// What occupies a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// Nothing printed.
    Empty,
    /// A scalar the program printed.
    Scalar(char),
}

/// One cell of the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    glyph: Glyph,
}

impl Cell {
    /// An unprinted cell in the default attributes.
    pub const BLANK: Self = Self {
        glyph: Glyph::Empty,
    };

    /// A printed scalar.
    #[must_use]
    pub const fn scalar(c: char) -> Self {
        Self {
            glyph: Glyph::Scalar(c),
        }
    }

    /// The glyph.
    #[must_use]
    pub const fn glyph(self) -> Glyph {
        self.glyph
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

    /// Exactly `cols` wide: cut, or padded with `blank`.
    pub(super) fn fit(&mut self, cols: u16, blank: Cell) {
        self.cells.resize(usize::from(cols), blank);
    }

    /// Without its trailing unprinted cells (a line kept in history).
    pub(super) fn trimmed(mut self) -> Self {
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
