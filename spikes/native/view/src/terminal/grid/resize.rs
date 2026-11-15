//! A new size. A width change reflows the primary screen and history
//! (`reflow`); a height change alone moves rows between the primary screen
//! and history as xterm.js moves them. The alternate screen is cut or
//! padded and keeps nothing it loses.

use super::cell::{Cell, Line};
use super::modes::{At, Pending, Shown};
use super::reflow::reflow;
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
        // New cells are plain, not the pen's background (xterm.js).
        let (old, blank) = (self.size, Cell::BLANK);
        self.alternate.fit_cols(size.cols(), blank);
        self.tabs.resize(old.cols(), size.cols());
        let mut at = At {
            pending: Pending::Nothing,
            ..self.at
        };
        if size.cols() == old.cols() {
            let row = (self.shown == Shown::Primary).then_some(&mut at.row);
            fit_rows(&mut self.primary, Some(&mut self.history), row, size, blank);
        } else if let Some(moved) = self.reflow_primary(size) {
            at = moved;
        }
        let row = (self.shown == Shown::Alternate).then_some(&mut at.row);
        fit_rows(&mut self.alternate, None, row, size, blank);
        self.primary.region = Region::whole(size);
        self.alternate.region = Region::whole(size);
        self.size = size;
        self.at = At {
            row: at.row.min(size.last_row()),
            col: at.col.min(size.last_col()),
            pending: at.pending,
        };
        self.changed();
    }

    /// History and the primary screen reflowed to `size`'s width (the
    /// cursor's run of lines only cut, `reflow`), the screen being their
    /// last rows: blank rows below the cursor go first, and the cursor's row
    /// is always on it, so rows a program printed below the cursor that no
    /// longer fit under it are dropped (xterm.js moves its viewport
    /// instead). Returns the cursor when the primary screen is showing.
    fn reflow_primary(&mut self, size: GridSize) -> Option<At> {
        let mut lines = self.history.take_all();
        let base = lines.len();
        lines.append(&mut self.primary.take_lines());
        let cursor =
            (self.shown == Shown::Primary).then(|| base.saturating_add(usize::from(self.at.row)));
        let (mut lines, moved) = reflow(lines, cursor, size.cols());
        let keep = moved.map_or(0, |line| line.saturating_add(1));
        while lines.len() > keep && lines.last().is_some_and(Line::is_blank) {
            drop(lines.pop());
        }
        let rows = usize::from(size.rows());
        let start = lines
            .len()
            .saturating_sub(rows)
            .min(moved.unwrap_or(usize::MAX));
        let mut screen = lines.split_off(start);
        screen.truncate(rows);
        for line in &mut screen {
            line.fit(size.cols(), Cell::BLANK);
        }
        screen.resize_with(rows, || Line::blank(size.cols(), Cell::BLANK));
        for line in lines {
            self.history.push(line);
        }
        self.primary.set_lines(screen);
        let row = u16::try_from(moved?.saturating_sub(start)).unwrap_or(u16::MAX);
        let col = self.at.col;
        // The cursor's line was not reflowed: it keeps its column. A
        // pending wrap with room after it becomes the next cell.
        Some(match self.at.pending {
            Pending::Wrap if col < size.last_col() => At {
                row,
                col: col.saturating_add(1),
                pending: Pending::Nothing,
            },
            pending @ (Pending::Wrap | Pending::Nothing) => At { row, col, pending },
        })
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
