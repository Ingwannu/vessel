//! Placing scalars, scrolling, and erasing. Split from the model so the
//! file stays within the line budget.

use unicode_width::UnicodeWidthChar;

use super::super::types::{
    Attrs, Cell, Glyph, Visibility, blank_cells, continuation_cell, scalar_cell,
};
use super::{Line, Saved, Shown, TerminalModel, blank_line, blank_screen, home_cursor};

/// How much of the screen an erase covers.
enum Region {
    /// The cursor's line only.
    Line,
    /// The cursor's line and the lines on that side of it.
    Screen,
}

impl TerminalModel {
    pub(super) fn print_char(&mut self, ch: char) {
        let width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width == 0 {
            return;
        }
        let need = u16::try_from(width).unwrap_or(1);
        if self.pending_wrap || self.cursor.col.saturating_add(need) > self.cols() {
            self.soft_wrap();
        }
        self.damage_wide();
        self.place(ch, need);
        self.mark();
    }

    pub(super) fn execute_control(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0b | 0x0c => self.newline(),
            b'\r' => {
                self.cursor.col = 0;
                self.pending_wrap = false;
                self.mark();
            }
            b'\t' => self.tab(),
            0x08 => self.backspace(),
            _ => {}
        }
    }

    pub(super) fn set_attrs(&mut self, attrs: Attrs) {
        self.attrs = attrs;
    }

    pub(super) const fn attrs(&self) -> Attrs {
        self.attrs
    }

    pub(super) fn move_cursor(&mut self, row: u16, col: u16) {
        let max_row = self.rows().saturating_sub(1);
        let max_col = self.cols().saturating_sub(1);
        self.cursor.row = row.min(max_row);
        self.cursor.col = col.min(max_col);
        self.pending_wrap = false;
        self.mark();
    }

    pub(super) fn move_by(&mut self, drow: i16, dcol: i16) {
        let row = offset(self.cursor.row, drow, self.rows().saturating_sub(1));
        let col = offset(self.cursor.col, dcol, self.cols().saturating_sub(1));
        self.move_cursor(row, col);
    }

    pub(super) fn erase_display(&mut self, kind: u16) {
        match kind {
            0 => self.erase_forward(Region::Screen),
            1 => self.erase_backward(Region::Screen),
            2 | 3 => self.clear_screen(kind),
            _ => {}
        }
        self.mark();
    }

    pub(super) fn erase_line(&mut self, kind: u16) {
        match kind {
            0 => self.erase_forward(Region::Line),
            1 => self.erase_backward(Region::Line),
            2 => {
                let cols = self.cols();
                let attrs = self.attrs;
                if let Some(line) = self.current_line() {
                    line.cells = blank_cells(usize::from(cols), attrs);
                    line.wrapped = false;
                }
            }
            _ => {}
        }
        self.mark();
    }

    pub(super) fn show_cursor(&mut self, visibility: Visibility) {
        self.cursor.visibility = visibility;
        self.mark();
    }

    pub(super) fn enter_alternate(&mut self) {
        if self.shown == Shown::Alternate {
            return;
        }
        self.saved = Some(Saved {
            cursor: self.cursor,
            pending_wrap: self.pending_wrap,
        });
        self.alternate = blank_screen(self.size);
        self.shown = Shown::Alternate;
        self.cursor = home_cursor();
        self.pending_wrap = false;
        self.mark();
    }

    pub(super) fn leave_alternate(&mut self) {
        let Shown::Alternate = self.shown else {
            return;
        };
        self.shown = Shown::Primary;
        if let Some(saved) = self.saved.take() {
            self.cursor = saved.cursor;
            self.pending_wrap = saved.pending_wrap;
        }
        self.clamp_cursor();
        self.mark();
    }

    pub(super) const fn cursor_pos(&self) -> (u16, u16) {
        (self.cursor.row, self.cursor.col)
    }

    fn place(&mut self, ch: char, need: u16) {
        let col = usize::from(self.cursor.col);
        let row = usize::from(self.cursor.row);
        let attrs = self.attrs;
        if let Some(line) = self.screen_mut().get_mut(row) {
            if let Some(cell) = line.cells.get_mut(col) {
                *cell = scalar_cell(ch, attrs);
            }
            if need == 2 {
                if let Some(next) = line.cells.get_mut(col.saturating_add(1)) {
                    *next = continuation_cell(attrs);
                }
            }
        }
        let next = self.cursor.col.saturating_add(need);
        if next >= self.cols() {
            self.pending_wrap = true;
            self.cursor.col = self.cols().saturating_sub(1);
        } else {
            self.cursor.col = next;
        }
    }

    /// A wide scalar occupies two cells. Overwriting either one clears both,
    /// so a later column is not left as a continuation of a scalar that is gone.
    fn damage_wide(&mut self) {
        let col = usize::from(self.cursor.col);
        let row = usize::from(self.cursor.row);
        let Some(line) = self.screen_mut().get_mut(row) else {
            return;
        };
        let glyph = line.cells.get(col).copied().map(Cell::glyph);
        if glyph == Some(Glyph::Continuation) && col > 0 {
            let prev = col.saturating_sub(1);
            if let Some(cell) = line.cells.get_mut(prev) {
                *cell = Cell::blank(Attrs::plain());
            }
        }
        if let Some(Glyph::Scalar(_)) = glyph {
            clear_following_continuation(line, col);
        }
    }

    fn soft_wrap(&mut self) {
        let row = usize::from(self.cursor.row);
        if let Some(line) = self.screen_mut().get_mut(row) {
            line.wrapped = true;
        }
        self.pending_wrap = false;
        self.cursor.col = 0;
        self.newline();
    }

    fn newline(&mut self) {
        let last = self.rows().saturating_sub(1);
        if self.cursor.row >= last {
            self.cursor.row = last;
            self.scroll();
        } else {
            self.cursor.row = self.cursor.row.saturating_add(1);
        }
        self.mark();
    }

    fn scroll(&mut self) {
        let blank = self.blank_row();
        let shown = self.shown;
        let screen = self.screen_mut();
        let top = take_front(screen);
        screen.push(blank);
        if shown == Shown::Primary {
            if let Some(top) = top {
                self.push_history(top);
            }
        }
    }

    pub(super) fn push_history(&mut self, line: Line) {
        let limit = usize::try_from(self.scrollback.get()).unwrap_or(usize::MAX);
        if limit == 0 {
            return;
        }
        self.history.push_back(line);
        while self.history.len() > limit {
            if self.history.pop_front().is_none() {
                break;
            }
        }
    }

    fn tab(&mut self) {
        // Stops are every 8 columns. 8 is a power of two, so the next stop
        // is the sum masked, with no division.
        let sum = self.cursor.col.saturating_add(8);
        let stop = sum & !7;
        let cols = self.cols();
        if stop >= cols {
            self.cursor.col = cols.saturating_sub(1);
            self.pending_wrap = true;
        } else {
            self.cursor.col = stop;
            self.pending_wrap = false;
        }
        self.mark();
    }

    fn backspace(&mut self) {
        if self.pending_wrap {
            self.pending_wrap = false;
        } else {
            self.cursor.col = self.cursor.col.saturating_sub(1);
        }
        self.mark();
    }

    fn clear_screen(&mut self, kind: u16) {
        let blank = self.blank_row();
        let rows = usize::from(self.rows());
        let shown = self.shown;
        let screen = self.screen_mut();
        screen.clear();
        screen.resize(rows, blank);
        if kind == 3 && shown == Shown::Primary {
            self.history.clear();
        }
    }

    fn erase_forward(&mut self, region: Region) {
        let row = usize::from(self.cursor.row);
        let col = usize::from(self.cursor.col);
        let attrs = self.attrs;
        let rows = self.screen().len();
        let Some(line) = self.screen_mut().get_mut(row) else {
            return;
        };
        for cell in line.cells.iter_mut().skip(col) {
            *cell = Cell::blank(attrs);
        }
        line.wrapped = false;
        if let Region::Line = region {
            return;
        }
        let screen = self.screen_mut();
        for line in screen.iter_mut().take(rows).skip(row.saturating_add(1)) {
            blank_line_cells(line, attrs);
        }
    }

    fn erase_backward(&mut self, region: Region) {
        let row = usize::from(self.cursor.row);
        let col = usize::from(self.cursor.col);
        let attrs = self.attrs;
        if let Region::Screen = region {
            let screen = self.screen_mut();
            for line in screen.iter_mut().take(row) {
                blank_line_cells(line, attrs);
            }
        }
        let Some(line) = self.screen_mut().get_mut(row) else {
            return;
        };
        for cell in line.cells.iter_mut().take(col.saturating_add(1)) {
            *cell = Cell::blank(attrs);
        }
    }

    fn current_line(&mut self) -> Option<&mut Line> {
        let row = usize::from(self.cursor.row);
        self.screen_mut().get_mut(row)
    }

    fn blank_row(&self) -> Line {
        blank_line(self.cols())
    }
}

fn clear_following_continuation(line: &mut Line, col: usize) {
    let Some(next) = line.cells.get(col.saturating_add(1)) else {
        return;
    };
    if next.glyph() != Glyph::Continuation {
        return;
    }
    if let Some(cell) = line.cells.get_mut(col.saturating_add(1)) {
        *cell = Cell::blank(Attrs::plain());
    }
}

fn blank_line_cells(line: &mut Line, attrs: Attrs) {
    for cell in &mut line.cells {
        *cell = Cell::blank(attrs);
    }
    line.wrapped = false;
}

/// The first row, if there is one. `split_off(1)` stays in range because
/// the screen was not empty: index 1 is at most the length.
fn take_front(screen: &mut Vec<Line>) -> Option<Line> {
    if screen.is_empty() {
        return None;
    }
    let rest = screen.split_off(1);
    let top = screen.pop();
    *screen = rest;
    top
}

fn offset(value: u16, delta: i16, max: u16) -> u16 {
    if delta < 0 {
        let step = u16::try_from(delta.saturating_neg()).unwrap_or(u16::MAX);
        value.saturating_sub(step)
    } else {
        let step = u16::try_from(delta).unwrap_or(0);
        value.saturating_add(step).min(max)
    }
}
