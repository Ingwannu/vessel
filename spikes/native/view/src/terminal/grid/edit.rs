//! The erasing, inserting, deleting and scrolling sequences. Each clears
//! the pending wrap (DEC's rule for an edit) and clamps its count to the
//! screen.

use super::Grid;
use super::modes::Pending;
use super::motion::Keep;
use crate::terminal::parse::{Count, Erase};

impl Grid {
    /// ED.
    pub(super) fn erase_display(&mut self, erase: Erase) {
        let (row, col, blank) = self.edit_at();
        let (rows, cols) = (self.size.rows(), self.size.cols());
        match erase {
            Erase::ToEnd => {
                self.screen_mut().erase_cells(row, col..cols, blank);
                self.screen_mut()
                    .erase_rows(row.saturating_add(1)..rows, blank);
            }
            Erase::ToStart => {
                self.screen_mut().erase_rows(0..row, blank);
                self.screen_mut()
                    .erase_cells(row, 0..col.saturating_add(1), blank);
            }
            Erase::All => self.screen_mut().erase_rows(0..rows, blank),
            Erase::Scrollback => self.history.clear(),
        }
    }

    /// EL.
    pub(super) fn erase_line(&mut self, erase: Erase) {
        let (row, col, blank) = self.edit_at();
        let cols = self.size.cols();
        let span = match erase {
            Erase::ToEnd => col..cols,
            Erase::ToStart => 0..col.saturating_add(1),
            Erase::All => 0..cols,
            // The parser never sends it (`CSI 3 K` is unsupported).
            Erase::Scrollback => return,
        };
        self.screen_mut().erase_cells(row, span, blank);
    }

    /// IL: `n` blank lines at the cursor's row, inside the region only;
    /// the cursor goes to column 0.
    pub(super) fn insert_lines(&mut self, n: Count) {
        let (row, _, blank) = self.edit_at();
        if self.screen().region.holds(row) {
            self.screen_mut().scroll_down(row, n.get(), blank);
            self.at.col = 0;
        }
    }

    /// DL: `n` lines removed at the cursor's row, inside the region only;
    /// nothing goes to history.
    pub(super) fn delete_lines(&mut self, n: Count) {
        let (row, _, blank) = self.edit_at();
        if self.screen().region.holds(row) {
            drop(self.screen_mut().scroll_up(row, n.get(), blank));
            self.at.col = 0;
        }
    }

    /// ICH.
    pub(super) fn insert_chars(&mut self, n: Count) {
        let (row, col, blank) = self.edit_at();
        self.screen_mut().insert_cells(row, col, n.get(), blank);
    }

    /// DCH.
    pub(super) fn delete_chars(&mut self, n: Count) {
        let (row, col, blank) = self.edit_at();
        self.screen_mut().delete_cells(row, col, n.get(), blank);
    }

    /// ECH: `n` cells blank from the cursor; the cursor stays.
    pub(super) fn erase_chars(&mut self, n: Count) {
        let (row, col, blank) = self.edit_at();
        let end = col.saturating_add(n.get());
        self.screen_mut().erase_cells(row, col..end, blank);
    }

    /// SU: the region up `n`; nothing goes to history (xterm.js).
    pub(super) fn scroll_up(&mut self, n: Count) {
        self.at.pending = Pending::Nothing;
        self.scroll_region_up(n.get(), Keep::Nothing);
    }

    /// SD: the region down `n`.
    pub(super) fn scroll_down(&mut self, n: Count) {
        let (_, _, blank) = self.edit_at();
        let top = self.screen().region.top;
        self.screen_mut().scroll_down(top, n.get(), blank);
    }

    /// The cursor's cell and the blank, the pending wrap cleared.
    const fn edit_at(&mut self) -> (u16, u16, super::Cell) {
        self.at.pending = Pending::Nothing;
        (self.at.row, self.at.col, self.blank())
    }
}
