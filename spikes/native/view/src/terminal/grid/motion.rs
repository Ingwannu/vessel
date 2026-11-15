//! Moving the cursor and scrolling: line feeds, the `CUx` family,
//! positions in origin mode, the alternate screen and saved cursors. Each
//! count is clamped to the grid before it moves anything.

use super::Grid;
use super::modes::{At, LineFeed, Origin, Pending, Saved, Shown};
use super::screen::Region;
use crate::terminal::parse::{Count, Position};

impl Grid {
    /// LF, VT, FF: down one line, scrolling at the region's bottom; with
    /// LNM set also back to column 0.
    pub(super) fn line_feed(&mut self) {
        self.index();
        if self.modes.line_feed == LineFeed::DownAndReturn {
            self.at.col = 0;
        }
    }

    /// IND: down one line, scrolling the region up at its bottom. The line
    /// that leaves the top of the primary screen goes to history when the
    /// region starts at the top (xterm.js's rule).
    pub(super) fn index(&mut self) {
        self.at.pending = Pending::Nothing;
        let region = self.screen().region;
        if self.at.row == region.bottom {
            self.scroll_region_up(1, Keep::History);
        } else if self.at.row < self.size.last_row() {
            self.at.row = self.at.row.saturating_add(1);
        }
    }

    /// RI: up one line, scrolling the region down at its top.
    pub(super) fn reverse_index(&mut self) {
        self.at.pending = Pending::Nothing;
        let region = self.screen().region;
        if self.at.row == region.top {
            let blank = self.blank();
            self.screen_mut().scroll_down(region.top, 1, blank);
        } else {
            self.at.row = self.at.row.saturating_sub(1);
        }
    }

    /// The region scrolled up by `n`; what leaves the top is kept in
    /// history only by a line feed on the primary screen whose region
    /// starts at the top.
    pub(super) fn scroll_region_up(&mut self, n: u16, keep: Keep) {
        let blank = self.blank();
        let top = self.screen().region.top;
        let left = self.screen_mut().scroll_up(top, n, blank);
        if keep == Keep::History && self.shown == Shown::Primary && top == 0 {
            for line in left {
                self.history.push(line);
            }
        }
    }

    /// CUU and CPL: up `n`, stopping at the region's top when the cursor
    /// starts inside it.
    pub(super) fn up(&mut self, n: Count) {
        let region = self.screen().region;
        let floor = if self.at.row >= region.top {
            region.top
        } else {
            0
        };
        self.at.row = self.at.row.saturating_sub(n.get()).max(floor);
        self.at.pending = Pending::Nothing;
    }

    /// CUD and CNL: down `n`, stopping at the region's bottom when the
    /// cursor starts inside it.
    pub(super) fn down(&mut self, n: Count) {
        let region = self.screen().region;
        let ceiling = if self.at.row <= region.bottom {
            region.bottom
        } else {
            self.size.last_row()
        };
        self.at.row = self.at.row.saturating_add(n.get()).min(ceiling);
        self.at.pending = Pending::Nothing;
    }

    /// CUF and CUB: right or left `n`, within the line.
    pub(super) fn sideways(&mut self, n: Count, way: Way) {
        self.at.col = match way {
            Way::Forward => self
                .at
                .col
                .saturating_add(n.get())
                .min(self.size.last_col()),
            Way::Back => self.at.col.saturating_sub(n.get()),
        };
        self.at.pending = Pending::Nothing;
    }

    /// CHA: column `col` (1-based).
    pub(super) fn column(&mut self, col: Position) {
        self.at.col = col.get().saturating_sub(1).min(self.size.last_col());
        self.at.pending = Pending::Nothing;
    }

    /// VPA: row `row` (1-based, from the region's top in origin mode).
    pub(super) fn row(&mut self, row: Position) {
        let region = self.screen().region;
        let row = row.get().saturating_sub(1);
        self.at.row = match self.modes.origin {
            Origin::Screen => row.min(self.size.last_row()),
            Origin::Region => region.top.saturating_add(row).min(region.bottom),
        };
        self.at.pending = Pending::Nothing;
    }

    /// The home cell: the screen's or, in origin mode, the region's top.
    pub(super) fn home(&mut self) {
        self.row(Count::ONE);
        self.at.col = 0;
    }

    /// DECSC: the cursor and its modes kept for this screen.
    pub(super) fn save_cursor(&mut self) {
        let saved = Saved {
            at: self.at,
            origin: self.modes.origin,
            auto_wrap: self.modes.auto_wrap,
            pen: self.pen,
        };
        *self.saved_slot() = Some(saved);
    }

    /// DECRC: what DECSC kept, clamped to the grid; with nothing kept, the
    /// home cell, the modes' reset values and the plain pen (xterm's rule).
    pub(super) fn restore_cursor(&mut self) {
        let saved = self.saved_slot().unwrap_or_default();
        self.modes.origin = saved.origin;
        self.modes.auto_wrap = saved.auto_wrap;
        self.pen = saved.pen;
        self.at = At {
            row: saved.at.row.min(self.size.last_row()),
            col: saved.at.col.min(self.size.last_col()),
            pending: saved.at.pending,
        };
    }

    /// The alternate screen, cleared, with the cursor where it was (what
    /// xterm.js does for `?47`, `?1047` and `?1049`). Already showing: no
    /// change.
    pub(super) fn enter_alternate(&mut self) {
        if self.shown == Shown::Alternate {
            return;
        }
        let (size, blank) = (self.size, self.blank());
        self.alternate.clear(size, blank);
        self.shown = Shown::Alternate;
    }

    /// The primary screen again, as it was left.
    pub(super) fn leave_alternate(&mut self) {
        self.shown = Shown::Primary;
        self.at.row = self.at.row.min(self.size.last_row());
    }

    /// DECSTBM: rows `top` through `bottom` (1-based; absent is the
    /// screen's edge). A bottom past the screen is the screen's last row;
    /// a region of fewer than two rows is refused and nothing changes.
    pub(super) fn set_region(&mut self, top: Option<Position>, bottom: Option<Position>) {
        let top = top.map_or(0, |top| top.get().saturating_sub(1));
        let bottom = bottom.map_or(self.size.last_row(), |bottom| {
            bottom.get().saturating_sub(1).min(self.size.last_row())
        });
        if top >= bottom {
            return;
        }
        self.screen_mut().region = Region { top, bottom };
        self.home();
    }
}

/// Whether lines that leave a scroll go to history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Keep {
    /// A line feed's scroll.
    History,
    /// `CSI S`, and every scroll of the alternate screen.
    Nothing,
}

/// Right or left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Way {
    Forward,
    Back,
}
