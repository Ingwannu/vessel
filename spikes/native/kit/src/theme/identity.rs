//! Scheme-aware identity ink and stable tints, after Paseo's identity-colors.
//! This is generic theme data: no workspace, session, or protocol types.
//!
//! [Decision Log]
//! - Purpose: label dots/chips and host/project identity should share one
//!   contrast band rather than borrow unrelated status or brand colors.
//! - Constraints: the reference uses separate light/dark foregrounds and
//!   a theme-independent fill at the exact 8-bit alpha 0x1a.
//! - Alternatives: screen-local hex colors, brand accents, or status tokens.
//! - Choice: one hue enum and one token table, converted to GPUI colors here.
//! - Why: callers choose identity; the theme owns paint and contrast.
//! - Costs: contributed schemes keep these named identities and select the
//!   matching light/dark foreground, without changing the palette builder.

use gpui::{Hsla, rgb, rgba};

use super::Scheme;

const TINT_ALPHA: u8 = 0x1a;

/// Reference identity order is stable so existing projects keep their hue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hue {
    /// Violet.
    Violet,
    /// Sky blue.
    Sky,
    /// Emerald green.
    Emerald,
    /// Orange.
    Orange,
    /// Pink.
    Pink,
    /// Indigo.
    Indigo,
    /// Teal.
    Teal,
    /// Red.
    Red,
    /// Amber.
    Amber,
    /// Blue.
    Blue,
}

impl Hue {
    /// The reference's identity ordering, not alphabetical order.
    pub const ALL: [Self; 10] = [
        Self::Violet,
        Self::Sky,
        Self::Emerald,
        Self::Orange,
        Self::Pink,
        Self::Indigo,
        Self::Teal,
        Self::Red,
        Self::Amber,
        Self::Blue,
    ];

    const fn tone(self) -> Tone {
        match self {
            Self::Violet => Tone {
                fill: 0x007a_6aa8,
                light: 0x006d_49b5,
                dark: 0x00a3_92d5,
            },
            Self::Sky => Tone {
                fill: 0x003d_7ea6,
                light: 0x003d_6985,
                dark: 0x006a_a6ce,
            },
            Self::Emerald => Tone {
                fill: 0x0038_8068,
                light: 0x003e_6e5d,
                dark: 0x006c_ae96,
            },
            Self::Orange => Tone {
                fill: 0x00a4_673a,
                light: 0x0084_5838,
                dark: 0x00cc_8f64,
            },
            Self::Pink => Tone {
                fill: 0x00b0_5c80,
                light: 0x0097_4168,
                dark: 0x00d8_7da3,
            },
            Self::Indigo => Tone {
                fill: 0x006a_70b8,
                light: 0x0052_51c2,
                dark: 0x0092_99d5,
            },
            Self::Teal => Tone {
                fill: 0x0036_8080,
                light: 0x003e_6d6c,
                dark: 0x006c_acab,
            },
            Self::Red => Tone {
                fill: 0x00b0_6260,
                light: 0x009c_4243,
                dark: 0x00d8_8381,
            },
            Self::Amber => Tone {
                fill: 0x008f_7838,
                light: 0x0071_6239,
                dark: 0x00b2_9d64,
            },
            Self::Blue => Tone {
                fill: 0x0051_79b0,
                light: 0x0039_649e,
                dark: 0x007b_a1d5,
            },
        }
    }
}

struct Tone {
    fill: u32,
    light: u32,
    dark: u32,
}

/// Foreground ink for identity glyphs/text; the scheme chooses the contrast band.
#[must_use]
pub fn foreground(hue: Hue, scheme: Scheme) -> Hsla {
    let tone = hue.tone();
    rgb(match scheme {
        Scheme::Light => tone.light,
        Scheme::Dark => tone.dark,
    })
    .into()
}

/// A stable translucent ground, independent of the active scheme.
#[must_use]
pub fn tint(hue: Hue) -> Hsla {
    // Rows are RGB24 (tested), so rotating one byte opens the alpha slot
    // without truncating a channel or doing screen-local float arithmetic.
    rgba(hue.tone().fill.rotate_left(u8::BITS) | u32::from(TINT_ALPHA)).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn identities_keep_distinct_ink_bands_and_exact_theme_independent_tint_alpha() {
        let mut light = BTreeSet::new();
        let mut dark = BTreeSet::new();
        for hue in Hue::ALL {
            assert!(hue.tone().fill.leading_zeros() >= u8::BITS);
            let light_ink = foreground(hue, Scheme::Light);
            let dark_ink = foreground(hue, Scheme::Dark);
            assert!(dark_ink.l > light_ink.l);
            assert_eq!(light_ink.a.to_bits(), 1_f32.to_bits());
            assert_eq!(dark_ink.a.to_bits(), 1_f32.to_bits());
            let alpha_reference: Hsla = rgba(u32::from(TINT_ALPHA)).into();
            assert_eq!(tint(hue).a.to_bits(), alpha_reference.a.to_bits());
            assert!(light.insert((
                light_ink.h.to_bits(),
                light_ink.s.to_bits(),
                light_ink.l.to_bits()
            )));
            assert!(dark.insert((
                dark_ink.h.to_bits(),
                dark_ink.s.to_bits(),
                dark_ink.l.to_bits()
            )));
        }
    }
}
