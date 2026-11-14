//! A new size, without reflow (the reflow slice joins soft-wrapped lines
//! again): each row is cut or padded to the new width, and rows move
//! between the primary screen and history as xterm.js moves them.

use super::cell::Cell;
use super::modes::{Pending, Shown};
use super::screen::{Region, Screen};
use super::{Grid, History};
use crate::terminal::size::GridSize;

impl Grid {
    /// Resize to `size`. A shorter screen first drops blank rows below
    /// the cursor, then its top rows (the primary screen's go to history);
    /// a taller primary screen takes lines back from history. Both scroll
    /// regions become the whole screen; the cursor is clamped.
    pub fn resize(&mut self, size: GridSize) {
        if size == self.size {
            return;
        }
        let (old, blank) = (self.size, self.blank());
        self.primary.fit_cols(size.cols(), blank);
        self.alternate.fit_cols(size.cols(), blank);
        self.tabs.resize(old.cols(), size.cols());
        let mut row = self.at.row;
        let (primary_row, alternate_row) = match self.shown {
            Shown::Primary => (Some(&mut row), None),
            Shown::Alternate => (None, Some(&mut row)),
        };
        fit_rows(
            &mut self.primary,
            Some(&mut self.history),
            primary_row,
            size,
            blank,
        );
        fit_rows(&mut self.alternate, None, alternate_row, size, blank);
        self.primary.region = Region::whole(size);
        self.alternate.region = Region::whole(size);
        self.size = size;
        self.at.row = row.min(size.last_row());
        self.at.col = self.at.col.min(size.last_col());
        self.at.pending = Pending::Nothing;
        self.changed();
    }
}

/// `screen` to `size`'s rows. `history` takes the rows that leave the top
/// and gives rows back when the screen grows (the primary screen only);
/// `cursor` is the cursor's row when this screen is showing.
fn fit_rows(
    screen: &mut Screen,
    mut history: Option<&mut History>,
    mut cursor: Option<&mut u16>,
    size: GridSize,
    blank: Cell,
) {
    let rows = usize::from(size.rows());
    while screen.rows() > rows {
        let below_cursor = cursor
            .as_deref()
            .is_none_or(|row| screen.rows().saturating_sub(1) > usize::from(*row));
        if below_cursor && screen.take_blank_bottom() {
            continue;
        }
        let Some(top) = screen.take_top() else {
            break;
        };
        if let Some(history) = history.as_deref_mut() {
            history.push(top);
        }
        if let Some(row) = cursor.as_deref_mut() {
            *row = row.saturating_sub(1);
        }
    }
    while screen.rows() < rows {
        match history.as_deref_mut().and_then(History::pop_newest) {
            Some(mut line) => {
                line.fit(size.cols(), blank);
                screen.push_top(line);
                if let Some(row) = cursor.as_deref_mut() {
                    *row = row.saturating_add(1);
                }
            }
            None => screen.push_bottom(size.cols(), blank),
        }
    }
}
