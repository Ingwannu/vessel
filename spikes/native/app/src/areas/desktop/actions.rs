//! Menus and the sole shortcut engine share supported native window operations.

use crate::shell::Shell;
use gpui::{Context, Window};
use vessel_app_view::shortcuts::desktop::DesktopAction;

impl Shell {
    /// Native clicks and menus respect the sole shortcut recorder's admission.
    #[must_use]
    pub(crate) fn desktop_commands_admitted(&self) -> bool {
        !self.areas.shortcuts.page.recording()
    }

    /// Called by the recorder before capture/blur, not from rendering. Retire
    /// the old shared-menu lease before it can restore focus from a stale frame.
    pub(crate) fn desktop_recording_started(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.desktop.menu.close_and_restore(window, cx);
        self.areas.desktop.menu_cycle = std::rc::Rc::new(());
    }

    /// GPUI has no production whole-viewport zoom seam yet; never advertise a test helper.
    #[must_use]
    pub(crate) fn desktop_shortcut_supported(action: DesktopAction) -> bool {
        match action {
            DesktopAction::NewWindow | DesktopAction::CloseWindow | DesktopAction::Fullscreen => {
                true
            }
            DesktopAction::ZoomIn | DesktopAction::ZoomOut | DesktopAction::ActualSize => false,
        }
    }

    /// Native menu clicks use exactly the same operation as matched shortcuts.
    pub(crate) fn run_desktop_shortcut(
        &mut self,
        action: DesktopAction,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if !self.desktop_commands_admitted() || !Self::desktop_shortcut_supported(action) {
            return false;
        }
        match action {
            DesktopAction::NewWindow => crate::desktop::open_additional(cx),
            DesktopAction::CloseWindow => vessel_app_kit::chrome::request_close(window, cx),
            DesktopAction::Fullscreen => window.toggle_fullscreen(),
            DesktopAction::ZoomIn | DesktopAction::ZoomOut | DesktopAction::ActualSize => {
                return false;
            }
        }
        true
    }
}
