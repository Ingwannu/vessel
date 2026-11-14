//! How wide the side panels are: the sidebar at the left and the code
//! panel at the right, each dragged at its inner edge (the owner,
//! 2026-11-14, word 29: "make both the left and right sidebars
//! resizable"). Paseo's rules (`desktop-sidebar-layout.ts`,
//! `explorer-sidebar-layout.ts`, `stores/panel-store/state.ts`): a panel's
//! width is the one asked for, kept between its least and its most, and
//! never so wide that what lies beside it gets less than its own least
//! width; when the window is too narrow for both, the panel's least wins.
//!
//! [Decision Log]
//! - Purpose: one rule for both panels, under test, so the kit's handle
//!   only reports where the pointer is.
//! - Values: the sidebar as Paseo's (least 200, most 600, the centre's
//!   least 400); its default stays Vessel's 264 (Paseo's 320: the shell
//!   was laid out at 264 and the owner asked for resizing, not a wider
//!   default). The code panel keeps track F's values (default 440, least
//!   320, the conversation's least 480; Paseo's Explorer: 320, 240, 360)
//!   because it shows a file's diff inside the panel, which Paseo opens in
//!   the main pane.
//! - Kept: the width as whole points in the preferences, per panel
//!   (Paseo keeps the Explorer's per workspace; here one width per panel,
//!   the coordinator's ask), `None` until dragged; a double-click on the
//!   edge returns to the default (an addition: Paseo has none).

/// A panel's limits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    /// The narrowest it is dragged to.
    pub least: f32,
    /// The widest.
    pub most: f32,
    /// Its width until dragged.
    pub default: f32,
    /// The least width left for what lies beside it.
    pub beside: f32,
}

/// The sidebar.
pub const SIDEBAR: Limits = Limits {
    least: 200.0,
    most: 600.0,
    default: 264.0,
    beside: 400.0,
};

/// The code panel beside a session.
pub const CODE_PANEL: Limits = Limits {
    least: 320.0,
    most: 1600.0,
    default: 440.0,
    beside: 480.0,
};

impl Limits {
    /// The width drawn for `asked` (the default when `None`) in `room`
    /// points (the window's width for the sidebar, the row's for the code
    /// panel): within its least and most, leaving `beside` to the rest;
    /// its least when the room is too small for both.
    #[must_use]
    pub fn width(self, asked: Option<f32>, room: f32) -> f32 {
        let roomy = self
            .beside
            .mul_add(-1.0, room)
            .min(self.most)
            .max(self.least);
        asked.unwrap_or(self.default).min(roomy).max(self.least)
    }

    /// `width` as kept in the preferences: whole points; `None` for the
    /// default (a double-click back to it forgets the width).
    #[must_use]
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the width is rounded and clamped to u16's range before the cast"
    )]
    pub fn kept(self, width: f32) -> Option<u16> {
        let points = width.round().clamp(0.0, f32::from(u16::MAX)) as u16;
        let off = f32::from(points).mul_add(1.0, self.default.mul_add(-1.0, 0.0));
        (off.abs() >= 0.5).then_some(points)
    }
}

#[cfg(test)]
mod tests {
    use super::{CODE_PANEL, SIDEBAR};

    fn near(a: f32, b: f32) -> bool {
        a.mul_add(1.0, b.mul_add(-1.0, 0.0)).abs() < 1e-3
    }

    /// Paseo's rule: the asked width, kept between the limits, and never
    /// leaving the centre less than its least; the least wins when the
    /// window is too narrow.
    #[test]
    fn a_panel_keeps_its_limits_and_leaves_room_beside_it() {
        assert!(near(SIDEBAR.width(None, 1280.0), 264.0));
        assert!(near(SIDEBAR.width(Some(150.0), 1280.0), 200.0));
        assert!(near(SIDEBAR.width(Some(900.0), 1280.0), 600.0));
        assert!(
            near(SIDEBAR.width(Some(700.0), 900.0), 500.0),
            "900 less the centre's 400"
        );
        assert!(
            near(SIDEBAR.width(Some(500.0), 500.0), 200.0),
            "too narrow: the least"
        );
        assert!(near(CODE_PANEL.width(None, 1000.0), 440.0));
        assert!(near(CODE_PANEL.width(Some(700.0), 1000.0), 520.0));
        assert!(near(CODE_PANEL.width(Some(100.0), 1000.0), 320.0));
    }

    /// A width is kept in whole points; the default is not kept.
    #[test]
    fn a_width_is_kept_in_whole_points_and_the_default_forgotten() {
        assert_eq!(SIDEBAR.kept(301.4), Some(301));
        assert_eq!(SIDEBAR.kept(264.2), None);
        assert_eq!(CODE_PANEL.kept(440.0), None);
        assert_eq!(CODE_PANEL.kept(512.6), Some(513));
    }
}
