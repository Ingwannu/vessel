//! The grid: two screens, history, the cursor and the modes, changed only
//! by the parser's typed actions (ADR 0073).
//!
//! [Decision Log]
//! - Purpose: the model a terminal view draws from. It must hold whatever
//!   a program prints without panicking and within a stated bound.
//! - Constraint: every count in an action is up to 65,535 and every size
//!   up to [`GridSize::MAX_COLS`] by [`GridSize::MAX_ROWS`]; the lint set
//!   forbids indexing and unchecked arithmetic, so each edit clamps.
//! - Choice: xterm.js's behaviour where VT100 leaves room (Paseo draws
//!   with xterm.js): a line that scrolls off the top of the primary screen
//!   by a line feed goes to history when the scroll region starts at the
//!   top; `CSI S` does not keep it; entering the alternate screen clears it
//!   and keeps the cursor; a resize resets both regions; a width change
//!   reflows the primary screen and history (`reflow`); a taller primary
//!   screen takes lines back from history.
//! - Bounds: two screens of at most 512 Ki cells each; history at most
//!   the setting's lines and `history::CELL_LIMIT` cells; a title at most
//!   the parser's 1,024 bytes; tab stops at most one per column.

mod act;
mod cell;
mod edit;
mod history;
mod modes;
mod motion;
mod pen;
mod print;
mod reflow;
mod resize;
mod screen;
mod sgr;
mod tabs;
mod width;

#[cfg(test)]
mod tests;

pub use cell::{Cell, Glyph, Line, Wrap};
pub use history::CELL_LIMIT;
pub use modes::{Bracketed, CursorKeys, Shown, Visibility};
pub use pen::{Colour, Concealment, Pen, Polarity, Rgb, Slant, Strike, Underline, Weight};

use history::History;
use modes::{At, Modes, Saved};
use screen::Screen;
use tabs::TabStops;

use super::size::{GridSize, ScrollbackLines};

/// How many times the drawn grid has changed. A view draws only when this
/// differs from the snapshot it drew last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Generation(u64);

impl Generation {
    /// The next change; it stops at the end of `u64` rather than wrapping,
    /// so a snapshot never looks older than the one before it.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// How far a view has scrolled back: lines of history above the live
/// screen. 0 is the live screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Scroll(pub u32);

/// The cursor's cell in a snapshot, 0-based from its top-left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorCell {
    /// Row.
    pub row: u16,
    /// Column.
    pub col: u16,
}

/// What a view draws: the visible cells, row-major, `cols × rows` of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// The grid's size.
    pub size: GridSize,
    /// Row-major cells.
    pub cells: Vec<Cell>,
    /// The cursor, when it is shown and in view.
    pub cursor: Option<CursorCell>,
    /// Lines of history above the screen (a view's scroll range).
    pub history: u32,
    /// The grid's change count when this was taken.
    pub generation: Generation,
}

/// The terminal grid.
#[derive(Debug)]
pub struct Grid {
    size: GridSize,
    primary: Screen,
    alternate: Screen,
    shown: Shown,
    history: History,
    at: At,
    /// DECSC's slot on the primary screen.
    saved_primary: Option<Saved>,
    /// DECSC's slot on the alternate screen.
    saved_alternate: Option<Saved>,
    tabs: TabStops,
    modes: Modes,
    /// What the next printed cell is drawn with (SGR).
    pen: Pen,
    title: Option<String>,
    generation: Generation,
}

impl Grid {
    /// An empty grid of `size` keeping `scrollback` lines of history.
    #[must_use]
    pub fn new(size: GridSize, scrollback: ScrollbackLines) -> Self {
        Self {
            size,
            primary: Screen::new(size),
            alternate: Screen::new(size),
            shown: Shown::Primary,
            history: History::new(scrollback),
            at: At::default(),
            saved_primary: None,
            saved_alternate: None,
            tabs: TabStops::new(size.cols()),
            modes: Modes::default(),
            pen: Pen::PLAIN,
            title: None,
            generation: Generation::default(),
        }
    }

    /// The size.
    #[must_use]
    pub const fn size(&self) -> GridSize {
        self.size
    }

    /// Which screen is showing.
    #[must_use]
    pub const fn shown(&self) -> Shown {
        self.shown
    }

    /// The arrow keys' encoding the program asked for.
    #[must_use]
    pub const fn cursor_keys(&self) -> CursorKeys {
        self.modes.cursor_keys
    }

    /// Whether the program asked for bracketed paste.
    #[must_use]
    pub const fn bracketed(&self) -> Bracketed {
        self.modes.bracketed
    }

    /// The title the program set last (OSC 0 or 2).
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// The cursor's cell on the live screen, shown or not (a report of its
    /// position answers either way).
    #[must_use]
    pub const fn cursor(&self) -> CursorCell {
        CursorCell {
            row: self.at.row,
            col: self.at.col,
        }
    }

    /// The change count.
    #[must_use]
    pub const fn generation(&self) -> Generation {
        self.generation
    }

    /// Lines of history held.
    #[must_use]
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// History line `index`, oldest first (no trailing blank cells).
    #[must_use]
    pub fn history_line(&self, index: usize) -> Option<&Line> {
        self.history.line(index)
    }

    /// The showing screen's rows, top first, each `cols` wide.
    #[must_use]
    pub fn screen_lines(&self) -> &[Line] {
        self.screen().lines()
    }

    /// The visible cells, `scroll` lines back into history (the alternate
    /// screen has none, so it is always live).
    #[must_use]
    pub fn snapshot(&self, scroll: Scroll) -> Snapshot {
        let cols = usize::from(self.size.cols());
        let rows = usize::from(self.size.rows());
        let history = match self.shown {
            Shown::Primary => self.history.len(),
            Shown::Alternate => 0,
        };
        let back = usize::try_from(scroll.0).unwrap_or(usize::MAX).min(history);
        let first = history.saturating_sub(back);
        let mut cells = Vec::with_capacity(cols.saturating_mul(rows));
        for index in first..first.saturating_add(rows) {
            let line = match index.checked_sub(history) {
                None => self.history.line(index),
                Some(row) => self.screen().lines().get(row),
            };
            let printed = line.map_or(&[][..], Line::cells);
            let printed = printed.get(..cols.min(printed.len())).unwrap_or_default();
            // History is reflowed on every width change, so no line is
            // wider than the grid; the cut only guards the slice.
            let row_start = cells.len();
            cells.extend_from_slice(printed);
            cells.resize(row_start.saturating_add(cols), Cell::BLANK);
        }
        let cursor = match self.modes.visibility {
            Visibility::Shown => u16::try_from(back)
                .ok()
                .and_then(|back| self.at.row.checked_add(back))
                .filter(|row| *row < self.size.rows())
                .map(|row| CursorCell {
                    row,
                    col: self.at.col,
                }),
            Visibility::Hidden => None,
        };
        Snapshot {
            size: self.size,
            cells,
            cursor,
            history: u32::try_from(history).unwrap_or(u32::MAX),
            generation: self.generation,
        }
    }

    fn screen(&self) -> &Screen {
        match self.shown {
            Shown::Primary => &self.primary,
            Shown::Alternate => &self.alternate,
        }
    }

    fn screen_mut(&mut self) -> &mut Screen {
        match self.shown {
            Shown::Primary => &mut self.primary,
            Shown::Alternate => &mut self.alternate,
        }
    }

    /// The blank an erase writes: the current background, nothing else
    /// (xterm.js's erase attributes).
    const fn blank(&self) -> Cell {
        Cell::blank(self.pen.erase())
    }

    const fn saved_slot(&mut self) -> &mut Option<Saved> {
        match self.shown {
            Shown::Primary => &mut self.saved_primary,
            Shown::Alternate => &mut self.saved_alternate,
        }
    }

    const fn changed(&mut self) {
        self.generation = self.generation.next();
    }
}
