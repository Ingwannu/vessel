//! Menus and the sole shortcut engine share these native window operations.

use crate::shell::Shell;
use gpui::{Context, Window};
use vessel_app_view::shortcuts::desktop::DesktopAction;

const MIN_STEP: i8 = -16;
const MAX_STEP: i8 = 16;
const HALF_LEVEL: f32 = 0.5;
const ZOOM_BASE: f32 = 1.2;

#[derive(Default)]
pub(super) struct Zoom {
    step: i8,
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
            DesktopAction::CloseWindow => window.remove_window(),
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
        self.step = match action {
            DesktopAction::ZoomIn => self.step.saturating_add(1).min(MAX_STEP),
            DesktopAction::ZoomOut => self.step.saturating_sub(1).max(MIN_STEP),
            DesktopAction::ActualSize => 0,
            DesktopAction::NewWindow | DesktopAction::CloseWindow | DesktopAction::Fullscreen => {
                return;
            }
        };
        let level = f32::from(self.step).mul_add(HALF_LEVEL, 0.0);
        window.set_scale_factor(ZOOM_BASE.powf(level).mul_add(native, 0.0));
    }
}
