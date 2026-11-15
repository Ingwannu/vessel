//! A cell's pen as drawn: its colours resolved against the palette the
//! screen is drawn in (ADR 0073), so the kit receives colours, not rules.
//!
//! [Decision Log]
//! - Purpose: the kit draws the snapshot's cells; which colour a cell is
//!   (bold drawn bright, inverse, the 256-colour table) is a rule, and
//!   rules live in `view` with tests.
//! - Choice: xterm.js's defaults, which Paseo keeps (`terminal-emulator-
//!   runtime.ts` 555–571 sets no `drawBoldTextInBrightColors`, so it is on,
//!   and `minimumContrastRatio: 1`, so no colour is adjusted). Inverse
//!   swaps the two colours first, a default colour taking the palette's
//!   foreground or background; then bold draws a foreground that is one
//!   of the eight ANSI colours in its bright form. xterm.js 6.1.0-beta.336
//!   (the version Paseo pins) does it in that order in both renderers:
//!   `addon-webgl/src/TextureAtlas.ts` 519–526 swaps, then 355 and 364–366
//!   brighten; `DomRendererRowFactory.ts` 347–355 swaps, then 437–438
//!   brighten. So `ESC [ 1 ; 7 ; 31 ; 44 m` is bright blue on red. Faint is
//!   half opacity; a concealed glyph is not drawn. Colours 16–231 are the 6 × 6 × 6 cube and 232–255 the grey
//!   ramp, xterm's values; 0–15 and the defaults are the palette's.
//! - Alternative: the kit resolving colours from its tokens (the rules
//!   would sit untested in a drawing crate).

#[cfg(test)]
mod tests;

use super::grid::{Colour, Concealment, Pen, Polarity, Rgb, Weight};

/// The colours a terminal is drawn in (the kit's `tokens.terminal`, which
/// the app copies here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// The 16 ANSI colours: 0–7 and their bright forms 8–15.
    pub ansi: [Rgb; 16],
    /// The default foreground.
    pub foreground: Rgb,
    /// The default background.
    pub background: Rgb,
    /// The cursor.
    pub cursor: Rgb,
}

/// How a cell's glyph is inked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ink {
    /// At full strength.
    Full,
    /// At half opacity (SGR 2).
    Faint,
    /// Not drawn (SGR 8); the background still is.
    Hidden,
}

/// A cell's colours as drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Painted {
    /// The glyph's colour.
    pub fg: Rgb,
    /// The cell's background; `None` is the palette's, which the grid's
    /// own background already shows (nothing to draw).
    pub bg: Option<Rgb>,
    /// How the glyph is inked.
    pub ink: Ink,
}

/// `pen` drawn in `palette`.
#[must_use]
pub fn paint(pen: Pen, palette: &Palette) -> Painted {
    // Inverse first: the colours trade places, defaults included.
    let (fg, bg, foreground, background) = match pen.polarity {
        Polarity::Normal => (pen.fg, pen.bg, palette.foreground, None),
        Polarity::Inverse => (pen.bg, pen.fg, palette.background, Some(palette.foreground)),
    };
    // Then bold brightens the foreground drawn, when it is one of the 8.
    let fg = match fg {
        Colour::Default => foreground,
        Colour::Indexed(index) => {
            let bright = match pen.weight {
                Weight::Bold if index < 8 => index.saturating_add(8),
                Weight::Bold | Weight::Regular | Weight::Faint => index,
            };
            indexed(bright, palette)
        }
        Colour::Rgb(rgb) => rgb,
    };
    let bg = match bg {
        Colour::Default => background,
        Colour::Indexed(index) => Some(indexed(index, palette)),
        Colour::Rgb(rgb) => Some(rgb),
    };
    let ink = match (pen.concealment, pen.weight) {
        (Concealment::Concealed, _) => Ink::Hidden,
        (Concealment::Shown, Weight::Faint) => Ink::Faint,
        (Concealment::Shown, Weight::Regular | Weight::Bold) => Ink::Full,
    };
    Painted { fg, bg, ink }
}

/// Palette index `index`: the palette's 16, then the cube and the ramp.
#[must_use]
pub fn indexed(index: u8, palette: &Palette) -> Rgb {
    match index.checked_sub(16) {
        None => palette
            .ansi
            .get(usize::from(index))
            .copied()
            .unwrap_or(palette.foreground),
        Some(cube) if cube < 216 => {
            let r = cube.checked_div(36).unwrap_or(0);
            let g = cube
                .checked_div(6)
                .and_then(|g| g.checked_rem(6))
                .unwrap_or(0);
            let b = cube.checked_rem(6).unwrap_or(0);
            Rgb {
                r: level(r),
                g: level(g),
                b: level(b),
            }
        }
        Some(cube) => {
            // 232–255: 8, 18, … 238.
            let step = cube.saturating_sub(216);
            let grey = step.saturating_mul(10).saturating_add(8);
            Rgb {
                r: grey,
                g: grey,
                b: grey,
            }
        }
    }
}

/// A cube coordinate (0–5) as a channel: 0, 95, 135, 175, 215, 255.
const fn level(step: u8) -> u8 {
    if step == 0 {
        0
    } else {
        step.saturating_mul(40).saturating_add(55)
    }
}
