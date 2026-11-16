//! Keyboard admission for the existing command frame. The frame retains its
//! own arrows/Enter/Escape; this query neither resolves nor dispatches a key.
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};

impl Shell {
    pub(crate) fn toggle_palette(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.areas.palette.is_open() {
            self.close_palette(window, cx);
        } else {
            self.open_palette(window, cx);
        }
    }

    pub(crate) fn command_center_owns_key(&self, event: &gpui::KeystrokeEvent) -> bool {
        let modifiers = event.keystroke.modifiers;
        self.areas.palette.is_open()
            && !(modifiers.control || modifiers.alt || modifiers.shift || modifiers.platform)
            && matches!(
                event.keystroke.key.as_str(),
                "up" | "down" | "enter" | "escape"
            )
    }
}
