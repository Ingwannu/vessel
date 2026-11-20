//! The grid as text a person selects and searches: a cell's address that
//! keeps its place as output scrolls, and the logical lines soft wraps
//! join.
//!
//! [Decision Log]
//! - Purpose: a selection and a find's matches must stay on the text they
//!   were made on while the program prints, and a match or a copy must
//!   cross a soft wrap as xterm.js's does (Paseo's desktop terminal).
//! - Constraint: a row's index into history moves with every line that
//!   scrolls off the top, and the oldest lines are dropped at the bound.
//! - Choice: a cell is addressed by its row's [`HistoryRow`] (an epoch and
//!   a count of rows ever kept, the viewport's own anchor) and a column.
//!   The live screen continues history's count, so a row keeps its address
//!   when it scrolls into history. A dropped or cleared row has no text,
//!   and asking for it answers nothing rather than another row's text.
//! - Cost: a width change reflows history and the screen, so addresses made
//!   before it point at re-wrapped text; the screen drops a selection and
//!   runs a find again when the width changes.

use super::history::HistoryRow;
use super::{Cell, Glyph, Grid, Line, Scroll, Shown, Wrap};

/// A cell of the grid's text, at a place that survives scrolling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextAt {
    row: HistoryRow,
    col: u16,
}

impl TextAt {
    /// The column.
    #[must_use]
    pub const fn col(self) -> u16 {
        self.col
    }

    /// The same row at `col`.
    #[must_use]
    pub const fn at_col(self, col: u16) -> Self {
        Self { row: self.row, col }
    }
}

/// A logical line: rows a soft wrap joins, its cells' text and where each
/// scalar came from.
#[derive(Debug, Clone, Default)]
pub(crate) struct LogicalLine {
    /// One entry a printed or blank cell, continuations left out.
    pub(crate) chars: Vec<char>,
    /// Where each of `chars` is.
    pub(crate) places: Vec<TextAt>,
}

impl Grid {
    /// The address of the first row the showing screen can scroll to: the
    /// oldest history row, or the alternate screen's top.
    pub(crate) fn text_first(&self) -> HistoryRow {
        match self.shown {
            Shown::Primary => self.history.first(),
            Shown::Alternate => self.screen_top(),
        }
    }

    /// The address one past the live screen's last row.
    pub(crate) fn text_end(&self) -> HistoryRow {
        self.screen_top()
            .after(Scroll(u32::from(self.size.rows())))
    }

    /// The live screen's top row.
    fn screen_top(&self) -> HistoryRow {
        self.history.first().after(Scroll(
            u32::try_from(self.history.len()).unwrap_or(u32::MAX),
        ))
    }

    /// The row at `row`, if it is still held.
    pub(crate) fn line_at(&self, row: HistoryRow) -> Option<&Line> {
        let first = self.history.first();
        let index = row.index_from(first)?;
        match index.checked_sub(self.history.len()) {
            None => match self.shown {
                Shown::Primary => self.history.line(index),
                // History is not reachable from the alternate screen.
                Shown::Alternate => None,
            },
            Some(screen_row) => self.screen().lines().get(screen_row),
        }
    }

    /// The cell a view `scroll` lines back shows at `view_row`, `col`.
    #[must_use]
    pub fn text_at(&self, scroll: Scroll, view_row: u16, col: u16) -> TextAt {
        let back = match self.shown {
            Shown::Primary => scroll,
            Shown::Alternate => Scroll::default(),
        };
        let top = self.screen_top().before(back);
        TextAt {
            row: top.after(Scroll(u32::from(view_row))),
            col: col.min(self.size.cols().saturating_sub(1)),
        }
    }

    /// The view row `at` shows on when the view is `scroll` lines back, if
    /// it is in view.
    #[must_use]
    pub fn view_row(&self, at: TextAt, scroll: Scroll) -> Option<u16> {
        let back = match self.shown {
            Shown::Primary => scroll,
            Shown::Alternate => Scroll::default(),
        };
        let top = self.screen_top().before(back);
        let row = at.row.index_from(top)?;
        u16::try_from(row)
            .ok()
            .filter(|row| *row < self.size.rows())
    }

    /// The row a scroll would bring to the top of a view to show `at`, as
    /// lines back from the live screen (0 when it is on the screen).
    #[must_use]
    pub fn scroll_to_show(&self, at: TextAt) -> Scroll {
        let top = self.screen_top();
        match at.row.index_from(self.text_first()) {
            Some(_) if at.row < top => top.distance(at.row),
            _ => Scroll::default(),
        }
    }

    /// The logical line starting at `row` (the row after a hard end), and
    /// the row after it. `None` once `row` is past the screen or dropped.
    pub(crate) fn logical_line(&self, row: HistoryRow) -> Option<(LogicalLine, HistoryRow)> {
        let end = self.text_end();
        if row >= end {
            return None;
        }
        let mut text = LogicalLine::default();
        let mut at = row;
        loop {
            let line = self.line_at(at)?;
            push_cells(&mut text, line.cells(), at);
            let next = at.after(Scroll(1));
            if line.wrap() == Wrap::Hard || next >= end {
                return Some((text, next));
            }
            at = next;
        }
    }

    /// The first row of the logical line `row` is part of.
    pub(crate) fn logical_start(&self, row: HistoryRow) -> HistoryRow {
        let first = self.text_first();
        let mut at = row;
        while at > first {
            let before = at.before(Scroll(1));
            match self.line_at(before) {
                Some(line) if line.wrap() == Wrap::Soft => at = before,
                _ => break,
            }
        }
        at
    }

    /// `at`, or the cell left of it when `at` is the right half of a wide
    /// scalar, so a selection's edge never splits one.
    #[must_use]
    pub fn lead_of(&self, at: TextAt) -> TextAt {
        let continuation = self
            .line_at(at.row)
            .and_then(|line| line.cells().get(usize::from(at.col)))
            .is_some_and(|cell| cell.glyph() == Glyph::Continuation);
        if continuation {
            at.at_col(at.col.saturating_sub(1))
        } else {
            at
        }
    }

    /// The text from `start` through `end`, both included: a soft wrap
    /// joins its rows, a hard end is a newline, and each line's trailing
    /// blanks are left out (xterm.js's copy).
    #[must_use]
    pub fn text_between(&self, start: TextAt, end: TextAt) -> String {
        let (start, end) = if end < start {
            (end, start)
        } else {
            (start, end)
        };
        let mut out = String::new();
        let mut row = self.logical_start(start.row);
        let mut first_line = true;
        while row <= end.row {
            let Some((line, next)) = self.logical_line(row) else {
                break;
            };
            let picked: String = line
                .chars
                .iter()
                .zip(&line.places)
                .filter(|(_, place)| **place >= start && **place <= end)
                .map(|(c, _)| *c)
                .collect();
            if !first_line {
                out.push('\n');
            }
            out.push_str(picked.trim_end_matches(' '));
            first_line = false;
            row = next;
        }
        out
    }

    /// The word around `at` as xterm.js finds it on a double click: the run
    /// of cells that are not separators, across soft wraps; a separator or
    /// a blank alone.
    #[must_use]
    pub fn word_around(&self, at: TextAt) -> (TextAt, TextAt) {
        let Some((line, _)) = self.logical_line(self.logical_start(at.row)) else {
            return (at, at);
        };
        let Some(index) = line.places.iter().position(|place| *place >= at) else {
            return (at, at);
        };
        let word = |c: char| !is_separator(c);
        let Some(&here) = line.chars.get(index) else {
            return (at, at);
        };
        if !word(here) {
            let same = |c: char| c == here;
            let (from, to) = run(&line.chars, index, same);
            return bounds(&line, from, to, at);
        }
        let (from, to) = run(&line.chars, index, word);
        bounds(&line, from, to, at)
    }

    /// The logical line around `at` (a triple click), its trailing blanks
    /// left out.
    #[must_use]
    pub fn line_around(&self, at: TextAt) -> (TextAt, TextAt) {
        let Some((line, _)) = self.logical_line(self.logical_start(at.row)) else {
            return (at, at);
        };
        let last = line
            .chars
            .iter()
            .rposition(|c| *c != ' ')
            .unwrap_or_default();
        bounds(&line, 0, last, at)
    }
}

/// xterm.js's default `wordSeparator`, and a blank.
fn is_separator(c: char) -> bool {
    matches!(c, ' ' | '(' | ')' | '[' | ']' | '{' | '}' | '\'' | ',' | '"' | '`')
}

/// The run of `chars` around `index` that `keep` holds for.
fn run(chars: &[char], index: usize, keep: impl Fn(char) -> bool) -> (usize, usize) {
    let mut from = index;
    while let Some(before) = from.checked_sub(1)
        && chars.get(before).is_some_and(|c| keep(*c))
    {
        from = before;
    }
    let mut to = index;
    while let Some(after) = to.checked_add(1)
        && chars.get(after).is_some_and(|c| keep(*c))
    {
        to = after;
    }
    (from, to)
}

fn bounds(line: &LogicalLine, from: usize, to: usize, at: TextAt) -> (TextAt, TextAt) {
    (
        line.places.get(from).copied().unwrap_or(at),
        line.places.get(to).copied().unwrap_or(at),
    )
}

fn push_cells(text: &mut LogicalLine, cells: &[Cell], row: HistoryRow) {
    for (col, cell) in (0_u16..).zip(cells) {
        let c = match cell.glyph() {
            Glyph::Scalar(c) => c,
            Glyph::Empty => ' ',
            Glyph::Continuation => continue,
        };
        text.chars.push(c);
        text.places.push(TextAt { row, col });
    }
}
