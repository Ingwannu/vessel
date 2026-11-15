//! Menus and the sole shortcut engine share these native window operations.

use crate::shell::Shell;
use gpui::{Context, Window};
use vessel_app_view::desktop::zoom::{ZoomChange, ZoomLevel};
use vessel_app_view::shortcuts::desktop::DesktopAction;

#[derive(Default)]
pub(super) struct Zoom {
    level: ZoomLevel,
    native: Option<f32>,
}

impl Shell {
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
                if self.areas.shortcuts.page.recording() {
                    return false;
                }
                self.areas.desktop.zoom.apply(action, window);
            }
        }
        true
    }
}

impl Zoom {
    fn apply(&mut self, action: DesktopAction, window: &mut Window) {
        let native = *self.native.get_or_insert_with(|| window.scale_factor());
        let change = match action {
            DesktopAction::ZoomIn => ZoomChange::In,
            DesktopAction::ZoomOut => ZoomChange::Out,
            DesktopAction::ActualSize => ZoomChange::Reset,
            DesktopAction::NewWindow | DesktopAction::CloseWindow | DesktopAction::Fullscreen => {
                return;
            }
        };
        self.level.adjust(change);
        window.set_scale_factor(self.level.factor().mul_add(native, 0.0));
    }
}
