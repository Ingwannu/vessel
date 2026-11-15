//! Borrow selected cells atomically; actors own frame revisions and wire allocation limits.
use crate::markers::{Epochs, ScreenIdentity};
use crate::{
    AnchorProjection, AnchorState, BufferEpoch, BufferKind, BufferPresence, Cell, Column, Cursor,
    InputModes, LineIndex, Row, Screen, ScreenError, Size, Snapshot, ViewportAnchor,
};

/// Requested prefix height of the selected viewport; this never resizes the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameRows(u16);
impl FrameRows {
    /// Parse once at the read edge, before projecting or allocating cells.
    ///
    /// # Errors
    /// Refuses zero-height frames.
    pub const fn new(rows: u16) -> Result<Self, ScreenError> {
        if rows == 0 {
            Err(ScreenError::Geometry)
        } else {
            Ok(Self(rows))
        }
    }
    /// Actual returned rows may be lower when the requested viewport exceeds the screen.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}
/// Cell cardinality supports wire-budget admission before any per-cell DTO allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameCells(u32);
impl FrameCells {
    /// Multiply by the consumer's minimum encoded cell cost before a full cell walk.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
/// A pinned handle is only borrowed for selection; it does not become frame authority.
#[derive(Debug, Clone, Copy)]
pub enum ViewportRequest<'a> {
    /// Read the moving live viewport; the node releases its former pinned view slot.
    FollowLive,
    /// Preserve the inspected physical marker until actual eviction or reset.
    Pinned(&'a ViewportAnchor),
}
/// A response states why its actual first row differs from the caller's pinned row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportAdjustment {
    /// The retained marker supplied the viewport unchanged.
    Unchanged,
    /// An actually evicted row clamps to the oldest retained row.
    Evicted,
    /// A reset invalidated the old identity; show live with that explicit reset reason.
    Reset,
    /// The other buffer is active; preserve this handle while showing that buffer live.
    Inactive,
    /// A marker within live rows cannot move the viewport below its live origin.
    BeyondLive,
}
/// Requested marker disposition and actual window remain separate and explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameOrigin {
    /// Live output was explicitly requested.
    FollowLive,
    /// The frame reports the current marker state even when another buffer is displayed.
    Pinned {
        /// Availability and retention of the requested marker.
        projection: AnchorProjection,
        /// Why the actual frame starts where it does.
        adjustment: ViewportAdjustment,
    },
}
/// Program cursor position/visibility projected into the selected display rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameCursor {
    /// Cursor lies outside this inspected viewport.
    OutsideViewport,
    /// Position is relative to this frame; shape and timer policy belong to the renderer.
    Present(Cursor),
}

#[derive(Clone, Copy)]
struct Source<'a> {
    state: &'a vt100::Screen,
    identity: &'a ScreenIdentity,
    epochs: Epochs,
    size: Size,
    modes: InputModes,
}
/// A cell grid is borrowed, never cloned before the consumer admits its encoded budget.
pub struct FrameView<'a> {
    source: Source<'a>,
    first: LineIndex,
    rows: FrameRows,
    cells: FrameCells,
    origin: FrameOrigin,
    cursor: FrameCursor,
}
impl std::fmt::Debug for FrameView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameView")
            .field("first", &self.first)
            .field("rows", &self.rows)
            .field("cells", &self.cells)
            .field("origin", &self.origin)
            .finish_non_exhaustive()
    }
}
impl FrameView<'_> {
    /// Actual first retained row; never inferred from event-log bytes.
    #[must_use]
    pub const fn first(&self) -> LineIndex {
        self.first
    }
    /// Effective display height, bounded by the current terminal geometry.
    #[must_use]
    pub const fn rows(&self) -> FrameRows {
        self.rows
    }
    /// Count before per-cell allocation; encoded wire-byte checks remain the node's job.
    #[must_use]
    pub const fn cells(&self) -> FrameCells {
        self.cells
    }
    /// Current visible width, excluding latent alternate-buffer columns.
    #[must_use]
    pub const fn columns(&self) -> u16 {
        self.source.size.cols()
    }
    /// Requested and effective viewport reasons accompany each frame.
    #[must_use]
    pub const fn origin(&self) -> FrameOrigin {
        self.origin
    }
    /// Current foreground input negotiation comes from this same read cut.
    #[must_use]
    pub const fn input_modes(&self) -> InputModes {
        self.source.modes
    }
    /// Position/visibility projected into this frame, independently of blink policy.
    #[must_use]
    pub const fn cursor(&self) -> FrameCursor {
        self.cursor
    }
    /// The active buffer supplies cells even while a primary anchor is inactive.
    #[must_use]
    pub fn buffer_kind(&self) -> BufferKind {
        crate::model::buffer_kind(self.source.state)
    }
    /// Reset identity belongs to the displayed buffer, not a route or transport lease.
    #[must_use]
    pub fn epoch(&self) -> BufferEpoch {
        self.source.epochs.of(self.buffer_kind())
    }
    /// Borrow only a selected visible cell, retaining the lead cell's continuation ink.
    #[must_use]
    pub fn cell(&self, row: Row, column: Column) -> Option<Cell<'_>> {
        if row.0 >= self.rows.0 || column.0 >= self.columns() {
            return None;
        }
        let line = self.first.0.checked_add(u32::from(row.0))?;
        crate::projection::frame_cell(self.source.state, LineIndex(line), column)
    }
}
impl Screen {
    /// Borrow an authoritative viewport before allocating any selected wire cells.
    ///
    /// # Errors
    /// Refuses released markers or an unrepresentable physical row.
    pub fn frame(
        &self,
        viewport: ViewportRequest<'_>,
        rows: FrameRows,
    ) -> Result<FrameView<'_>, ScreenError> {
        project(
            Source {
                state: self.parser.screen(),
                identity: &self.identity,
                epochs: self.parser.callbacks().epochs,
                size: self.size(),
                modes: self.input_modes(),
            },
            viewport,
            rows,
        )
    }
}
impl Snapshot {
    /// Project the same final immutable cell/mode/marker cut after actor exit.
    ///
    /// # Errors
    /// Refuses released markers or an unrepresentable physical row.
    pub fn frame(
        &self,
        viewport: ViewportRequest<'_>,
        rows: FrameRows,
    ) -> Result<FrameView<'_>, ScreenError> {
        project(
            Source {
                state: &self.state,
                identity: &self.identity,
                epochs: self.epochs,
                size: self.size(),
                modes: self.input_modes(),
            },
            viewport,
            rows,
        )
    }
}
fn project<'a>(
    source: Source<'a>,
    viewport: ViewportRequest<'_>,
    requested: FrameRows,
) -> Result<FrameView<'a>, ScreenError> {
    let live =
        LineIndex(u32::try_from(source.state.scrollback_len()).map_err(ScreenError::MarkerIndex)?);
    let (first, origin) = match viewport {
        ViewportRequest::FollowLive => (live, FrameOrigin::FollowLive),
        ViewportRequest::Pinned(anchor) => {
            let projection =
                crate::markers::resolve(source.state, source.identity, source.epochs, anchor)?;
            let (first, adjustment) = select(projection, live)?;
            (
                first,
                FrameOrigin::Pinned {
                    projection,
                    adjustment,
                },
            )
        }
    };
    // Both values are validated nonzero; limiting height cannot create an empty window.
    let rows = FrameRows(requested.0.min(source.size.rows()));
    let cells = FrameCells(
        u32::from(rows.0)
            .checked_mul(u32::from(source.size.cols()))
            .ok_or(ScreenError::Geometry)?,
    );
    let cursor = projected_cursor(source.state, live, first, rows)?;
    Ok(FrameView {
        source,
        first,
        rows,
        cells,
        origin,
        cursor,
    })
}
fn select(
    projection: AnchorProjection,
    live: LineIndex,
) -> Result<(LineIndex, ViewportAdjustment), ScreenError> {
    match projection.state {
        AnchorState::Released => Err(ScreenError::MarkerReleased),
        AnchorState::Reset => Ok((live, ViewportAdjustment::Reset)),
        AnchorState::Retained(_) | AnchorState::Evicted { .. }
            if projection.presence == BufferPresence::Inactive =>
        {
            Ok((live, ViewportAdjustment::Inactive))
        }
        AnchorState::Evicted { oldest } => Ok((oldest, ViewportAdjustment::Evicted)),
        AnchorState::Retained(row) if row.0 > live.0 => Ok((live, ViewportAdjustment::BeyondLive)),
        AnchorState::Retained(row) => Ok((row, ViewportAdjustment::Unchanged)),
    }
}
fn projected_cursor(
    state: &vt100::Screen,
    live: LineIndex,
    first: LineIndex,
    rows: FrameRows,
) -> Result<FrameCursor, ScreenError> {
    let cursor = crate::model::cursor(state);
    let at = live
        .0
        .checked_add(u32::from(cursor.row.0))
        .ok_or(ScreenError::Geometry)?;
    let Some(relative) = at
        .checked_sub(first.0)
        .filter(|row| *row < u32::from(rows.0))
    else {
        return Ok(FrameCursor::OutsideViewport);
    };
    Ok(FrameCursor::Present(Cursor {
        row: Row(u16::try_from(relative).map_err(ScreenError::MarkerIndex)?),
        ..cursor
    }))
}

#[cfg(test)]
mod tests;
