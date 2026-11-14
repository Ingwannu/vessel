//! Paseo's five dark themes, on the tokens [`Tokens`] already has.
//!
//! Light and Dark stay the functions in the parent: their accent is the
//! mark, and their page tokens are the page's (ADR 0070). A variant is
//! one tint plus the dark builder, so a field cannot be filled in four
//! themes and forgotten in the fifth. Names Paseo has and this struct
//! does not (`surfaceDiffEmpty`, the terminal, syntax themes) are not
//! added here.

use gpui::{Hsla, rgb, rgba};

use crate::assets::BRAND;

use super::{Neutrals, Scheme, Tokens};

/// One palette. System is not one of these: it resolves to Light or Dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeId {
    /// Paseo's `light`.
    Light,
    /// Paseo's default `dark`.
    Dark,
    /// Neutral gray, no tint.
    Zinc,
    /// A blue tint.
    Midnight,
    /// A warm tint, orange accent.
    Claude,
    /// Ghostty's blue-gray.
    Ghostty,
    /// A zero-luminance background.
    PureBlack,
}

impl ThemeId {
    /// Every palette, in Paseo's order with System omitted (System is a
    /// preference, and it only ever resolves to the first two).
    pub const ALL: [Self; 7] = [
        Self::Light,
        Self::Dark,
        Self::Zinc,
        Self::Midnight,
        Self::Claude,
        Self::Ghostty,
        Self::PureBlack,
    ];

    /// Light or dark, for the signals and code colours that do not vary
    /// by tint (Paseo spreads one dark status set into every dark theme).
    #[must_use]
    pub const fn scheme(self) -> Scheme {
        match self {
            Self::Light => Scheme::Light,
            Self::Dark
            | Self::Zinc
            | Self::Midnight
            | Self::Claude
            | Self::Ghostty
            | Self::PureBlack => Scheme::Dark,
        }
    }
}

/// The colours a dark tint actually sets. Foreground, ring and the
/// destructive's text are the builder's, the same for every one of these.
#[derive(Clone, Copy)]
struct Tint {
    surface0: u32,
    surface1: u32,
    surface2: u32,
    surface3: u32,
    surface4: u32,
    sidebar: u32,
    muted: u32,
    extra: u32,
    border: u32,
    border_accent: u32,
    accent: u32,
    accent_fg: u32,
    destructive: u32,
}

/// Paseo's dark builder (`theme.ts` 391-392, 415, 418): foreground, ring,
/// accent text except Zinc, destructive text.
const FOREGROUND: u32 = 0x00fa_fafa;
const RING: u32 = 0x00d4_d4d8;
const WHITE: u32 = 0x00ff_ffff;

/// `rgba(255, 255, 255, 0.08)`, the byte the dark neutrals already use.
const HIGHLIGHT: u32 = 0xffff_ff14;

const ZINC: Tint = Tint {
    surface0: 0x0018_181b,
    surface1: 0x001f_1f22,
    surface2: 0x0027_272a,
    surface3: 0x003f_3f46,
    surface4: 0x0052_525b,
    sidebar: 0x0013_1316,
    muted: 0x00a1_a1aa,
    extra: 0x0071_717a,
    border: 0x0027_272a,
    border_accent: 0x0030_3036,
    accent: 0x00e4_e4e7,
    accent_fg: 0x0018_181b,
    destructive: 0x00c4_4a4a,
};

const MIDNIGHT: Tint = Tint {
    surface0: 0x0016_1820,
    surface1: 0x001c_1e27,
    surface2: 0x0025_2731,
    surface3: 0x003c_3e4c,
    surface4: 0x0053_5564,
    sidebar: 0x0012_1420,
    muted: 0x009a_9db0,
    extra: 0x006b_6e82,
    border: 0x0024_2636,
    border_accent: 0x002e_3040,
    accent: 0x003b_6fcf,
    accent_fg: WHITE,
    destructive: 0x00c4_4a52,
};

const CLAUDE: Tint = Tint {
    surface0: 0x001f_1f1e,
    surface1: 0x0026_2523,
    surface2: 0x002f_2d2b,
    surface3: 0x004a_4745,
    surface4: 0x0060_5d5b,
    sidebar: 0x001a_1918,
    muted: 0x00ad_a9a5,
    extra: 0x0078_746f,
    border: 0x002c_2a27,
    border_accent: 0x0036_332f,
    accent: 0x00d9_7757,
    accent_fg: WHITE,
    destructive: 0x00cf_513e,
};

const GHOSTTY: Tint = Tint {
    surface0: 0x0028_2c34,
    surface1: 0x002f_333d,
    surface2: 0x0038_3c48,
    surface3: 0x004a_4f5e,
    surface4: 0x005b_6175,
    sidebar: 0x0021_252d,
    muted: 0x00c8_ccd8,
    extra: 0x00a0_a4b2,
    border: 0x0035_3a47,
    border_accent: 0x003f_4454,
    accent: 0x0035_70e0,
    accent_fg: WHITE,
    destructive: 0x00c4_4a55,
};

const PURE_BLACK: Tint = Tint {
    surface0: 0x0000_0000,
    surface1: 0x000a_0a0a,
    surface2: 0x0011_1111,
    surface3: 0x0020_2020,
    surface4: 0x002d_2d2d,
    sidebar: 0x0000_0000,
    muted: 0x00a1_a1aa,
    extra: 0x0071_717a,
    border: 0x001c_1c1c,
    border_accent: 0x0024_2424,
    // Paseo's green `#20744A` (`theme.ts:735`). The mark's colour
    // replaces that green on Light, Dark and Pure black alike (ADR 0070).
    accent: BRAND,
    accent_fg: WHITE,
    destructive: 0x00c4_4a4a,
};

/// The tokens of `id`. Light and Dark are not rebuilt.
pub(super) fn tokens(id: ThemeId) -> Tokens {
    let tint = match id {
        ThemeId::Light => return Tokens::light(),
        ThemeId::Dark => return Tokens::dark(),
        ThemeId::Zinc => ZINC,
        ThemeId::Midnight => MIDNIGHT,
        ThemeId::Claude => CLAUDE,
        ThemeId::Ghostty => GHOSTTY,
        ThemeId::PureBlack => PURE_BLACK,
    };
    from_tint(tint)
}

fn paint(hex: u32) -> Hsla {
    rgb(hex).into()
}

fn neutrals(tint: Tint) -> Neutrals {
    Neutrals {
        surface0: paint(tint.surface0),
        surface1: paint(tint.surface1),
        surface2: paint(tint.surface2),
        surface3: paint(tint.surface3),
        surface4: paint(tint.surface4),
        surface_sidebar: paint(tint.sidebar),
        foreground: paint(FOREGROUND),
        foreground_muted: paint(tint.muted),
        foreground_extra_muted: paint(tint.extra),
        border: paint(tint.border),
        border_accent: paint(tint.border_accent),
        interaction_highlight: rgba(HIGHLIGHT).into(),
        destructive: paint(tint.destructive),
        destructive_foreground: paint(WHITE),
        accent: paint(tint.accent),
        accent_foreground: paint(tint.accent_fg),
    }
}

/// Page tokens Paseo does not define stay the shipped dark page: the
/// hover and active overlays, the faint border, disabled text, and the
/// board's lane colours. Everything else is the tint (ADR 0070).
fn from_tint(tint: Tint) -> Tokens {
    let kept = Tokens::dark();
    let fg = paint(FOREGROUND);
    let surface0 = paint(tint.surface0);
    let surface2 = paint(tint.surface2);
    Tokens {
        scheme: Scheme::Dark,
        neutral: neutrals(tint),
        bg_shell: paint(tint.sidebar),
        bg_base: surface0,
        bg_surface: paint(tint.surface1),
        bg_elevated: surface2,
        bg_hover: kept.bg_hover,
        bg_active: kept.bg_active,
        bg_well: surface2,
        border_subtle: paint(tint.border),
        border_faint: kept.border_faint,
        border_strong: paint(tint.border_accent),
        text_primary: fg,
        text_secondary: paint(tint.muted),
        text_tertiary: paint(tint.extra),
        text_disabled: kept.text_disabled,
        accent: fg,
        ring: paint(RING),
        st_building: kept.st_building,
        st_validating: kept.st_validating,
        st_review: kept.st_review,
        st_ready: kept.st_ready,
        st_needs_you: kept.st_needs_you,
        danger: paint(tint.destructive),
        btn_primary_bg: fg,
        btn_primary_fg: surface0,
        btn_secondary_bg: surface2,
        btn_secondary_bg_hover: paint(tint.surface3),
    }
}

#[cfg(test)]
mod tests {
    use super::{CLAUDE, FOREGROUND, GHOSTTY, MIDNIGHT, PURE_BLACK, Tint, WHITE, ZINC, paint};
    use crate::assets::BRAND;
    use crate::theme::{Scheme, ThemeId, Tokens};

    /// Shipped light neutrals (`theme.rs`), accent included as the mark.
    const LIGHT: Tint = Tint {
        surface0: 0x00ff_ffff,
        surface1: 0x00fa_fafa,
        surface2: 0x00f4_f4f5,
        surface3: 0x00e4_e4e7,
        surface4: 0x00d4_d4d8,
        sidebar: 0x00f4_f4f5,
        muted: 0x0071_717a,
        extra: 0x00a1_a1aa,
        border: 0x00e4_e4e7,
        border_accent: 0x00ec_ecf1,
        accent: BRAND,
        accent_fg: WHITE,
        destructive: 0x00b0_4138,
    };

    /// Shipped dark neutrals. The accent is the mark, not `#20744A`.
    const DARK: Tint = Tint {
        surface0: 0x0018_1b1a,
        surface1: 0x001e_2120,
        surface2: 0x0027_2a29,
        surface3: 0x0043_4645,
        surface4: 0x0059_5b5b,
        sidebar: 0x0014_1716,
        muted: 0x00a1_a5a4,
        extra: 0x0071_7574,
        border: 0x0025_2b2a,
        border_accent: 0x002f_3534,
        accent: BRAND,
        accent_fg: WHITE,
        destructive: 0x00c6_4f43,
    };

    fn tint_of(id: ThemeId) -> Tint {
        match id {
            ThemeId::Light => LIGHT,
            ThemeId::Dark => DARK,
            ThemeId::Zinc => ZINC,
            ThemeId::Midnight => MIDNIGHT,
            ThemeId::Claude => CLAUDE,
            ThemeId::Ghostty => GHOSTTY,
            ThemeId::PureBlack => PURE_BLACK,
        }
    }

    #[expect(
        clippy::float_arithmetic,
        reason = "a WCAG channel is the byte divided by 255"
    )]
    fn channel(hex: u32, shift: u32) -> f64 {
        let byte = u8::try_from((hex >> shift) & 0xff).expect("a channel fits in a byte");
        f64::from(byte) / 255.0
    }

    #[expect(
        clippy::float_arithmetic,
        reason = "the sRGB transfer function is defined with these operations"
    )]
    fn linear(channel: f64) -> f64 {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }

    /// WCAG 2.1 contrast of two opaque `0x00RRGGBB` colours.
    #[expect(
        clippy::float_arithmetic,
        reason = "WCAG 2.1 defines the contrast ratio with these operations"
    )]
    fn contrast(a: u32, b: u32) -> f64 {
        let lum = |hex: u32| {
            0.2126 * linear(channel(hex, 16))
                + 0.7152 * linear(channel(hex, 8))
                + 0.0722 * linear(channel(hex, 0))
        };
        let (hi, lo) = {
            let (one, two) = (lum(a), lum(b));
            if one >= two { (one, two) } else { (two, one) }
        };
        (hi + 0.05) / (lo + 0.05)
    }

    fn surfaces(tint: Tint) -> [u32; 6] {
        [
            tint.surface0,
            tint.surface1,
            tint.surface2,
            tint.surface3,
            tint.surface4,
            tint.sidebar,
        ]
    }

    #[test]
    fn every_palette_builds_and_only_light_is_light() {
        assert_eq!(ThemeId::ALL.len(), 7);
        for id in ThemeId::ALL {
            let tokens = Tokens::of_theme(id);
            assert_eq!(tokens.scheme, id.scheme());
            let tint = tint_of(id);
            let neutral = tokens.neutral;
            let body = match id {
                ThemeId::Light => 0x001a_1a1e,
                ThemeId::Dark
                | ThemeId::Zinc
                | ThemeId::Midnight
                | ThemeId::Claude
                | ThemeId::Ghostty
                | ThemeId::PureBlack => FOREGROUND,
            };
            assert_eq!(neutral.foreground, paint(body));
            assert_eq!(neutral.surface0, paint(tint.surface0));
            assert_eq!(neutral.surface1, paint(tint.surface1));
            assert_eq!(neutral.surface2, paint(tint.surface2));
            assert_eq!(neutral.surface3, paint(tint.surface3));
            assert_eq!(neutral.surface4, paint(tint.surface4));
            assert_eq!(neutral.surface_sidebar, paint(tint.sidebar));
            assert_eq!(neutral.foreground_muted, paint(tint.muted));
            assert_eq!(neutral.foreground_extra_muted, paint(tint.extra));
            assert_eq!(neutral.border, paint(tint.border));
            assert_eq!(neutral.border_accent, paint(tint.border_accent));
            assert_eq!(neutral.accent, paint(tint.accent));
            assert_eq!(neutral.accent_foreground, paint(tint.accent_fg));
            assert_eq!(neutral.destructive, paint(tint.destructive));
            assert_eq!(neutral.destructive_foreground, paint(WHITE));
        }
    }

    #[test]
    fn light_and_dark_are_the_palettes_that_already_ship() {
        assert_eq!(Tokens::of_theme(ThemeId::Light), Tokens::of(Scheme::Light));
        assert_eq!(Tokens::of_theme(ThemeId::Dark), Tokens::of(Scheme::Dark));
    }

    #[test]
    fn a_variant_keeps_the_dark_page_lanes_and_paints_the_tint() {
        let dark = Tokens::of(Scheme::Dark);
        for id in [
            ThemeId::Zinc,
            ThemeId::Midnight,
            ThemeId::Claude,
            ThemeId::Ghostty,
            ThemeId::PureBlack,
        ] {
            let tokens = Tokens::of_theme(id);
            let tint = tint_of(id);
            assert_eq!(tokens.bg_hover, dark.bg_hover);
            assert_eq!(tokens.bg_active, dark.bg_active);
            assert_eq!(tokens.border_faint, dark.border_faint);
            assert_eq!(tokens.text_disabled, dark.text_disabled);
            assert_eq!(tokens.st_building, dark.st_building);
            assert_eq!(tokens.st_validating, dark.st_validating);
            assert_eq!(tokens.st_review, dark.st_review);
            assert_eq!(tokens.st_ready, dark.st_ready);
            assert_eq!(tokens.st_needs_you, dark.st_needs_you);
            assert_eq!(tokens.text_primary, paint(FOREGROUND));
            assert_eq!(tokens.text_tertiary, paint(tint.extra));
            assert_eq!(tokens.bg_shell, paint(tint.sidebar));
            assert_eq!(tokens.bg_base, paint(tint.surface0));
            assert_eq!(tokens.bg_surface, paint(tint.surface1));
            assert_eq!(tokens.bg_elevated, paint(tint.surface2));
            assert_eq!(tokens.bg_well, paint(tint.surface2));
            assert_eq!(tokens.danger, paint(tint.destructive));
            assert_eq!(tokens.btn_primary_fg, paint(tint.surface0));
            assert_eq!(tokens.btn_secondary_bg_hover, paint(tint.surface3));
            assert_eq!(tokens.ring, paint(0x00d4_d4d8));
        }
    }

    fn accent_floor(id: ThemeId) -> f64 {
        match id {
            ThemeId::Light | ThemeId::Dark | ThemeId::PureBlack => 7.95,
            ThemeId::Zinc => 13.96,
            ThemeId::Midnight => 4.81,
            ThemeId::Claude => 3.12,
            ThemeId::Ghostty => 4.62,
        }
    }

    fn destructive_floor(id: ThemeId) -> f64 {
        match id {
            ThemeId::Light => 5.73,
            ThemeId::Dark => 4.56,
            ThemeId::Zinc | ThemeId::PureBlack => 4.74,
            ThemeId::Midnight => 4.72,
            ThemeId::Claude => 4.28,
            ThemeId::Ghostty => 4.71,
        }
    }

    #[test]
    fn text_on_each_surface_meets_the_measured_contrast() {
        for id in ThemeId::ALL {
            let tint = tint_of(id);
            let body = match id {
                ThemeId::Light => 0x001a_1a1e,
                ThemeId::Dark
                | ThemeId::Zinc
                | ThemeId::Midnight
                | ThemeId::Claude
                | ThemeId::Ghostty
                | ThemeId::PureBlack => FOREGROUND,
            };
            assert_eq!(Tokens::of_theme(id).neutral.foreground, paint(body));
            for surface in surfaces(tint) {
                let ratio = contrast(body, surface);
                assert!(
                    ratio >= 5.89,
                    "{id:?} body on {surface:06x} is {ratio}, under 5.89"
                );
            }
            let on_page = contrast(tint.muted, tint.surface0);
            assert!(on_page >= 4.83, "{id:?} muted on surface0 is {on_page}");
            let on_side = contrast(tint.muted, tint.sidebar);
            assert!(on_side >= 4.39, "{id:?} muted on the sidebar is {on_side}");
            let accent = contrast(tint.accent_fg, tint.accent);
            assert!(accent >= accent_floor(id), "{id:?} accent text is {accent}");
            let destructive = contrast(WHITE, tint.destructive);
            assert!(
                destructive >= destructive_floor(id),
                "{id:?} destructive text is {destructive}"
            );
        }
    }
}
