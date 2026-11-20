//! Counts, cells and the snapshot a screen can draw.
//!
//! The grid's invariants live in the constructors. A cell is plain data: any
//! combination of a glyph and attributes is a cell the kit can paint, and the
//! model is what keeps a wide scalar paired with its continuation.

use std::num::NonZeroU16;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// One PTY read (`vessel-pty`'s read buffer). A feed applies at most this
/// many bytes so one frame cannot parse a whole ring.
pub const FEED_BUDGET: usize = 16_384;

/// Lines of scrollback the model keeps above the screen.
///
/// Paseo's setting: 0 through 1,000,000, default 10,000. The node's ring is
/// a separate, smaller bound; this count is only what the app has already
/// been shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbackLines(u32);

impl ScrollbackLines {
    /// Paseo's default.
    pub const DEFAULT: Self = Self(10_000);

    /// The most a person can ask for, Paseo's maximum.
    pub const MAX: u32 = 1_000_000;

    /// `lines` kept, or a refusal when it is above [`MAX`](Self::MAX).
    ///
    /// # Errors
    ///
    /// [`TerminalError::Scrollback`] when `lines` is above the maximum.
    /// Zero is kept: it means no history above the screen.
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

    /// Whether `lines` is the default, so a preferences file omits it.
    #[must_use]
    pub const fn is_default(lines: &Self) -> bool {
        lines.0 == Self::DEFAULT.0
    }
}

impl Default for ScrollbackLines {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Serialize for ScrollbackLines {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ScrollbackLines {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let lines = u32::deserialize(deserializer)?;
        Self::new(lines).map_err(serde::de::Error::custom)
    }
}

/// A non-zero number of cells along one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellCount(NonZeroU16);

impl CellCount {
    /// `n` cells, or a refusal of zero.
    ///
    /// # Errors
    ///
    /// [`TerminalError::Empty`] when `n` is zero. A terminal of no columns
    /// or no rows cannot place a cursor.
    pub const fn new(n: u16) -> Result<Self, TerminalError> {
        match NonZeroU16::new(n) {
            Some(count) => Ok(Self(count)),
            None => Err(TerminalError::Empty),
        }
    }

    /// The count.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0.get()
    }
}

/// The visible grid, in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSize {
    cols: CellCount,
    rows: CellCount,
}

impl GridSize {
    /// 80×24, the size a program assumes when nobody has resized.
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            cols: CellCount(nonzero(80)),
            rows: CellCount(nonzero(24)),
        }
    }

    /// `cols` by `rows`, both non-zero.
    ///
    /// # Errors
    ///
    /// [`TerminalError::Empty`] when either axis is zero.
    pub const fn new(cols: u16, rows: u16) -> Result<Self, TerminalError> {
        match (CellCount::new(cols), CellCount::new(rows)) {
            (Ok(cols), Ok(rows)) => Ok(Self { cols, rows }),
            (Err(error), _) | (_, Err(error)) => Err(error),
        }
    }

    /// Columns.
    #[must_use]
    pub const fn cols(self) -> u16 {
        self.cols.get()
    }

    /// Rows.
    #[must_use]
    pub const fn rows(self) -> u16 {
        self.rows.get()
    }
}

/// `n` as non-zero. `n` here is a non-zero constant; the `MIN` arm exists
/// so the constructor stays free of `unwrap`.
const fn nonzero(n: u16) -> NonZeroU16 {
    match NonZeroU16::new(n) {
        Some(count) => count,
        None => NonZeroU16::MIN,
    }
}

/// Why a terminal value was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TerminalError {
    /// A column or row count was zero.
    #[error("a terminal size needs columns and rows")]
    Empty,
    /// Scrollback was above [`ScrollbackLines::MAX`].
    #[error("scrollback is above 1000000 lines")]
    Scrollback,
    /// A place was not a cell the grid holds.
    #[error("that cell is not on the grid")]
    Place,
    /// A path paste would not be the path Paseo would type.
    ///
    /// Paseo deletes `` ` $ | & > ~ # ! ^ * ; < `` and pastes the rest.
    /// That is a different path, so the paste is refused instead.
    #[error("the path has a character that would be dropped")]
    Path,
}

/// Foreground or background.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colour {
    /// The palette's default.
    Default,
    /// A 0–255 palette index (0–15 are the ANSI and bright sets).
    Indexed(u8),
    /// A 24-bit colour a program named.
    Rgb(u8, u8, u8),
}

/// Stroke weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Weight {
    /// Not bold.
    #[default]
    Regular,
    /// Bold or intense.
    Bold,
}

/// Upright or italic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Slant {
    /// Upright.
    #[default]
    Roman,
    /// Italic.
    Italic,
}

/// Underline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rule {
    /// None.
    #[default]
    None,
    /// A single underline.
    Single,
}

/// Whether the colours are swapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Polarity {
    /// As named.
    #[default]
    Normal,
    /// Foreground and background swapped.
    Inverse,
}

/// How a cell is drawn, apart from its glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attrs {
    /// Foreground.
    pub fg: Colour,
    /// Background.
    pub bg: Colour,
    /// Weight.
    pub weight: Weight,
    /// Slant.
    pub slant: Slant,
    /// Underline.
    pub rule: Rule,
    /// Polarity.
    pub polarity: Polarity,
}

impl Default for Attrs {
    fn default() -> Self {
        Self {
            fg: Colour::Default,
            bg: Colour::Default,
            weight: Weight::Regular,
            slant: Slant::Roman,
            rule: Rule::None,
            polarity: Polarity::Normal,
        }
    }
}

impl Attrs {
    /// The attributes SGR 0 restores.
    #[must_use]
    pub const fn plain() -> Self {
        Self {
            fg: Colour::Default,
            bg: Colour::Default,
            weight: Weight::Regular,
            slant: Slant::Roman,
            rule: Rule::None,
            polarity: Polarity::Normal,
        }
    }
}

/// What occupies a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// Nothing printed.
    Empty,
    /// A scalar. Its width is 1, or 2 when the next cell is a continuation.
    Scalar(char),
    /// The second column of a wide scalar. Not a character of its own.
    Continuation,
}

/// One cell of the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    glyph: Glyph,
    attrs: Attrs,
}

impl Cell {
    /// An unprinted cell in `attrs` (an erase keeps the current background).
    #[must_use]
    pub const fn blank(attrs: Attrs) -> Self {
        Self {
            glyph: Glyph::Empty,
            attrs,
        }
    }

    /// The glyph.
    #[must_use]
    pub const fn glyph(self) -> Glyph {
        self.glyph
    }

    /// The attributes.
    #[must_use]
    pub const fn attrs(self) -> Attrs {
        self.attrs
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::blank(Attrs::plain())
    }
}

/// Whether the cursor is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Drawn.
    Shown,
    /// Hidden by the program.
    Hidden,
}

/// The cursor, in cells of the visible screen. `col` and `row` are 0-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    /// Column.
    pub col: u16,
    /// Row.
    pub row: u16,
    /// Whether it is drawn.
    pub visibility: Visibility,
}

/// A cell in the whole buffer: history first, then the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
    /// Line from the oldest history line. The screen starts after history.
    pub line: u32,
    /// Column.
    pub col: u16,
}

/// A range of cells, in buffer order. `end` is inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    /// First cell.
    pub start: Place,
    /// Last cell.
    pub end: Place,
}

/// How many times the visible grid has changed.
///
/// A renderer draws only when this differs from the snapshot it last drew.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Generation(u64);

impl Generation {
    /// The next change. It stops at the end of `u64` rather than wrapping,
    /// so a snapshot never looks older than the one before it.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// What one feed did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feed {
    /// Bytes taken from the front of the slice. At most [`FEED_BUDGET`].
    /// The caller keeps the rest.
    pub consumed: usize,
    /// Bytes the program asked the terminal to answer (cursor position,
    /// device attributes). They go out as terminal input, in order with keys.
    pub replies: Vec<u8>,
}

/// The visible grid, for a renderer that does not parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Columns.
    pub cols: u16,
    /// Rows.
    pub rows: u16,
    /// Row-major visible cells, `rows * cols` of them.
    pub cells: Vec<Cell>,
    /// The cursor.
    pub cursor: Cursor,
    /// A selection that lies entirely on the visible screen, in visible
    /// coordinates (line 0 is the top row). History selections are omitted:
    /// they are not on this grid.
    pub selection: Option<Selection>,
    /// Changes when the visible grid or the cursor changes.
    pub generation: Generation,
}

pub(super) fn blank_cells(cols: usize, attrs: Attrs) -> Vec<Cell> {
    let mut cells = Vec::new();
    cells.resize(cols, Cell::blank(attrs));
    cells
}

pub(super) fn scalar_cell(ch: char, attrs: Attrs) -> Cell {
    Cell {
        glyph: Glyph::Scalar(ch),
        attrs,
    }
}

pub(super) fn continuation_cell(attrs: Attrs) -> Cell {
    Cell {
        glyph: Glyph::Continuation,
        attrs,
    }
}
