//! Concrete palettes and the reference's fixed palette colours, separate
//! from light/dark appearance and the app's stored preference. System
//! resolution belongs to the app; a fixed success colour is not an accent.
//!
//! [Decision Log]
//! - Purpose: add Paseo's named tints without repainting shipped Light/Dark.
//! - Constraint: the kit cannot import the view's preference type.
//! - Alternatives: expand Scheme, or duplicate every token per theme.
//! - Choice: one palette id, authored tint data and one mapping (ADR 0070).
//! - Cost: the app maps its preference to this UI-only id once.
mod contributed;
mod tint;
pub use contributed::ContributedColors;

use super::{Neutrals, Scheme, Tokens};
use gpui::{Hsla, rgb};
use tint::Tint;

/// Paseo palette.green[500]; menu success uses this in both themes.
pub const GREEN_500: u32 = 0x0022_c55e;

/// Fixed `palette.red[600]`, used by the source voice circle in both themes.
pub const RED_600: u32 = 0x00dc_2626;
/// Fixed `palette.red[800]`, its border, independent of semantic destructive.
pub const RED_800: u32 = 0x0099_1b1b;

/// A concrete token set. System is a preference, not a concrete palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Palette {
    /// The existing light palette, without a colour change.
    Light,
    /// The existing default dark palette, without a colour change.
    Dark,
    /// Paseo's neutral gray variant.
    Zinc,
    /// Paseo's blue-tinted variant.
    Midnight,
    /// Paseo's warm neutral variant.
    Claude,
    /// Paseo's slate-blue variant.
    Ghostty,
    /// Paseo's black-background variant.
    PureBlack,
}

impl Palette {
    /// Concrete catalogue order, omitting the adaptive System preference.
    pub const ALL: [Self; 7] = [
        Self::Light,
        Self::Dark,
        Self::Zinc,
        Self::Midnight,
        Self::Claude,
        Self::Ghostty,
        Self::PureBlack,
    ];

    /// Which shared status, syntax and official-mark alternatives apply.
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

impl From<Scheme> for Palette {
    fn from(scheme: Scheme) -> Self {
        match scheme {
            Scheme::Light => Self::Light,
            Scheme::Dark => Self::Dark,
        }
    }
}

impl Tokens {
    /// Build a named palette without changing the existing scheme entry point.
    #[must_use]
    pub fn palette(palette: Palette) -> Self {
        match palette {
            Palette::Light => Self::of(Scheme::Light),
            Palette::Dark => Self::of(Scheme::Dark),
            Palette::Zinc => tinted(tint::ZINC),
            Palette::Midnight => tinted(tint::MIDNIGHT),
            Palette::Claude => tinted(tint::CLAUDE),
            Palette::Ghostty => tinted(tint::GHOSTTY),
            Palette::PureBlack => tinted(tint::PURE_BLACK),
        }
    }
}

fn neutrals(tint: Tint, base: &Neutrals) -> Neutrals {
    Neutrals {
        surface0: rgb(tint.surface0).into(),
        surface1: rgb(tint.surface1).into(),
        surface2: rgb(tint.surface2).into(),
        surface3: rgb(tint.surface3).into(),
        surface4: rgb(tint.surface4).into(),
        surface_sidebar: rgb(tint.sidebar).into(),
        foreground: base.foreground,
        foreground_muted: rgb(tint.muted).into(),
        foreground_extra_muted: rgb(tint.extra_muted).into(),
        sheet_handle: base.sheet_handle,
        border: rgb(tint.border).into(),
        border_accent: rgb(tint.border_accent).into(),
        interaction_highlight: base.interaction_highlight,
        destructive: rgb(tint.destructive).into(),
        destructive_foreground: base.destructive_foreground,
        accent: rgb(tint.accent).into(),
        accent_foreground: tint
            .accent_foreground
            .map_or(base.accent_foreground, |color| rgb(color).into()),
    }
}

fn tinted(tint: Tint) -> Tokens {
    let base = Tokens::of(Scheme::Dark);
    let neutral = neutrals(tint, &base.neutral);
    mapped(&base, &neutral, rgb(tint::DARK_RING).into())
}

// One neutral-to-token mapping serves authored tints and validated custom
// paints. The two shipped constructors remain untouched.
fn mapped(base: &Tokens, neutral: &Neutrals, ring: Hsla) -> Tokens {
    Tokens {
        scheme: base.scheme,
        neutral: *neutral,
        bg_shell: neutral.surface_sidebar,
        bg_base: neutral.surface0,
        bg_surface: neutral.surface1,
        bg_elevated: match base.scheme {
            Scheme::Light => neutral.surface0,
            Scheme::Dark => neutral.surface2,
        },
        bg_hover: base.bg_hover,
        bg_active: base.bg_active,
        bg_well: neutral.surface2,
        border_subtle: neutral.border,
        border_faint: base.border_faint,
        border_strong: neutral.border_accent,
        text_primary: neutral.foreground,
        text_secondary: neutral.foreground_muted,
        text_tertiary: neutral.foreground_extra_muted,
        text_disabled: base.text_disabled,
        accent: neutral.foreground,
        ring,
        st_building: base.st_building,
        st_validating: base.st_validating,
        st_review: base.st_review,
        st_ready: base.st_ready,
        st_needs_you: base.st_needs_you,
        danger: neutral.destructive,
        btn_primary_bg: neutral.foreground,
        btn_primary_fg: neutral.surface0,
        btn_secondary_bg: neutral.surface2,
        btn_secondary_bg_hover: neutral.surface3,
        // A tinted theme is a dark one: the terminal keeps Paseo's dark
        // terminal palette (T1's tokens.terminal, ADR 0073).
        terminal: base.terminal,
    }
}

#[cfg(test)]
mod tests;
