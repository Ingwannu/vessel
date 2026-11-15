//! Menus and the sole shortcut engine share supported native window operations.

use crate::shell::Shell;
use gpui::{Context, Window};
use vessel_app_view::shortcuts::desktop::DesktopAction;

impl Shell {
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
