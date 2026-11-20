//! The cell buffer: cursor, scroll, wrap and resize.
//!
//! Bytes are decoded in [`parse`]. This file places cells, so a test can
//! see a scalar without drawing it.

mod edit;
mod find;
mod parse;
mod reflow;

use std::collections::VecDeque;
use std::fmt;

pub use find::{FindBound, FindView};

use super::keys::CursorKeys;
use super::paste::Bracketed;
use super::types::{
    Attrs, Cell, Cursor, FEED_BUDGET, Feed, Generation, GridSize, Place, ScrollbackLines,
    Selection, Snapshot, TerminalError, Visibility,
};
use find::FindState;

/// Whether the primary screen or the alternate screen is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shown {
    Primary,
    Alternate,
}

/// Whether a query in this feed may be typed back. Replay never may.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    /// The viewer [`super::super::duty::query_duty`] named.
    Live,
    /// History, or any viewer that must not answer.
    Replay,
}

/// One row of cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Line {
    pub(super) cells: Vec<Cell>,
    /// The cursor wrapped off this line, so a resize joins it with the next.
    pub(super) wrapped: bool,
}

#[derive(Debug, Clone, Copy)]
struct Saved {
    cursor: Cursor,
    pending_wrap: bool,
}

/// A terminal grid. The parser and the key encoder sit beside it.
pub struct TerminalModel {
    size: GridSize,
    scrollback: ScrollbackLines,
    history: VecDeque<Line>,
    primary: Vec<Line>,
    alternate: Vec<Line>,
    shown: Shown,
    cursor: Cursor,
    pending_wrap: bool,
    saved: Option<Saved>,
    attrs: Attrs,
    cursor_keys: CursorKeys,
    bracketed: Bracketed,
    selection: Option<Selection>,
    find: FindState,
    generation: Generation,
    /// Set by the first change in a feed, so one feed bumps [`Generation`] once.
    dirty: bool,
    parser: vte::Parser,
}

impl fmt::Debug for TerminalModel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The parser holds a partial UTF-8 sequence. That is not a cell.
        formatter
            .debug_struct("TerminalModel")
            .field("size", &self.size)
            .field("scrollback", &self.scrollback)
            .field("history", &self.history)
            .field("primary", &self.primary)
            .field("alternate", &self.alternate)
            .field("shown", &self.shown)
            .field("cursor", &self.cursor)
            .field("pending_wrap", &self.pending_wrap)
            .field("saved", &self.saved)
            .field("attrs", &self.attrs)
            .field("cursor_keys", &self.cursor_keys)
            .field("bracketed", &self.bracketed)
            .field("selection", &self.selection)
            .field("find", &self.find)
            .field("generation", &self.generation)
            .field("dirty", &self.dirty)
            .field("parser", &"partial sequence")
            .finish()
    }
}

impl TerminalModel {
    /// An empty grid of `size`, keeping `scrollback` lines above it.
    #[must_use]
    pub fn new(size: GridSize, scrollback: ScrollbackLines) -> Self {
        let primary = blank_screen(size);
        let alternate = blank_screen(size);
        Self {
            size,
            scrollback,
            history: VecDeque::new(),
            primary,
            alternate,
            shown: Shown::Primary,
            cursor: home_cursor(),
            pending_wrap: false,
            saved: None,
            attrs: Attrs::plain(),
            cursor_keys: CursorKeys::Normal,
            bracketed: Bracketed::Off,
            selection: None,
            find: FindState::default(),
            generation: Generation::default(),
            dirty: false,
            parser: vte::Parser::new(),
        }
    }

    /// Apply at most one read of live PTY bytes. Replies are returned.
    ///
    /// The link sends them only for the viewer [`super::super::duty::query_duty`]
    /// returns [`super::super::duty::QueryDuty::Answer`] for. Any other
    /// viewer uses [`Self::replay`].
    #[must_use]
    pub fn feed(&mut self, bytes: &[u8]) -> Feed {
        self.apply(bytes, Answer::Live)
    }

    /// Apply history, or a viewer's bytes that must not be typed back.
    ///
    /// A query in the bytes updates the grid and produces no reply, so a
    /// replay cannot write a cursor report into the program.
    #[must_use]
    pub fn replay(&mut self, bytes: &[u8]) -> Feed {
        self.apply(bytes, Answer::Replay)
    }

    fn apply(&mut self, bytes: &[u8], answer: Answer) -> Feed {
        let consumed = bytes.len().min(FEED_BUDGET);
        let slice = bytes.get(..consumed).unwrap_or(bytes);
        let mut parser = std::mem::replace(&mut self.parser, vte::Parser::new());
        let replies = {
            let mut step = parse::Step {
                model: self,
                replies: Vec::new(),
            };
            parser.advance(&mut step, slice);
            match answer {
                Answer::Live => step.replies,
                Answer::Replay => Vec::new(),
            }
        };
        self.parser = parser;
        self.settle();
        Feed { consumed, replies }
    }

    /// The visible grid. [`Snapshot::generation`] changes only when it does.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        let mut cells = Vec::new();
        for line in self.screen() {
            cells.extend(line.cells.iter().copied());
        }
        Snapshot {
            cols: self.cols(),
            rows: self.rows(),
            cells,
            cursor: self.cursor,
            selection: self.visible_selection(),
            generation: self.generation,
        }
    }

    /// Remember a selection, or refuse a place the buffer does not hold.
    ///
    /// # Errors
    ///
    /// [`TerminalError::Place`] when either end is off the buffer.
    pub fn select(&mut self, start: Place, end: Place) -> Result<(), TerminalError> {
        if !self.holds(start) || !self.holds(end) {
            return Err(TerminalError::Place);
        }
        self.selection = Some(Selection { start, end });
        self.mark();
        self.settle();
        Ok(())
    }

    /// Forget the selection.
    pub fn clear_selection(&mut self) {
        if self.selection.take().is_some() {
            self.mark();
        }
        self.settle();
    }

    /// Resize. The primary screen's soft-wrapped lines reflow, including
    /// while the alternate screen is showing, so leaving it does not bring
    /// back the old width. The alternate screen is only clipped or padded,
    /// and nothing of it is written into scrollback.
    pub fn resize(&mut self, size: GridSize) {
        self.size = size;
        self.reflow_primary(size);
        self.alternate = reflow::resize_plain(&self.alternate, size);
        self.clamp_cursor();
        self.mark();
        self.settle();
    }

    pub(super) const fn cols(&self) -> u16 {
        self.size.cols()
    }

    pub(super) const fn rows(&self) -> u16 {
        self.size.rows()
    }

    pub(super) const fn cursor_keys(&self) -> CursorKeys {
        self.cursor_keys
    }

    pub(super) const fn bracketed(&self) -> Bracketed {
        self.bracketed
    }

    pub(super) fn set_cursor_keys(&mut self, keys: CursorKeys) {
        self.cursor_keys = keys;
    }

    pub(super) fn set_bracketed(&mut self, mode: Bracketed) {
        self.bracketed = mode;
    }

    fn holds(&self, place: Place) -> bool {
        let lines = self.history.len().saturating_add(usize::from(self.rows()));
        let Some(line) = u32::try_from(lines).ok() else {
            return false;
        };
        place.line < line && place.col < self.cols()
    }

    fn visible_selection(&self) -> Option<Selection> {
        let selection = self.selection?;
        let history = u32::try_from(self.history.len()).unwrap_or(u32::MAX);
        let rows = u32::from(self.rows());
        let start = visible_place(selection.start, history, rows)?;
        let end = visible_place(selection.end, history, rows)?;
        Some(Selection { start, end })
    }

    fn screen(&self) -> &[Line] {
        match self.shown {
            Shown::Primary => &self.primary,
            Shown::Alternate => &self.alternate,
        }
    }

    fn screen_mut(&mut self) -> &mut Vec<Line> {
        match self.shown {
            Shown::Primary => &mut self.primary,
            Shown::Alternate => &mut self.alternate,
        }
    }

    fn clamp_cursor(&mut self) {
        let max_row = self.rows().saturating_sub(1);
        let max_col = self.cols().saturating_sub(1);
        if self.cursor.row > max_row {
            self.cursor.row = max_row;
        }
        if self.cursor.col > max_col {
            self.cursor.col = max_col;
        }
    }

    fn mark(&mut self) {
        if self.dirty {
            return;
        }
        self.generation = self.generation.next();
        self.dirty = true;
    }

    fn settle(&mut self) {
        self.dirty = false;
    }
}

fn home_cursor() -> Cursor {
    Cursor {
        col: 0,
        row: 0,
        visibility: Visibility::Shown,
    }
}

pub(super) fn blank_screen(size: GridSize) -> Vec<Line> {
    let mut lines = Vec::new();
    lines.resize(usize::from(size.rows()), blank_line(size.cols()));
    lines
}

pub(super) fn blank_line(cols: u16) -> Line {
    Line {
        cells: super::types::blank_cells(usize::from(cols), Attrs::plain()),
        wrapped: false,
    }
}

fn visible_place(place: Place, history: u32, rows: u32) -> Option<Place> {
    if place.line < history {
        return None;
    }
    let row = place.line.saturating_sub(history);
    if row >= rows {
        return None;
    }
    let row = u16::try_from(row).ok()?;
    Some(Place {
        line: u32::from(row),
        col: place.col,
    })
}
