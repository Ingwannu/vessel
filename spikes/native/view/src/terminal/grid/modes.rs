//! The modes a program sets, each its own two-valued type (no `bool`
//! crosses the grid's boundary), and the cursor they move.

use super::pen::Pen;

/// IRM: whether a printed character replaces the cell or pushes the line
/// right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Insert {
    #[default]
    Replace,
    Insert,
}

/// LNM: whether a line feed also returns the carriage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum LineFeed {
    #[default]
    Down,
    DownAndReturn,
}

/// DECOM: whether rows are counted from the screen's top or the scroll
/// region's, and whether the cursor may leave the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Origin {
    #[default]
    Screen,
    Region,
}

/// DECAWM: whether printing at the last column wraps onto the next line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum AutoWrap {
    #[default]
    Wrap,
    Stay,
}

/// DECCKM: how the arrow keys are encoded (the input slice reads it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorKeys {
    /// `ESC [ A`.
    #[default]
    Normal,
    /// `ESC O A`.
    Application,
}

/// Bracketed paste (`?2004`): whether a paste is wrapped in markers (the
/// input slice reads it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Bracketed {
    /// Pasted as typed.
    #[default]
    Off,
    /// Wrapped in `ESC [ 200 ~` and `ESC [ 201 ~`.
    On,
}

/// Whether the cursor is drawn (DECTCEM).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    /// Drawn.
    #[default]
    Shown,
    /// Hidden by the program.
    Hidden,
}

/// Which screen is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shown {
    /// The primary screen, with history.
    #[default]
    Primary,
    /// The alternate screen (full-screen programs), with none.
    Alternate,
}

/// Every mode, at its reset value by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct Modes {
    pub(super) insert: Insert,
    pub(super) line_feed: LineFeed,
    pub(super) origin: Origin,
    pub(super) auto_wrap: AutoWrap,
    pub(super) cursor_keys: CursorKeys,
    pub(super) bracketed: Bracketed,
    pub(super) visibility: Visibility,
}

/// Whether the next printed character first wraps to the next line: set
/// by a print at the last column (DEC's "last column flag"), so the
/// cursor stays on that column until something is printed after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Pending {
    #[default]
    Nothing,
    Wrap,
}

/// Where the cursor is: 0-based, always on the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct At {
    pub(super) row: u16,
    pub(super) col: u16,
    pub(super) pending: Pending,
}

/// What DECSC keeps and DECRC restores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct Saved {
    pub(super) at: At,
    pub(super) origin: Origin,
    pub(super) auto_wrap: AutoWrap,
    pub(super) pen: Pen,
}
