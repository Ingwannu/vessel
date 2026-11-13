//! The design system's tokens (`docs/design/web-ui/DESIGN.md` section 4,
//! `web/src/ui/tokens.css`), by the page's names, in both themes.
//!
//! The values are the page's, one for one, so the app and the page are
//! the same product to look at; a token changed there is changed here in
//! the same change (the real kit generates both from one table).
//! Overlays (hover, hairlines) keep their alpha, as the page's
//! `rgb(... / 4%)` do, so one value is right on every surface.

use gpui::{Hsla, rgb, rgba};

/// Which half of each token is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// The design's light theme.
    Light,
    /// The dark theme.
    Dark,
}

/// Every colour token the app's screens draw with.
#[derive(Debug, Clone, Copy)]
pub struct Tokens {
    /// Which theme these are.
    pub scheme: Scheme,
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
    /// `--field-bg`: a field's or a segmented track's fill.
    pub field_bg: Hsla,
    /// `--field-edge`.
    pub field_edge: Hsla,
    /// `--segment-thumb`: the chosen segment.
    pub segment_thumb: Hsla,
    /// `--segment-edge`: its hairline.
    pub segment_edge: Hsla,
}

impl Tokens {
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
            field_bg: rgba(0x1212_120a).into(),
            field_edge: rgba(0x0000_0014).into(),
            segment_thumb: rgb(0x00ff_ffff).into(),
            segment_edge: rgba(0x0000_0014).into(),
        }
    }

    /// Dark. Alpha bytes: 4% = 0x0a, 7% = 0x12, 14% = 0x24.
    fn dark() -> Self {
        Self {
            scheme: Scheme::Dark,
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
            // 4%, 3%, 10%, 6%.
            field_bg: rgba(0xffff_ff0a).into(),
            field_edge: rgba(0xffff_ff08).into(),
            segment_thumb: rgba(0xffff_ff1a).into(),
            segment_edge: rgba(0xffff_ff0f).into(),
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
}
