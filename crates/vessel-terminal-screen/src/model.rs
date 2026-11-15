//! One owner feeds the engine; immutable snapshots survive actor exit without log replay.
use crate::{
    BufferKind, Capture, CaptureRange, Cell, Column, Cursor, Row, ScreenError, ScrollbackLines,
    Size, Visibility,
};
use std::sync::Arc;

/// Maximum parsing work requested from one pure feed call; the actor decides when to yield.
pub const FEED_BUDGET: usize = 16 * 1024;
/// Bytes consumed from the front of one output slice, never a record sequence or row count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BytesTaken(usize);
impl BytesTaken {
    /// Prefix length; zero only for empty input.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}
/// A bounded feed's progress; no actor, transport or UI scheduling occurs here.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Feed {
    /// Prefix consumed by this call.
    pub consumed: BytesTaken,
}
/// Mutable screen authority, owned by exactly one terminal output actor.
pub struct Screen {
    parser: vt100::Parser,
    size: Size,
    requested: ScrollbackLines,
    retained_columns: u16,
}
impl std::fmt::Debug for Screen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Screen")
            .field("size", &self.size)
            .field("requested", &self.requested)
            .finish_non_exhaustive()
    }
}
impl Screen {
    /// Allocate a bounded, empty terminal.
    ///
    /// # Errors
    /// Returns [`ScreenError::Geometry`] if both screens cannot fit the cell budget.
    pub fn new(size: Size, scrollback: ScrollbackLines) -> Result<Self, ScreenError> {
        let capacity = size.capacity(scrollback)?;
        let (rows, cols) = size.engine();
        Ok(Self {
            parser: vt100::Parser::new(rows, cols, capacity),
            size,
            requested: scrollback,
            retained_columns: size.cols(),
        })
    }
    /// Interpret one bounded prefix. Ordered actors repeatedly feed the unconsumed suffix.
    pub fn feed(&mut self, bytes: &[u8]) -> Feed {
        let consumed = bytes.len().min(FEED_BUDGET);
        if let Some(prefix) = bytes.get(..consumed) {
            self.parser.process(prefix);
        }
        Feed {
            consumed: BytesTaken(consumed),
        }
    }
    /// Current visible geometry.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }
    /// Resize both engine screens, retaining hidden columns rather than replaying ANSI.
    ///
    /// # Errors
    /// Returns [`ScreenError::Geometry`] before mutation when retained cells cannot fit.
    pub fn resize(&mut self, size: Size) -> Result<(), ScreenError> {
        let retained_columns = self.retained_columns.max(size.cols());
        let budget = Size::new(retained_columns, size.rows())?;
        let capacity = budget.capacity(self.requested)?;
        self.parser.set_scrollback_capacity(capacity);
        let (rows, cols) = size.engine();
        self.parser.set_size(rows, cols);
        self.size = size;
        self.retained_columns = retained_columns;
        Ok(())
    }
    /// Change retention without clearing rows which remain inside the bound.
    ///
    /// # Errors
    /// Returns [`ScreenError::Geometry`] without changing the setting if it cannot fit.
    pub fn set_scrollback(&mut self, limit: ScrollbackLines) -> Result<(), ScreenError> {
        let budget = Size::new(self.retained_columns, self.size.rows())?;
        self.parser.set_scrollback_capacity(budget.capacity(limit)?);
        self.requested = limit;
        Ok(())
    }
    /// Cursor state comes from the engine, independently of viewport presentation.
    #[must_use]
    pub fn cursor(&self) -> Cursor {
        cursor(self.parser.screen())
    }
    /// Which engine buffer is active.
    #[must_use]
    pub fn buffer_kind(&self) -> BufferKind {
        buffer_kind(self.parser.screen())
    }
    /// Physical primary history currently retained (zero in the alternate buffer).
    #[must_use]
    pub fn history_len(&self) -> usize {
        self.parser.screen().scrollback_len()
    }

    /// A borrowed visible cell with the lead cell's continuation attributes.
    #[must_use]
    pub fn cell(&self, row: Row, column: Column) -> Option<Cell<'_>> {
        crate::projection::cell(self.parser.screen(), row, column)
    }
    /// Translate retained physical rows; no clone or mutation of the live engine.
    ///
    /// # Errors
    /// Returns [`ScreenError::CaptureTooLarge`] before returning an oversized response.
    pub fn capture(&self, range: CaptureRange) -> Result<Capture, ScreenError> {
        crate::projection::capture(self.parser.screen(), range)
    }
    /// Retain final state independently from a pruned or closed event ring.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            state: Arc::new(self.parser.screen().clone()),
            size: self.size,
        }
    }
}
/// Read-only retained state; cloned handles share the same immutable engine snapshot.
#[derive(Debug, Clone)]
pub struct Snapshot {
    state: Arc<vt100::Screen>,
    size: Size,
}
impl Snapshot {
    /// Retained geometry.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }
    /// Retained visible cells, without leaking the mutable engine API.
    #[must_use]
    pub fn cell(&self, row: Row, column: Column) -> Option<Cell<'_>> {
        crate::projection::cell(&self.state, row, column)
    }
    /// Capture remains available after the mutable authority has exited.
    ///
    /// # Errors
    /// Returns the same bounded projection errors as [`Screen::capture`].
    pub fn capture(&self, range: CaptureRange) -> Result<Capture, ScreenError> {
        crate::projection::capture(&self.state, range)
    }
}

fn cursor(screen: &vt100::Screen) -> Cursor {
    let (row, column) = screen.cursor_position();
    Cursor {
        row: Row(row),
        column: Column(column),
        visibility: if screen.hide_cursor() {
            Visibility::Hidden
        } else {
            Visibility::Shown
        },
    }
}
fn buffer_kind(screen: &vt100::Screen) -> BufferKind {
    if screen.alternate_screen() {
        BufferKind::Alternate
    } else {
        BufferKind::Primary
    }
}
