//! The pen: how the program asked a cell to be drawn, apart from its
//! glyph. Each attribute is its own type (no `bool`); the kit's colours
//! are resolved from these by `terminal::paint`.

/// A colour as a program named it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Colour {
    /// The palette's foreground or background.
    #[default]
    Default,
    /// A palette index: 0–7 the ANSI colours, 8–15 their bright forms,
    /// 16–231 the 6 × 6 × 6 cube, 232–255 the grey ramp.
    Indexed(u8),
    /// A 24-bit colour.
    Rgb(Rgb),
}

/// Red, green and blue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
}

/// Stroke weight (SGR 1, 2, 22).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Weight {
    /// Neither bold nor faint.
    #[default]
    Regular,
    /// Bold.
    Bold,
    /// Faint (drawn at half opacity, as xterm.js does).
    Faint,
}

/// Upright or italic (SGR 3, 23).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Slant {
    /// Upright.
    #[default]
    Upright,
    /// Italic.
    Italic,
}

/// The underline (SGR 4 with its sub-parameter, 21, 24).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Underline {
    /// None.
    #[default]
    None,
    /// One line.
    Single,
    /// Two lines (`4:2`, `21`).
    Double,
    /// A wave (`4:3`).
    Curly,
    /// Dots (`4:4`).
    Dotted,
    /// Dashes (`4:5`).
    Dashed,
}

/// Whether foreground and background are swapped (SGR 7, 27).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Polarity {
    /// As named.
    #[default]
    Normal,
    /// Swapped.
    Inverse,
}

/// Whether the glyph is drawn (SGR 8, 28).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Concealment {
    /// Drawn.
    #[default]
    Shown,
    /// Not drawn; the cell's background still is.
    Concealed,
}

/// A line through the glyph (SGR 9, 29).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Strike {
    /// None.
    #[default]
    None,
    /// Struck through.
    Through,
}

/// Every attribute a printed cell carries. Blink (SGR 5, 6), overline
/// (53) and the underline's own colour (58) are read and dropped, a
/// stated difference: xterm.js (Paseo's terminal) draws all three, but
/// blink needs a timer over the cells and an underline colour adds four
/// bytes to every cell history keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pen {
    /// Foreground.
    pub fg: Colour,
    /// Background.
    pub bg: Colour,
    /// Weight.
    pub weight: Weight,
    /// Slant.
    pub slant: Slant,
    /// Underline.
    pub underline: Underline,
    /// Polarity.
    pub polarity: Polarity,
    /// Concealment.
    pub concealment: Concealment,
    /// Strike.
    pub strike: Strike,
}

impl Pen {
    /// SGR 0's pen: every attribute at its default.
    pub const PLAIN: Self = Self {
        fg: Colour::Default,
        bg: Colour::Default,
        weight: Weight::Regular,
        slant: Slant::Upright,
        underline: Underline::None,
        polarity: Polarity::Normal,
        concealment: Concealment::Shown,
        strike: Strike::None,
    };

    /// What an erase writes: the background only (xterm.js's erase
    /// attributes keep the current background and nothing else).
    #[must_use]
    pub const fn erase(self) -> Self {
        Self {
            bg: self.bg,
            ..Self::PLAIN
        }
    }
}
