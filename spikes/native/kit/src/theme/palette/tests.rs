//! Authored-byte parity and the measured contrast contract in ADR 0070.

use super::*;

const BODY_FLOOR: f64 = 5.89;
const MUTED_BASE_FLOOR: f64 = 4.83;
const MUTED_SIDEBAR_FLOOR: f64 = 4.39;
const BRAND_ACCENT_FLOOR: f64 = 7.95;
const ZINC_ACCENT_FLOOR: f64 = 13.96;
const MIDNIGHT_ACCENT_FLOOR: f64 = 4.81;
const CLAUDE_ACCENT_FLOOR: f64 = 3.12;
const GHOSTTY_ACCENT_FLOOR: f64 = 4.62;
const LIGHT_DESTRUCTIVE_FLOOR: f64 = 5.73;
const DARK_DESTRUCTIVE_FLOOR: f64 = 4.56;
const NEUTRAL_RED_DESTRUCTIVE_FLOOR: f64 = 4.74;
const MIDNIGHT_DESTRUCTIVE_FLOOR: f64 = 4.72;
const CLAUDE_DESTRUCTIVE_FLOOR: f64 = 4.28;
const GHOSTTY_DESTRUCTIVE_FLOOR: f64 = 4.71;
const SRGB_KNEE: f64 = 0.04045;
const BYTE_MAX: f64 = 255.0;
const WHITE: u32 = 0x00ff_ffff;
const DARK_FOREGROUND: u32 = 0x00fa_fafa;
const LIGHT_FOREGROUND: u32 = 0x001a_1a1e;

// Baseline authored neutrals before this slice. These test-only bytes keep
// contrast independent of lossy round trips through GPUI's Hsla.
const LIGHT: Tint = Tint {
    surface0: WHITE,
    surface1: 0x00fa_fafa,
    surface2: 0x00f4_f4f5,
    surface3: 0x00e4_e4e7,
    surface4: 0x00d4_d4d8,
    sidebar: 0x00f4_f4f5,
    muted: 0x0071_717a,
    extra_muted: 0x00a1_a1aa,
    border: 0x00e4_e4e7,
    border_accent: 0x00ec_ecf1,
    accent: crate::assets::BRAND,
    accent_foreground: None,
    destructive: 0x00b0_4138,
};
const DARK: Tint = Tint {
    surface0: 0x0018_1b1a,
    surface1: 0x001e_2120,
    surface2: 0x0027_2a29,
    surface3: 0x0043_4645,
    surface4: 0x0059_5b5b,
    sidebar: 0x0014_1716,
    muted: 0x00a1_a5a4,
    extra_muted: 0x0071_7574,
    border: 0x0025_2b2a,
    border_accent: 0x002f_3534,
    accent: crate::assets::BRAND,
    accent_foreground: None,
    destructive: 0x00c6_4f43,
};

struct Sample {
    palette: Palette,
    tint: Tint,
    accent_floor: f64,
    destructive_floor: f64,
}

fn samples() -> [Sample; 7] {
    [
        Sample {
            palette: Palette::Light,
            tint: LIGHT,
            accent_floor: BRAND_ACCENT_FLOOR,
            destructive_floor: LIGHT_DESTRUCTIVE_FLOOR,
        },
        Sample {
            palette: Palette::Dark,
            tint: DARK,
            accent_floor: BRAND_ACCENT_FLOOR,
            destructive_floor: DARK_DESTRUCTIVE_FLOOR,
        },
        Sample {
            palette: Palette::Zinc,
            tint: tint::ZINC,
            accent_floor: ZINC_ACCENT_FLOOR,
            destructive_floor: NEUTRAL_RED_DESTRUCTIVE_FLOOR,
        },
        Sample {
            palette: Palette::Midnight,
            tint: tint::MIDNIGHT,
            accent_floor: MIDNIGHT_ACCENT_FLOOR,
            destructive_floor: MIDNIGHT_DESTRUCTIVE_FLOOR,
        },
        Sample {
            palette: Palette::Claude,
            tint: tint::CLAUDE,
            accent_floor: CLAUDE_ACCENT_FLOOR,
            destructive_floor: CLAUDE_DESTRUCTIVE_FLOOR,
        },
        Sample {
            palette: Palette::Ghostty,
            tint: tint::GHOSTTY,
            accent_floor: GHOSTTY_ACCENT_FLOOR,
            destructive_floor: GHOSTTY_DESTRUCTIVE_FLOOR,
        },
        Sample {
            palette: Palette::PureBlack,
            tint: tint::PURE_BLACK,
            accent_floor: BRAND_ACCENT_FLOOR,
            destructive_floor: NEUTRAL_RED_DESTRUCTIVE_FLOOR,
        },
    ]
}

#[test]
fn light_and_dark_entries_preserve_every_shipped_token() {
    assert_eq!(Tokens::palette(Palette::Light), Tokens::of(Scheme::Light));
    assert_eq!(Tokens::palette(Palette::Dark), Tokens::of(Scheme::Dark));
}

#[gpui::test]
fn switching_dark_palettes_updates_the_component_layer_together(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        for palette in [
            Palette::Dark,
            Palette::Zinc,
            Palette::Midnight,
            Palette::Claude,
            Palette::Ghostty,
            Palette::PureBlack,
        ] {
            crate::set_palette(cx, palette);
            let expected = Tokens::palette(palette);
            assert_eq!(crate::tokens(cx), expected);
            let component = cx.global::<gpui_component::Theme>();
            assert_eq!(component.colors.background, expected.bg_base);
            assert_eq!(component.colors.foreground, expected.neutral.foreground);
            assert_eq!(component.colors.border, expected.neutral.border);
            assert_eq!(
                component.colors.muted_foreground,
                expected.neutral.foreground_muted
            );
        }
    });
}

#[test]
fn every_palette_matches_authored_bytes_before_measuring_contrast() {
    assert_eq!(samples().map(|sample| sample.palette), Palette::ALL);
    for sample in samples() {
        let tokens = Tokens::palette(sample.palette);
        let neutral = tokens.neutral;
        let tint = sample.tint;
        let actual = [
            neutral.surface0,
            neutral.surface1,
            neutral.surface2,
            neutral.surface3,
            neutral.surface4,
            neutral.surface_sidebar,
            neutral.foreground_muted,
            neutral.foreground_extra_muted,
            neutral.border,
            neutral.border_accent,
            neutral.accent,
            neutral.destructive,
            neutral.accent_foreground,
            neutral.destructive_foreground,
            neutral.foreground,
        ];
        let expected = [
            tint.surface0,
            tint.surface1,
            tint.surface2,
            tint.surface3,
            tint.surface4,
            tint.sidebar,
            tint.muted,
            tint.extra_muted,
            tint.border,
            tint.border_accent,
            tint.accent,
            tint.destructive,
            tint.accent_foreground.unwrap_or(WHITE),
            WHITE,
            foreground(sample.palette),
        ];
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert_eq!(actual, rgb(expected).into(), "{:?}", sample.palette);
        }
        assert_eq!(tokens.scheme, sample.palette.scheme());
        assert_eq!(tokens.status(), super::super::Status::of(tokens.scheme));
    }
}

#[test]
fn contrast_of_each_palette_keeps_the_measured_reference_floors() {
    for sample in samples() {
        let tint = sample.tint;
        for surface in [
            tint.surface0,
            tint.surface1,
            tint.surface2,
            tint.surface3,
            tint.surface4,
            tint.sidebar,
        ] {
            assert_contrast(
                sample.palette,
                foreground(sample.palette),
                surface,
                BODY_FLOOR,
            );
        }
        assert_contrast(sample.palette, tint.muted, tint.surface0, MUTED_BASE_FLOOR);
        assert_contrast(
            sample.palette,
            tint.muted,
            tint.sidebar,
            MUTED_SIDEBAR_FLOOR,
        );
        assert_contrast(
            sample.palette,
            tint.accent_foreground.unwrap_or(WHITE),
            tint.accent,
            sample.accent_floor,
        );
        assert_contrast(
            sample.palette,
            WHITE,
            tint.destructive,
            sample.destructive_floor,
        );
    }
}

#[test]
fn variants_project_page_surfaces_but_keep_shared_lane_and_alpha_tokens() {
    let base = Tokens::of(Scheme::Dark);
    for palette in [
        Palette::Zinc,
        Palette::Midnight,
        Palette::Claude,
        Palette::Ghostty,
        Palette::PureBlack,
    ] {
        let tokens = Tokens::palette(palette);
        let neutral = tokens.neutral;
        for (actual, expected) in [
            (tokens.bg_shell, neutral.surface_sidebar),
            (tokens.bg_base, neutral.surface0),
            (tokens.bg_surface, neutral.surface1),
            (tokens.bg_elevated, neutral.surface2),
            (tokens.bg_well, neutral.surface2),
            (tokens.border_subtle, neutral.border),
            (tokens.border_strong, neutral.border_accent),
            (tokens.text_primary, neutral.foreground),
            (tokens.text_secondary, neutral.foreground_muted),
            (tokens.text_tertiary, neutral.foreground_extra_muted),
            (tokens.danger, neutral.destructive),
            (tokens.btn_primary_fg, neutral.surface0),
            (tokens.bg_hover, base.bg_hover),
            (tokens.bg_active, base.bg_active),
            (tokens.border_faint, base.border_faint),
            (tokens.text_disabled, base.text_disabled),
            (tokens.st_building, base.st_building),
            (tokens.st_validating, base.st_validating),
            (tokens.st_review, base.st_review),
            (tokens.st_ready, base.st_ready),
            (tokens.st_needs_you, base.st_needs_you),
            (
                neutral.interaction_highlight,
                base.neutral.interaction_highlight,
            ),
            (neutral.sheet_handle, base.neutral.sheet_handle),
        ] {
            assert_eq!(actual, expected, "{palette:?}");
        }
    }
}

fn foreground(palette: Palette) -> u32 {
    match palette.scheme() {
        Scheme::Light => LIGHT_FOREGROUND,
        Scheme::Dark => DARK_FOREGROUND,
    }
}

fn assert_contrast(palette: Palette, foreground: u32, background: u32, floor: f64) {
    let ratio = contrast(foreground, background);
    assert!(
        ratio >= floor,
        "{palette:?}: {foreground:06x}/{background:06x} {ratio} < {floor}"
    );
}

fn contrast(first: u32, second: u32) -> f64 {
    let first = luminance(first);
    let second = luminance(second);
    first
        .max(second)
        .mul_add(1.0, 0.05)
        .mul_add(first.min(second).mul_add(1.0, 0.05).recip(), 0.0)
}

fn luminance(color: u32) -> f64 {
    let [_, red, green, blue] = color.to_be_bytes();
    channel(red).mul_add(
        0.2126,
        channel(green).mul_add(0.7152, channel(blue).mul_add(0.0722, 0.0)),
    )
}

fn channel(byte: u8) -> f64 {
    let value = f64::from(byte).mul_add(BYTE_MAX.recip(), 0.0);
    if value <= SRGB_KNEE {
        value.mul_add(12.92_f64.recip(), 0.0)
    } else {
        value
            .mul_add(1.0, 0.055)
            .mul_add(1.055_f64.recip(), 0.0)
            .powf(2.4)
    }
}
