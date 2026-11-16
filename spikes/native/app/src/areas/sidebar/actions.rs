//! Sidebar actions reuse the shell's existing panel state and lifecycle.
//!
//! Paseo workspace-screen.tsx:2632–2657 delegates to
//! utils/desktop-sidebar-toggle.ts:15–30. Off a workspace, _layout.tsx:398–411
//! toggles only the agent list. Desktop intent reads the saved open state,
//! including when a narrow width temporarily prevents sharing the row.

use crate::{routes::Route, shell::Shell};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::panels::SidebarLayout;

impl Shell {
    /// Collapse every open side, or restore both when neither is open.
    /// W22 owns dispatch and the subsequent W12 focus-mode exit; this method
    /// neither installs an observer nor creates another panel preference.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "W22 consumes the checked owner action seam")
    )]
    pub(crate) fn sidebar_toggle_both(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if self.route != Route::Session || self.areas.session.open.is_none() {
            self.toggle_sidebar(window, cx);
            return true;
        }
        let left = match super::arrangement(self, window) {
            SidebarLayout::Sheet => self.sidebar_open(),
            SidebarLayout::Hidden | SidebarLayout::Docked { .. } => self.sidebar_shown(),
        };
        let right = self.areas.workspace.code.open;
        let show = !left && !right;
        if left != show {
            self.toggle_sidebar(window, cx);
        }
        if right != show {
            self.toggle_code(window, cx);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::language_tests;
    use gpui::TestAppContext;
    use vessel_app_view::prefs::Panel;

    #[gpui::test]
    fn sidebar_both_action_closes_only_open_sides_or_opens_both(cx: &mut TestAppContext) {
        language_tests::init(cx);
        for (left, right, shown) in [
            (false, false, true),
            (true, false, false),
            (false, true, false),
            (true, true, false),
        ] {
            let (shell, cx) = super::super::policy_tests::setup(cx, 1280.0, Route::Session);
            shell.update_in(cx, |shell, window, cx| {
                shell.areas.session.open = Some(crate::areas::session::Open {
                    machine: "fixture".to_owned(),
                    session: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
                    title: "Workspace".to_owned(),
                    agent: "fixture".to_owned(),
                    structured: false,
                    live: false,
                    archived_at: None,
                    binding: None,
                });
                shell.prefs.settings.layout.sidebar =
                    if left { Panel::Shown } else { Panel::Hidden };
                shell.areas.workspace.code.open = right;
                assert!(shell.sidebar_toggle_both(window, cx));
                assert_eq!(shell.sidebar_shown(), shown);
                assert_eq!(shell.areas.workspace.code.open, shown);
            });
        }
    }

    #[gpui::test]
    fn sidebar_both_action_off_workspace_only_toggles_left(cx: &mut TestAppContext) {
        language_tests::init(cx);
        let (shell, cx) = super::super::policy_tests::setup(cx, 1280.0, Route::Draft);
        shell.update_in(cx, |shell, window, cx| {
            shell.prefs.settings.layout.sidebar = Panel::Hidden;
            shell.areas.workspace.code.open = true;
            assert!(shell.sidebar_toggle_both(window, cx));
            assert!(shell.sidebar_shown());
            assert!(shell.areas.workspace.code.open);
        });
    }
}
