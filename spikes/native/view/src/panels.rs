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

/// Paseo compact layouts are below 600 (`constants/layout.ts:36`), independent
/// of the native shell's 768-point phone chrome and other areas.
pub const SIDEBAR_COMPACT: f32 = 600.0;
/// Paseo settings split minimum: navigation 320 plus detail 400.
pub const SETTINGS_CONTENT_MINIMUM: f32 = 720.0;

/// Whether the sidebar uses a transient overlay or can dock beside content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarForm {
    /// Compact: its own sheet state, independent of desktop visibility.
    Sheet,
    /// Wide: persistent visibility and shared width budget apply.
    Docked,
}

/// What must remain usable beside the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarContext {
    /// The normal workspace, with an optional code panel.
    Workspace(Option<Placement>),
    /// Settings' list/detail content.
    Settings,
}

/// Whether the requested native sidebar and content minimum can coexist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sharing {
    /// Both fit after clamping the sidebar to its limits.
    Fits,
    /// Keeping the sidebar would reduce the content below its minimum.
    TooNarrow,
}

/// One computed sidebar placement, also consumed by Settings' list/back
/// fallback. Both surfaces keep navigation when docking cannot share the row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarLayout {
    /// Compact overlay, opened/closed independently by the shell.
    Sheet,
    /// Desktop preference is hidden or the content cannot share the row.
    Hidden,
    /// Docked width and the real budget result.
    Docked {
        /// Whole logical points after native frame insets and clamping.
        width: f32,
        /// Settings uses this result instead of copying a width constant.
        sharing: Sharing,
    },
}

/// Sidebar-only form factor, without changing the global phone breakpoint.
#[must_use]
pub fn sidebar_form(viewport: f32) -> SidebarForm {
    if !viewport.is_finite() || viewport < SIDEBAR_COMPACT {
        SidebarForm::Sheet
    } else {
        SidebarForm::Docked
    }
}

/// Admission minimum, separate from the width resolver's 400-point target.
/// Paseo `desktop-sidebar-layout.ts:66–72` admits ordinary content at zero;
/// an open native code panel keeps its separately documented budget.
#[must_use]
pub fn sidebar_minimum(context: SidebarContext) -> f32 {
    match context {
        SidebarContext::Workspace(None) => 0.0,
        SidebarContext::Workspace(Some(code)) => sidebar_leaves(Some(code)),
        SidebarContext::Settings => SETTINGS_CONTENT_MINIMUM,
    }
}

/// Reference `canDesktopAppSidebarShare`, measured inside the native frame.
#[must_use]
pub fn sidebar_share(room: f32, width: f32, context: SidebarContext) -> Sharing {
    if width.mul_add(-1.0, room) >= sidebar_minimum(context) {
        Sharing::Fits
    } else {
        Sharing::TooNarrow
    }
}

/// Compute form, width and auto-hiding once from typed route/preference facts.
/// A sidebar docks only when its content can share the row. Settings consumes
/// the same result for its list/back fallback, without a second width rule.
#[must_use]
pub fn sidebar_layout(
    viewport: f32,
    room: f32,
    asked: Option<f32>,
    preference: crate::prefs::Panel,
    context: SidebarContext,
) -> SidebarLayout {
    if sidebar_form(viewport) == SidebarForm::Sheet {
        return SidebarLayout::Sheet;
    }
    if preference == crate::prefs::Panel::Hidden {
        return SidebarLayout::Hidden;
    }
    // Width targeting must not become the admission test: Paseo keeps 400
    // when ordinary content admits zero (`desktop-sidebar-layout.ts:40–87`).
    // Settings targets a width that leaves its existing content minimum.
    let width = match context {
        SidebarContext::Workspace(_) => SIDEBAR.width(asked, room),
        SidebarContext::Settings => SIDEBAR.width_leaving(asked, room, SETTINGS_CONTENT_MINIMUM),
    };
    let sharing = sidebar_share(room, width, context);
    match sharing {
        Sharing::TooNarrow => SidebarLayout::Hidden,
        Sharing::Fits => SidebarLayout::Docked { width, sharing },
    }
}

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
        self.width_leaving(asked, room, self.beside)
    }

    /// [`Self::width`], leaving `beside` instead of the panel's own (the
    /// sidebar leaves more while the code panel is out, [`sidebar_leaves`]).
    #[must_use]
    pub fn width_leaving(self, asked: Option<f32>, room: f32, beside: f32) -> f32 {
        let roomy = beside.mul_add(-1.0, room).min(self.most).max(self.least);
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

/// Where the code panel is drawn on a desktop window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Beside the conversation, each keeping its least width.
    Beside,
    /// Over the conversation, at the row's right (Paseo's explorer on its
    /// compact layout, `presentation={isMobile ? "overlay" : "dock"}`).
    Over,
}

/// Where the code panel goes when the row, with the sidebar at its least,
/// is `row` points wide: beside the conversation when the row holds the
/// panel's least and the conversation's least, over it otherwise.
///
/// The 0.0.0.0.16 smoke test at 800 x 600: each panel kept its own least
/// and together they left the conversation 190 points (110 with the
/// sidebar dragged to 360), one character a line. Paseo shows the
/// sidebar only when the content keeps its minimum beside it
/// (`canDesktopAppSidebarShare`) and lays the explorer over the workspace
/// on its compact layout; here the conversation always keeps its least.
#[must_use]
pub fn code_placement(row: f32) -> Placement {
    if row >= CODE_PANEL.least.mul_add(1.0, CODE_PANEL.beside) {
        Placement::Beside
    } else {
        Placement::Over
    }
}

/// What the sidebar leaves beside it: the centre's least with no code
/// panel out; the panel's least and the conversation's while the panel is
/// beside the conversation; the conversation's while it is over it. The
/// sidebar gives way first (down to its least), so the conversation does
/// not.
#[must_use]
pub fn sidebar_leaves(panel: Option<Placement>) -> f32 {
    match panel {
        None => SIDEBAR.beside,
        Some(Placement::Beside) => CODE_PANEL.least.mul_add(1.0, CODE_PANEL.beside),
        Some(Placement::Over) => CODE_PANEL.beside,
    }
}

/// The code panel's width over the conversation: as asked (the default
/// when `None`), never wider than the row, at least its own least where
/// the row has it.
#[must_use]
pub fn over_width(asked: Option<f32>, row: f32) -> f32 {
    asked
        .unwrap_or(CODE_PANEL.default)
        .max(CODE_PANEL.least)
        .min(row)
}

#[cfg(test)]
mod tests {
    use super::{CODE_PANEL, Placement, SIDEBAR, code_placement, over_width, sidebar_leaves};

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

    /// The 0.0.0.0.16 smoke test: at 800 points both panels at their
    /// widest stored widths left the conversation 110 points. Now the
    /// panel goes over the conversation there, the sidebar leaves the
    /// conversation its 480, and at 1024 both fit beside it.
    #[test]
    fn a_narrow_window_keeps_the_conversation_s_least() {
        // 800 less the frame's 12 and the sidebar's least: 588.
        assert_eq!(code_placement(588.0), Placement::Over);
        // 1024 less 12 and 200: 812, room for 320 and 480.
        assert_eq!(code_placement(812.0), Placement::Beside);
        assert_eq!(code_placement(800.0), Placement::Beside);
        assert!(near(sidebar_leaves(None), 400.0));
        assert!(near(sidebar_leaves(Some(Placement::Beside)), 800.0));
        assert!(near(sidebar_leaves(Some(Placement::Over)), 480.0));
        // The sidebar stored at its widest (600) at 800 with the panel
        // over the conversation: 800 - 12 - 480 = 308.
        let sidebar =
            SIDEBAR.width_leaving(Some(600.0), 788.0, sidebar_leaves(Some(Placement::Over)));
        assert!(near(sidebar, 308.0), "{sidebar}");
        // At 1024 with the panel beside: 1012 - 800 = 212, then the row
        // (800) gives the panel its least and the conversation its 480.
        let sidebar =
            SIDEBAR.width_leaving(Some(600.0), 1012.0, sidebar_leaves(Some(Placement::Beside)));
        assert!(near(sidebar, 212.0), "{sidebar}");
        assert!(near(CODE_PANEL.width(Some(1600.0), 800.0), 320.0));
        // Over the conversation, the panel is as asked, never past the row.
        assert!(near(over_width(Some(1600.0), 480.0), 480.0));
        assert!(near(over_width(None, 480.0), 440.0));
        assert!(near(over_width(Some(100.0), 480.0), 320.0));
    }
}

#[cfg(test)]
mod sidebar_policy_tests {
    use super::*;
    use crate::prefs::Panel;
    #[test]
    fn sidebar_only_compact_boundary_is_six_hundred() {
        for width in [-1.0, 390.0, 599.99, f32::NAN] {
            assert_eq!(sidebar_form(width), SidebarForm::Sheet);
        }
        for width in [600.0, 650.0, 767.0, 768.0, 1280.0] {
            assert_eq!(sidebar_form(width), SidebarForm::Docked);
        }
    }
    #[test]
    fn ordinary_workspace_admits_zero_while_settings_keeps_its_navigation_exception() {
        let workspace = SidebarContext::Workspace(None);
        assert_eq!(
            sidebar_layout(600.0, 588.0, None, Panel::Shown, workspace),
            SidebarLayout::Docked {
                width: 200.0,
                sharing: Sharing::Fits
            }
        );
        assert_eq!(
            sidebar_layout(612.0, 600.0, None, Panel::Shown, workspace),
            SidebarLayout::Docked {
                width: 200.0,
                sharing: Sharing::Fits
            }
        );
        assert_eq!(
            sidebar_layout(800.0, 788.0, Some(600.0), Panel::Shown, workspace),
            SidebarLayout::Docked {
                width: 388.0,
                sharing: Sharing::Fits
            }
        );
        assert_eq!(
            sidebar_layout(800.0, 788.0, None, Panel::Shown, SidebarContext::Settings),
            SidebarLayout::Hidden
        );
        assert_eq!(
            sidebar_layout(932.0, 920.0, None, Panel::Shown, SidebarContext::Settings),
            SidebarLayout::Docked {
                width: 200.0,
                sharing: Sharing::Fits
            }
        );
        assert_eq!(
            sidebar_layout(
                1280.0,
                1268.0,
                Some(600.0),
                Panel::Shown,
                SidebarContext::Settings
            ),
            SidebarLayout::Docked {
                width: 548.0,
                sharing: Sharing::Fits
            }
        );
        assert_eq!(
            sidebar_layout(800.0, 788.0, None, Panel::Hidden, SidebarContext::Settings),
            SidebarLayout::Hidden
        );
        assert_eq!(
            sidebar_layout(390.0, 378.0, None, Panel::Hidden, workspace),
            SidebarLayout::Sheet
        );
    }
    #[test]
    fn open_explorer_keeps_four_hundred_width_target_and_separate_native_admission() {
        let context = SidebarContext::Workspace(Some(Placement::Over));
        assert_eq!(SIDEBAR.width(None, 588.0).to_bits(), 200.0_f32.to_bits());
        assert_eq!(
            sidebar_layout(600.0, 588.0, None, Panel::Shown, context),
            SidebarLayout::Hidden
        );
        // Admission must not silently shrink a requested 600 to 308 to fit:
        // the source width target is 400, independently of native admission.
        assert_eq!(
            SIDEBAR.width(Some(600.0), 788.0).to_bits(),
            388.0_f32.to_bits()
        );
        assert_eq!(
            sidebar_layout(800.0, 788.0, Some(600.0), Panel::Shown, context),
            SidebarLayout::Hidden
        );
        assert_eq!(
            sidebar_layout(800.0, 788.0, None, Panel::Shown, context),
            SidebarLayout::Docked {
                width: 264.0,
                sharing: Sharing::Fits,
            }
        );
        assert_eq!(
            sidebar_minimum(SidebarContext::Workspace(None)).to_bits(),
            0.0_f32.to_bits()
        );
    }
    #[test]
    fn code_budget_and_intended_native_default_are_not_redefined() {
        assert_eq!(
            sidebar_minimum(SidebarContext::Workspace(Some(Placement::Beside))).to_bits(),
            800.0_f32.to_bits()
        );
        assert_eq!(
            sidebar_minimum(SidebarContext::Workspace(Some(Placement::Over))).to_bits(),
            480.0_f32.to_bits()
        );
        assert_eq!(
            sidebar_minimum(SidebarContext::Settings).to_bits(),
            720.0_f32.to_bits()
        );
        assert_eq!(
            sidebar_layout(
                1280.0,
                1268.0,
                None,
                Panel::Shown,
                SidebarContext::Workspace(None)
            ),
            SidebarLayout::Docked {
                width: 264.0,
                sharing: Sharing::Fits
            }
        );
        assert_eq!(
            sidebar_share(900.0, 200.0, SidebarContext::Settings),
            Sharing::TooNarrow
        );
        assert_eq!(
            sidebar_share(920.0, 200.0, SidebarContext::Settings),
            Sharing::Fits
        );
    }
}
