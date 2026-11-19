//! Paseo's sidebar row surfaces (`styles/theme.ts`): the sidebar itself,
//! a hovered row, a selected row, and a pressed row (`surface2`, as
//! `getSidebarRowBackdrop` names it).
//!
//! [Decision Log]
//! - Purpose: one place for the fills a sidebar row paints, so a status
//!   badge knocking out of the row (TRACKER 662) paints the very same
//!   colour in every state; Paseo keeps the rule in one function
//!   (`sidebar-row-backdrop.ts`) for that reason.
//! - Constraint: Paseo's two themes do not map these the same way: the
//!   light theme hovers on `surface3` and selects on `surface4`, the dark
//!   one hovers on `surface1` and selects on `surface2`.
//! - Before: the projects section hovered on `surface3` in the dark (the
//!   light theme's rule), brighter than Paseo's dark sidebar.

use super::{Scheme, Tokens};
use gpui::Hsla;

/// A sidebar row's fills, in one theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SidebarSurfaces {
    /// `surfaceSidebar`: a row at rest.
    pub rest: Hsla,
    /// `surfaceSidebarHover`.
    pub hover: Hsla,
    /// `surfaceSidebarSelected`.
    pub selected: Hsla,
    /// `surface2`: a row while pressed (or dragged).
    pub pressed: Hsla,
}

impl SidebarSurfaces {
    /// `tokens`' sidebar fills.
    #[must_use]
    pub const fn of(tokens: &Tokens) -> Self {
        let neutral = tokens.neutral;
        let (hover, selected) = match tokens.scheme {
            Scheme::Light => (neutral.surface3, neutral.surface4),
            Scheme::Dark => (neutral.surface1, neutral.surface2),
        };
        Self {
            rest: neutral.surface_sidebar,
            hover,
            selected,
            pressed: neutral.surface2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_theme_maps_the_sidebar_fills_as_paseo_does() {
        // Paseo styles/theme.ts: light 269-271, dark 400-402.
        let light = Tokens::of(Scheme::Light);
        let fills = SidebarSurfaces::of(&light);
        assert_eq!(fills.rest, light.neutral.surface_sidebar);
        assert_eq!(fills.hover, light.neutral.surface3);
        assert_eq!(fills.selected, light.neutral.surface4);
        assert_eq!(fills.pressed, light.neutral.surface2);
        let dark = Tokens::of(Scheme::Dark);
        let fills = SidebarSurfaces::of(&dark);
        assert_eq!(fills.rest, dark.neutral.surface_sidebar);
        assert_eq!(fills.hover, dark.neutral.surface1);
        assert_eq!(fills.selected, dark.neutral.surface2);
        assert_eq!(fills.pressed, dark.neutral.surface2);
    }
}
