//! The terminal's counts as types (ADR 0073's contract): the grid's size
//! and how many lines of history it keeps.
//!
//! [Decision Log]
//! - Purpose: a size reaches the grid from a window (cells that fit) or
//!   from a node's record of a terminal; neither is trusted to be sane.
//! - Constraint: each screen holds `cols × rows` cells, and the app keeps
//!   two screens (primary and alternate), so the largest size bounds the
//!   memory a terminal can take before any byte is read.
//! - Choice: zero on either axis is refused ([`TerminalError::Empty`]), and
//!   so is more than [`GridSize::MAX_COLS`] by [`GridSize::MAX_ROWS`]
//!   ([`TerminalError::TooLarge`]): a 4K screen at the smallest code face
//!   is about 550 columns and 150 rows, so the bounds leave room and cap a
//!   screen at 512 Ki cells.
//! - Alternative: no upper bound, as xterm.js (a size from a node could then
//!   ask for 65,535 × 65,535 cells); clamping silently (the caller would not
//!   know the grid is smaller than the window it measured).

use std::num::NonZeroU16;

/// Lines of history kept above the screen, validated against [`Self::MAX`].
/// The grid also caps history by cells (`grid::history`), whichever bound comes first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct ScrollbackLines(u32);

impl ScrollbackLines {
    /// Paseo's default.
    pub const DEFAULT: Self = Self(10_000);

    /// The most a person can ask for (Paseo's maximum).
    pub const MAX: u32 = 1_000_000;

    /// `lines`, or a refusal above [`MAX`](Self::MAX). Zero keeps no
    /// history.
    ///
    /// # Errors
    ///
    /// [`TerminalError::Scrollback`] when `lines` is above the maximum.
    pub const fn new(lines: u32) -> Result<Self, TerminalError> {
        if lines > Self::MAX {
            Err(TerminalError::Scrollback)
        } else {
            Ok(Self(lines))
        }
    }

    /// The count.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Default for ScrollbackLines {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl TryFrom<u32> for ScrollbackLines {
    type Error = TerminalError;

    fn try_from(lines: u32) -> Result<Self, Self::Error> {
        Self::new(lines)
    }
}

impl From<ScrollbackLines> for u32 {
    fn from(lines: ScrollbackLines) -> Self {
        lines.get()
    }
}

/// The visible grid, in cells: both axes non-zero and within the bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSize {
    cols: NonZeroU16,
    rows: NonZeroU16,
}

impl GridSize {
    /// The most columns a grid has.
    pub const MAX_COLS: u16 = 1024;

    /// The most rows a grid has.
    pub const MAX_ROWS: u16 = 512;

    /// 80 × 24, the size a program assumes when nobody has resized.
    pub const STANDARD: Self = Self {
        cols: non_zero(80),
        rows: non_zero(24),
    };

    /// `cols` by `rows`.
    ///
    /// # Errors
    ///
    /// [`TerminalError::Empty`] when either is zero (no cell can hold the
    /// cursor); [`TerminalError::TooLarge`] above the bounds.
    pub const fn new(cols: u16, rows: u16) -> Result<Self, TerminalError> {
        let (Some(cols), Some(rows)) = (NonZeroU16::new(cols), NonZeroU16::new(rows)) else {
            return Err(TerminalError::Empty);
        };
        if cols.get() > Self::MAX_COLS || rows.get() > Self::MAX_ROWS {
            return Err(TerminalError::TooLarge);
        }
        Ok(Self { cols, rows })
    }

    /// `cols` by `rows` as a node reported them, drawn at the bounds when
    /// larger (the wire holds `u16`; the grid cannot be larger, and the
    /// program then writes past what is drawn).
    ///
    /// # Errors
    ///
    /// [`TerminalError::Empty`] when either is zero.
    pub const fn within(cols: u16, rows: u16) -> Result<Self, TerminalError> {
        let cols = if cols > Self::MAX_COLS {
            Self::MAX_COLS
        } else {
            cols
        };
        let rows = if rows > Self::MAX_ROWS {
            Self::MAX_ROWS
        } else {
            rows
        };
        Self::new(cols, rows)
    }

    /// Columns, at least 1.
    #[must_use]
    pub const fn cols(self) -> u16 {
        self.cols.get()
    }

    /// Rows, at least 1.
    #[must_use]
    pub const fn rows(self) -> u16 {
        self.rows.get()
    }

    /// The last column (0-based).
    #[must_use]
    pub const fn last_col(self) -> u16 {
        self.cols.get().saturating_sub(1)
    }

    /// The last row (0-based).
    #[must_use]
    pub const fn last_row(self) -> u16 {
        self.rows.get().saturating_sub(1)
    }
}

/// `n`, a non-zero constant; the `MIN` arm keeps the constant free of
/// `unwrap`.
const fn non_zero(n: u16) -> NonZeroU16 {
    match NonZeroU16::new(n) {
        Some(n) => n,
        None => NonZeroU16::MIN,
    }
}

impl Default for GridSize {
    fn default() -> Self {
        Self::STANDARD
    }
}

/// Why a terminal value was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TerminalError {
    /// A column or row count was zero.
    #[error("a terminal needs at least one column and one row")]
    Empty,
    /// A size above the grid's bounds.
    #[error(
        "a terminal is at most {} columns by {} rows",
        GridSize::MAX_COLS,
        GridSize::MAX_ROWS
    )]
    TooLarge,
    /// Scrollback above [`ScrollbackLines::MAX`].
    #[error("scrollback is at most {} lines", ScrollbackLines::MAX)]
    Scrollback,
}
