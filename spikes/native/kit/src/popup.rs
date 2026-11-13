//! The surfaces every floating thing shares (menus, dialogs, a phone's
//! sheets, the composer's pickers): Paseo's app, cloned (the owner's rule
//! after build 0.0.0.0.3: the app's controls follow Paseo and GOAT; the
//! values and their sources are in the reference notes' "Appendix P",
//! A10 and B5).
//!
//! [Decision Log]
//! - Purpose: one recipe for every popup, so a menu, a dialog and a
//!   picker cannot drift apart, in Paseo's values: a menu on `surface1`
//!   edged with `borderAccent` at radius 8 under the medium shadow; a
//!   dialog card on `surface0` edged with `surface2` at radius 12 over a
//!   55% black scrim; a phone's sheet on `surface0` with its top corners
//!   at 16 over a 45% scrim.
//! - Constraint: the kit's theme does not hold Paseo's surface scale yet
//!   (the lead is adding `surface0` to `surface4`, `border` and
//!   `borderAccent`); the theme file is the lead's.
//! - Choice: [`Surfaces`] holds those values per scheme, named as Paseo
//!   names them, so moving them into `Tokens` is a rename; the controls
//!   read them only through this module.
//! - Differences from the page's DESIGN.md (the references win for the
//!   app, ADR 0036 brings the page along): the page's popup is white at
//!   radius 10 with a 2% edge and a softer shadow, its scrim the ink at
//!   45%; Paseo's are the values here.

use gpui::{BoxShadow, Hsla, point, px, rgba};

use crate::theme::{Scheme, Tokens};

/// Paseo's surface scale and the colours drawn on it, for one scheme
/// (Paseo's `theme.ts`: its light theme, and its default dark).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Surfaces {
    /// `surface0`: a dialog's card, a sheet.
    pub surface0: Hsla,
    /// `surface1`: a menu.
    pub surface1: Hsla,
    /// `surface2`: a hovered, focused or open row; a dialog's edge.
    pub surface2: Hsla,
    /// `borderAccent`: a menu's edge and its separators.
    pub border_accent: Hsla,
    /// `foreground`: a row's words.
    pub foreground: Hsla,
    /// `foregroundMuted`: a group's label, a description.
    pub foreground_muted: Hsla,
    /// `destructive`: the destructive row's words.
    pub destructive: Hsla,
}

impl Surfaces {
    /// The surfaces of the scheme `tokens` are in.
    #[must_use]
    pub fn of(tokens: &Tokens) -> Self {
        // The theme's neutral scale: one set of Paseo's values for the kit.
        let neutral = &tokens.neutral;
        Self {
            surface0: neutral.surface0,
            surface1: neutral.surface1,
            surface2: neutral.surface2,
            border_accent: neutral.border_accent,
            foreground: neutral.foreground,
            foreground_muted: neutral.foreground_muted,
            destructive: neutral.destructive,
        }
    }
}

/// Paseo's medium shadow (`theme.shadow.md`), under a menu: 4 down, 16 of
/// blur, black at 4% light; 4 down, 8 of blur, black at 20% dark.
#[must_use]
pub fn shadow_md(tokens: &Tokens) -> Vec<BoxShadow> {
    let (alpha, blur) = match tokens.scheme {
        Scheme::Light => (0x0a, 16.0),
        Scheme::Dark => (0x33, 8.0),
    };
    vec![BoxShadow {
        color: rgba(alpha).into(),
        offset: point(px(0.0), px(4.0)),
        blur_radius: px(blur),
        spread_radius: px(0.0),
        inset: false,
    }]
}

/// A dialog's scrim on a wide window: black at 55%.
#[must_use]
pub fn scrim() -> Hsla {
    rgba(0x0000_008c).into()
}

/// A phone's sheet's scrim: black at 45%.
#[must_use]
pub fn sheet_scrim() -> Hsla {
    rgba(0x0000_0073).into()
}

/// A menu's radius (Paseo's `lg`).
pub const MENU_RADIUS: f32 = 8.0;

/// A menu row's radius (Paseo's `md`).
pub const ROW_RADIUS: f32 = 6.0;

/// A dialog card's radius (Paseo's `xl`).
pub const DIALOG_RADIUS: f32 = 12.0;

/// A sheet's top corners (Paseo's `2xl`).
pub const SHEET_RADIUS: f32 = 16.0;

/// A row's height: Paseo's tight row, 28 points with a pointer, 44 on a
/// phone (a finger's target).
#[must_use]
pub const fn row_height(narrow: bool) -> f32 {
    if narrow { 44.0 } else { 28.0 }
}
