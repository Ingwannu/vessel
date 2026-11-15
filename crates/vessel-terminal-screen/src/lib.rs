//! One pure terminal interpreter. Actors own bytes/PTY ordering; views own drawing.
//! vt100 is private to this crate. No runtime, filesystem, environment or toolkit.
mod cell;
mod geometry;
mod markers;
mod model;
mod modes;
mod presentation;
mod projection;
mod queries;
mod range;

pub use cell::{
    Attributes, BufferKind, Cell, Color, Column, Cursor, Row, Style, Visibility, Width,
};
pub use geometry::{MAX_SCREEN_CELLS, MIN_COLUMNS, ScrollbackLines, Size};
pub use markers::{
    AnchorProjection, AnchorRelease, AnchorState, BufferEpoch, BufferPresence,
    MAX_VIEWPORT_MARKERS, ViewportAnchor,
};
pub use model::{BytesTaken, FEED_BUDGET, Feed, Screen, Snapshot};
pub use modes::{InputModes, KittyFlags, MAX_KEYBOARD_STACK, ModeState};
pub use presentation::{
    MAX_CONTROL_TOKEN_BYTES, Multiplexer, Presentation, PresentationEvent, PresentationFeed,
    PresentationOffset,
};
pub use queries::{MAX_REPLY_BYTES, Replies, ReplyDuty};
pub use range::{
    Capture, CaptureRange, HistoryLines, LineCount, LineIndex, LineOffset, MAX_CAPTURE_BYTES,
};

/// A screen operation refuses before replacing a valid authority or allocating a reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ScreenError {
    /// Unsupported geometry or more cells than the persistent screen budget.
    #[error("terminal geometry is outside the screen limits")]
    Geometry,
    /// A scrollback preference exceeds the supported validated range.
    #[error("terminal scrollback exceeds its line limit")]
    Scrollback,
    /// The requested rows would exceed the bounded response allocation.
    #[error("terminal capture exceeds its byte budget; request a smaller line range")]
    CaptureTooLarge,
    /// No retained row exists for a supposedly valid projection.
    #[error("terminal screen projection is unavailable")]
    Unavailable,
    /// The bounded reply sink could not admit a query response.
    #[error("terminal query replies exceed their allocation budget")]
    ReplyUnavailable,
    /// A pending control token exceeded the presentation allocation ceiling.
    #[error("terminal presentation exceeds its token or allocation budget")]
    PresentationUnavailable,
    /// Mode negotiation cannot fit its scalar or saved-stack bounds.
    #[error("terminal input mode exceeds its scalar or stack budget")]
    InputModeUnavailable,
    /// A marker cannot point outside retained physical rows.
    #[error("terminal viewport marker row is unavailable")]
    MarkerRow,
    /// Every marker slot still belongs to a caller.
    #[error("terminal viewport marker capacity is exhausted")]
    MarkerCapacity,
    /// A non-reused marker or reset identity cannot be allocated or projected.
    #[error("terminal viewport marker identity is unavailable")]
    MarkerIdentity,
    /// Physical-row narrowing preserves its original conversion error.
    #[error("terminal viewport marker row cannot be represented")]
    MarkerIndex(#[source] std::num::TryFromIntError),
    /// Another authority or reset epoch owns this handle.
    #[error("terminal viewport marker was reset")]
    MarkerReset,
    /// Input for a hidden primary buffer cannot retarget the active alternate buffer.
    #[error("terminal viewport marker belongs to an inactive buffer")]
    MarkerInactive,
    /// Replacement names an already released handle.
    #[error("terminal viewport marker was released")]
    MarkerReleased,
}

#[cfg(test)]
mod tests;
