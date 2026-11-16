//! The design system's tokens (`docs/design/web-ui/DESIGN.md` section 4,
//! `web/src/ui/tokens.css`), with named Paseo palettes in [`Palette`].
//!
//! The two shipped page token sets stay unchanged. Additional dark tints
//! map Paseo's authored colours to these names (ADR 0070), without a second
//! set of controls or a dependency on the view's stored theme preference.
//! Overlays (hover, hairlines) keep their alpha, as the page's
//! `rgb(... / 4%)` do, so one value is right on every surface.

pub mod identity;
pub mod palette;
pub mod spacing;
pub mod terminal;
pub use palette::Palette;

/// Paseo attachment-lightbox backdrop: black at 90 percent.
pub const IMAGE_SCRIM: u32 = 0x0000_00e6;

/// The desktop command panel's scrim: black at 50 percent after Paseo's
/// command center, as `rgba` takes it.
pub const COMMAND_SCRIM: u32 = 0x0000_0080;

/// A form summary's light fill: GOAT's zinc25 (OKLCH .992, 0, 0) at its
/// nearest 8-bit sRGB value, as `rgb` takes it.
pub const SUMMARY_LIGHT: u32 = 0x00fc_fcfc;
mod status;
pub use status::{Signal, Status};

use gpui::{Hsla, rgb, rgba};

/// Which half of each token is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// The design's light theme.
    Light,
    /// The dark theme.
    Dark,
}

/// Paseo's neutral scale (`packages/app/src/styles/theme.ts`: its light
/// theme and its default dark), by Paseo's names: what the app's controls
/// are drawn with, so they read as Paseo's (the owner: the app's layout
/// and controls follow Paseo and GOAT).
///
/// [Decision Log]
/// - Purpose: controls looked like a library's defaults and like the
///   page's tokens approximated; the owner named Paseo as the model.
/// - Choice: Paseo's neutrals, value for value, in both themes. Two
///   deliberate differences, both Vessel's brand: the accent is the mark's
///   colour (`accent`, Paseo's is green), and the faces are Geist and
///   Pretendard (the owner's type rule), not the system's.
/// - The page's tokens beside them (`bg_*`, `text_*`) stay until each
///   screen is moved; a new control uses these.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Neutrals {
    /// `surface0`: the background; a combobox, a dialog, a sheet.
    pub surface0: Hsla,
    /// `surface1`: the composer's card, a menu; a pill pressed or open.
    pub surface1: Hsla,
    /// `surface2`: a hovered pill or row; an input's fill.
    pub surface2: Hsla,
    /// `surface3`: a hovered sidebar row (light).
    pub surface3: Hsla,
    /// `surface4`: a selected sidebar row (light); the composer's
    /// placeholder.
    pub surface4: Hsla,
    /// `surfaceSidebar`.
    pub surface_sidebar: Hsla,
    /// `foreground`.
    pub foreground: Hsla,
    /// `foregroundMuted`: a pill's value, a description.
    pub foreground_muted: Hsla,
    /// `foregroundExtraMuted`.
    pub foreground_extra_muted: Hsla,
    /// Command Center's sheet handle: Paseo's `palette.zinc[600]`.
    pub sheet_handle: Hsla,
    /// `border`: a combobox's edge.
    pub border: Hsla,
    /// `borderAccent`: the composer's card, a menu's edge, a field on
    /// hover.
    pub border_accent: Hsla,
    /// `interactionHighlight`: a window control's hover.
    pub interaction_highlight: Hsla,
    /// `destructive`: the stop button, a destructive row.
    pub destructive: Hsla,
    /// `destructiveForeground`: white in both reference themes.
    pub destructive_foreground: Hsla,
    /// The send button's accent: Vessel's brand where Paseo uses its green;
    /// otherwise the named palette's authored accent (ADR 0070).
    pub accent: Hsla,
    /// What is drawn on the accent.
    pub accent_foreground: Hsla,
}

impl Neutrals {
    /// Paseo's light theme (`theme.ts` 270-344), the brand's accent.
    fn light() -> Self {
        Self {
            surface0: rgb(0x00ff_ffff).into(),
            surface1: rgb(0x00fa_fafa).into(),
            surface2: rgb(0x00f4_f4f5).into(),
            surface3: rgb(0x00e4_e4e7).into(),
            surface4: rgb(0x00d4_d4d8).into(),
            surface_sidebar: rgb(0x00f4_f4f5).into(),
            foreground: rgb(0x001a_1a1e).into(),
            foreground_muted: rgb(0x0071_717a).into(),
            foreground_extra_muted: rgb(0x00a1_a1aa).into(),
            sheet_handle: rgb(0x0052_525b).into(),
            border: rgb(0x00e4_e4e7).into(),
            border_accent: rgb(0x00ec_ecf1).into(),
            interaction_highlight: rgba(0x0000_000f).into(),
            destructive: rgb(0x00b0_4138).into(),
            destructive_foreground: rgb(0x00ff_ffff).into(),
            accent: rgb(crate::assets::BRAND).into(),
            accent_foreground: rgb(0x00ff_ffff).into(),
        }
    }

    /// Paseo's default dark theme (`theme.ts` 391-477), the brand's accent.
    fn dark() -> Self {
        Self {
            surface0: rgb(0x0018_1b1a).into(),
            surface1: rgb(0x001e_2120).into(),
            surface2: rgb(0x0027_2a29).into(),
            surface3: rgb(0x0043_4645).into(),
            surface4: rgb(0x0059_5b5b).into(),
            surface_sidebar: rgb(0x0014_1716).into(),
            foreground: rgb(0x00fa_fafa).into(),
            foreground_muted: rgb(0x00a1_a5a4).into(),
            foreground_extra_muted: rgb(0x0071_7574).into(),
            sheet_handle: rgb(0x0052_525b).into(),
            border: rgb(0x0025_2b2a).into(),
            border_accent: rgb(0x002f_3534).into(),
            interaction_highlight: rgba(0xffff_ff14).into(),
            destructive: rgb(0x00c6_4f43).into(),
            destructive_foreground: rgb(0x00ff_ffff).into(),
            accent: rgb(crate::assets::BRAND).into(),
            accent_foreground: rgb(0x00ff_ffff).into(),
        }
    }
}

/// Every colour token the app's screens draw with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tokens {
    /// Which theme these are.
    pub scheme: Scheme,
    /// Paseo's neutral scale, for controls.
    pub neutral: Neutrals,
    /// `--bg-shell`, `--bg-sidebar`: the sidebar and the frame's margin.
    pub bg_shell: Hsla,
    /// `--bg-base`: the page inside the frame.
    pub bg_base: Hsla,
    /// `--bg-surface`: cards, inputs, the composer.
    pub bg_surface: Hsla,
    /// `--bg-elevated`: dialogs, menus.
    pub bg_elevated: Hsla,
    /// `--bg-hover`: the ink at 4%.
    pub bg_hover: Hsla,
    /// `--bg-active`: the ink at 7%, the current row.
    pub bg_active: Hsla,
    /// `--bg-well`: the search row, an avatar's tile.
    pub bg_well: Hsla,
    /// `--border-subtle`: a card's edge.
    pub border_subtle: Hsla,
    /// `--border-faint`: the frame, lane dividers.
    pub border_faint: Hsla,
    /// `--border-strong`: a focused well.
    pub border_strong: Hsla,
    /// `--text-primary`.
    pub text_primary: Hsla,
    /// `--text-secondary`.
    pub text_secondary: Hsla,
    /// `--text-tertiary`.
    pub text_tertiary: Hsla,
    /// `--text-disabled`.
    pub text_disabled: Hsla,
    /// `--accent`: selection and the primary button, the ink.
    pub accent: Hsla,
    /// `--ring`: the focus ring.
    pub ring: Hsla,
    /// `--st-building`.
    pub st_building: Hsla,
    /// `--st-validating`.
    pub st_validating: Hsla,
    /// `--st-review`.
    pub st_review: Hsla,
    /// `--st-ready`.
    pub st_ready: Hsla,
    /// `--st-needs-you`.
    pub st_needs_you: Hsla,
    /// `--danger`.
    pub danger: Hsla,
    /// `--btn-primary-bg`.
    pub btn_primary_bg: Hsla,
    /// `--btn-primary-fg`.
    pub btn_primary_fg: Hsla,
    /// `--btn-secondary-bg`.
    pub btn_secondary_bg: Hsla,
    /// `--btn-secondary-bg-hover`.
    pub btn_secondary_bg_hover: Hsla,
    /// The terminal's palette (Paseo's, ADR 0073).
    pub terminal: terminal::TerminalColors,
}

impl Tokens {
    /// Semantic status signals for badges and status text, after Paseo.
    #[must_use]
    pub fn status(&self) -> Status {
        Status::of(self.scheme)
    }

    /// The tokens of `scheme`.
    #[must_use]
    pub fn of(scheme: Scheme) -> Self {
        match scheme {
            Scheme::Light => Self::light(),
            Scheme::Dark => Self::dark(),
        }
    }

    /// Light (the design). Alpha bytes: 4% = 0x0a, 7% = 0x12, 8% = 0x14,
    /// 16% = 0x29.
    fn light() -> Self {
        Self {
            scheme: Scheme::Light,
            neutral: Neutrals::light(),
            bg_shell: rgb(0x00f4_f4f4).into(),
            bg_base: rgb(0x00fa_fafa).into(),
            bg_surface: rgb(0x00ff_ffff).into(),
            bg_elevated: rgb(0x00ff_ffff).into(),
            bg_hover: rgba(0x1212_120a).into(),
            bg_active: rgba(0x1212_1212).into(),
            bg_well: rgba(0x1212_120a).into(),
            border_subtle: rgba(0x0000_0014).into(),
            border_faint: rgba(0x0000_000a).into(),
            border_strong: rgba(0x0000_0029).into(),
            text_primary: rgb(0x0012_1212).into(),
            text_secondary: rgb(0x005d_5d5d).into(),
            text_tertiary: rgb(0x0066_6666).into(),
            text_disabled: rgb(0x00a3_a3a3).into(),
            accent: rgb(0x0012_1212).into(),
            ring: rgb(0x008c_8c8c).into(),
            st_building: rgb(0x0025_63eb).into(),
            st_validating: rgb(0x009b_4871).into(),
            st_review: rgb(0x00b4_5309).into(),
            st_ready: rgb(0x0015_803d).into(),
            st_needs_you: rgb(0x00c2_412d).into(),
            danger: rgb(0x009d_0006).into(),
            btn_primary_bg: rgb(0x001f_1f1f).into(),
            btn_primary_fg: rgb(0x00ff_ffff).into(),
            btn_secondary_bg: rgb(0x00f0_f0f0).into(),
            btn_secondary_bg_hover: rgb(0x00e4_e4e4).into(),
            terminal: terminal::TerminalColors::light(),
        }
    }

    /// Dark. Alpha bytes: 4% = 0x0a, 7% = 0x12, 14% = 0x24.
    fn dark() -> Self {
        Self {
            scheme: Scheme::Dark,
            neutral: Neutrals::dark(),
            bg_shell: rgb(0x0012_1212).into(),
            bg_base: rgb(0x0018_1818).into(),
            bg_surface: rgb(0x0022_2222).into(),
            bg_elevated: rgb(0x002e_2e2e).into(),
            bg_hover: rgba(0xf5f5_f50a).into(),
            bg_active: rgba(0xf5f5_f512).into(),
            bg_well: rgba(0xffff_ff0a).into(),
            border_subtle: rgba(0xffff_ff12).into(),
            border_faint: rgba(0xffff_ff0a).into(),
            border_strong: rgba(0xffff_ff24).into(),
            text_primary: rgb(0x00f5_f5f5).into(),
            text_secondary: rgb(0x00a4_a4a4).into(),
            text_tertiary: rgb(0x008f_8f8f).into(),
            text_disabled: rgb(0x005c_5c5c).into(),
            accent: rgb(0x00f5_f5f5).into(),
            ring: rgb(0x0086_8686).into(),
            st_building: rgb(0x0060_a5fa).into(),
            st_validating: rgb(0x00e5_8bb5).into(),
            st_review: rgb(0x00f5_9e0b).into(),
            st_ready: rgb(0x004a_de80).into(),
            st_needs_you: rgb(0x00f0_6445).into(),
            danger: rgb(0x00fb_4934).into(),
            btn_primary_bg: rgb(0x00de_dede).into(),
            btn_primary_fg: rgb(0x0018_1818).into(),
            btn_secondary_bg: rgb(0x002c_2c2c).into(),
            btn_secondary_bg_hover: rgb(0x0036_3636).into(),
            terminal: terminal::TerminalColors::dark(),
        }
    }
}

/// The radii (DESIGN.md section 4.5), in pixels.
pub mod radius {
    /// `--r-1`: badges, small chips.
    pub const R1: f32 = 4.0;
    /// `--r-2`, `--r-3`: buttons, fields.
    pub const R2: f32 = 8.0;
    /// `--r-4`: sidebar rows, board cards.
    pub const R4: f32 = 10.0;
    /// `--r-5`: the frame.
    pub const R5: f32 = 12.0;
    /// Paseo's `borderRadius.md`: small icon actions and popup rows. The
    /// page's scale has no 6; controls drawn after Paseo's use this one.
    pub const MD: f32 = 6.0;
}

/// Paseo's icon sizes (`ICON_SIZE` in its theme), in pixels.
pub mod icon {
    /// `ICON_SIZE.sm`: an icon-only action's glyph.
    pub const SM: f32 = 14.0;
    /// `ICON_SIZE.md`: the desktop sidebar footer glyph.
    pub const MD: f32 = 16.0;
    /// `ICON_SIZE.lg`: source circular action and schedule line-box glyph.
    pub const LG: f32 = 20.0;
}
