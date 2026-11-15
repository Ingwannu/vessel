//! Typed borrowed cells hide the engine representation and continuation artifacts.

/// Zero-based row index; the screen checks whether it exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row(pub u16);
/// Zero-based column index; the screen checks whether it exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Column(pub u16);

/// A cell's logical span; a continuation is never another printable glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    /// A normal cell, including an unprinted cell.
    One,
    /// One grapheme occupies this cell and its continuation.
    Two,
    /// The second cell of a wide glyph.
    Continuation,
}
/// A palette reference or direct color; no UI color or theme lives in the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// Terminal default.
    Default,
    /// ANSI/256-color index.
    Indexed(u8),
    /// Explicit red, green, blue.
    Rgb(u8, u8, u8),
}
impl From<vt100::Color> for Color {
    fn from(color: vt100::Color) -> Self {
        match color {
            vt100::Color::Default => Self::Default,
            vt100::Color::Idx(index) => Self::Indexed(index),
            vt100::Color::Rgb(r, g, b) => Self::Rgb(r, g, b),
        }
    }
}
/// Independent attributes, including simultaneous bold and dim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// Bold.
    Bold,
    /// Dim.
    Dim,
    /// Italic.
    Italic,
    /// Underline.
    Underline,
    /// Inverse foreground/background.
    Inverse,
}
impl Style {
    const fn bit(self) -> u8 {
        match self {
            Self::Bold => 1,
            Self::Dim => 2,
            Self::Italic => 4,
            Self::Underline => 8,
            Self::Inverse => 16,
        }
    }
}
/// The lead cell's ink is also the continuation's ink, matching xterm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attributes {
    foreground: Color,
    background: Color,
    styles: u8,
}
impl Attributes {
    pub(crate) fn of(cell: &vt100::Cell) -> Self {
        let mut styles = 0;
        for (on, style) in [
            (cell.bold(), Style::Bold),
            (cell.dim(), Style::Dim),
            (cell.italic(), Style::Italic),
            (cell.underline(), Style::Underline),
            (cell.inverse(), Style::Inverse),
        ] {
            if on {
                styles |= style.bit();
            }
        }
        Self {
            foreground: cell.fgcolor().into(),
            background: cell.bgcolor().into(),
            styles,
        }
    }
    /// Foreground color or default.
    #[must_use]
    pub const fn foreground(self) -> Color {
        self.foreground
    }
    /// Background color or default.
    #[must_use]
    pub const fn background(self) -> Color {
        self.background
    }
    /// Whether a named independent style is active.
    #[must_use]
    pub const fn has(self, style: Style) -> bool {
        self.styles & style.bit() != 0
    }
}
/// A borrowed cell; no vt100 type or independent text interpretation escapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell<'a> {
    text: &'a str,
    width: Width,
    attributes: Attributes,
}
impl<'a> Cell<'a> {
    pub(crate) fn of(body: &'a vt100::Cell, lead: &vt100::Cell) -> Self {
        let width = if body.is_wide_continuation() {
            Width::Continuation
        } else if body.is_wide() {
            Width::Two
        } else {
            Width::One
        };
        Self {
            text: body.contents(),
            width,
            attributes: Attributes::of(lead),
        }
    }
    /// Original unnormalized grapheme text; empty for an unprinted/continuation cell.
    #[must_use]
    pub const fn text(self) -> &'a str {
        self.text
    }
    /// Logical width in cells.
    #[must_use]
    pub const fn width(self) -> Width {
        self.width
    }
    /// Attributes inherited from the lead for a continuation.
    #[must_use]
    pub const fn attributes(self) -> Attributes {
        self.attributes
    }
}

/// Whether the active cursor is shown by the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// DECTCEM enabled.
    Shown,
    /// DECTCEM disabled.
    Hidden,
}
/// Active engine buffer, independent from a UI pane or workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferKind {
    /// Primary buffer and its history.
    Primary,
    /// Alternate buffer, without primary history.
    Alternate,
}
/// Engine cursor state; a pending-wrap column can be immediately after the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    /// Zero-based row on the live screen.
    pub row: Row,
    /// Zero-based column, including pending wrap.
    pub column: Column,
    /// Program-requested visibility, never presentation/blink state.
    pub visibility: Visibility,
}
