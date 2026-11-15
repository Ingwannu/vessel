//! Dialog uses the one shared weak-return policy; its caller decides actual close.
use super::DialogState;
use gpui::{App, Window};
impl DialogState {
    /// Sibling adaptive controls share this focus owner and its return policy.
    pub(crate) const fn focus_handle(&self) -> &gpui::FocusHandle {
        &self.focus
    }
    /// Take keyboard input, remembering the original opener once per cycle.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.opener.remember(window, cx);
        self.focus.focus(window, cx);
    }
    /// Acknowledge actual dismissal after the feature accepted its identity.
    /// Pending/stale refusals must not call this; a newer focus is preserved.
    pub fn dismissed(&self, window: &mut Window, cx: &mut App) {
        self.opener.restore_if_owned(&self.focus, window, cx);
        self.sheet.clear();
    }
}
#[cfg(test)]
mod tests;
