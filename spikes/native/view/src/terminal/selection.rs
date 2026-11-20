//! A terminal's selection: xterm.js's desktop mouse selection, which
//! Paseo's web and desktop terminal keeps (`terminal-emulator-runtime.ts`
//! adds no selection code of its own).
//!
//! [Decision Log]
//! - Purpose: select text in a terminal tab and copy it, as a person does
//!   in Paseo's desktop terminal: a drag selects cells, a double click a
//!   word, a triple click a line, Shift+click extends.
//! - Choice: an anchor and a head as [`TextAt`]s, so the selection stays
//!   on its text while output scrolls (xterm.js's selection follows its
//!   buffer lines), and a unit (cells, words, lines) that grows both ends
//!   when it is read. A wide scalar is never split: an edge on its right
//!   half takes the left.
//! - Copy: the selected text with soft wraps joined, a newline at each
//!   hard end and trailing blanks left out (xterm.js's `getSelection`).
//! - Not here: what clears it (typing, a click) is the screen's.

use super::grid::{Grid, TextAt};

/// What a selection grows by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectBy {
    /// Cells (a drag).
    Cells,
    /// Words (a double click, then a drag).
    Words,
    /// Logical lines (a triple click, then a drag).
    Lines,
}

impl SelectBy {
    /// The unit a press of `clicks` clicks selects by: one cells, two
    /// words, three a line; a fourth starts over at cells.
    #[must_use]
    pub const fn of_clicks(clicks: usize) -> Self {
        match clicks.checked_rem(3) {
            Some(2) => Self::Words,
            Some(0) if clicks > 0 => Self::Lines,
            _ => Self::Cells,
        }
    }
}

/// A selection in a terminal's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    anchor: TextAt,
    head: TextAt,
    by: SelectBy,
}

impl Selection {
    /// A selection at `at`, by `by`; one of cells holds nothing until it
    /// is extended to another cell.
    #[must_use]
    pub const fn new(at: TextAt, by: SelectBy) -> Self {
        Self {
            anchor: at,
            head: at,
            by,
        }
    }

    /// Move the free end to `to`.
    pub const fn extend(&mut self, to: TextAt) {
        self.head = to;
    }

    /// Its free end.
    #[must_use]
    pub const fn head(&self) -> TextAt {
        self.head
    }

    /// What it grows by.
    #[must_use]
    pub const fn by(&self) -> SelectBy {
        self.by
    }

    /// Whether it holds no text yet: a press of cells not dragged.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by == SelectBy::Cells && self.anchor == self.head
    }

    /// Its first and last cell, both included, grown to whole words or
    /// lines and to whole wide scalars.
    #[must_use]
    pub fn range(&self, grid: &Grid) -> (TextAt, TextAt) {
        let (from, to) = if self.head < self.anchor {
            (self.head, self.anchor)
        } else {
            (self.anchor, self.head)
        };
        match self.by {
            SelectBy::Cells => (grid.lead_of(from), grid.lead_of(to)),
            SelectBy::Words => (grid.word_around(from).0, grid.word_around(to).1),
            SelectBy::Lines => (grid.line_around(from).0, grid.line_around(to).1),
        }
    }

    /// The text it holds, as a copy takes it.
    #[must_use]
    pub fn text(&self, grid: &Grid) -> String {
        if self.is_empty() {
            return String::new();
        }
        let (start, end) = self.range(grid);
        grid.text_between(start, end)
    }
}

/// The columns of the row that starts at `row_start` that `range` covers,
/// first and last both included, in a grid `cols` wide.
#[must_use]
pub fn span_on(range: (TextAt, TextAt), row_start: TextAt, cols: u16) -> Option<(u16, u16)> {
    let (start, end) = range;
    let row_last = row_start.at_col(cols.saturating_sub(1));
    if end < row_start || start > row_last {
        return None;
    }
    let first = if start >= row_start { start.col() } else { 0 };
    let last = if end <= row_last {
        end.col()
    } else {
        cols.saturating_sub(1)
    };
    (first <= last).then_some((first, last))
}
