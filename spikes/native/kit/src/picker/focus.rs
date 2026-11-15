//! Return to the opener without keeping removed fields alive or displacing a
//! newer dialog/field. Both searchable and plain lists own focus while open.
//!
//! [Decision Log]
//! - Purpose: an unmounted list's focus must not swallow the caller's shortcuts.
//! - Constraint: callbacks can drill into another list or open a new dialog.
//! - Choice: weak opener capture once, state-only close kept compatible, and
//!   restore after final choices only while this list/search still owns focus.
//! - Alternative: each area manually focuses its root after every dismissal.
//!   That duplicates policy and steals focus from a newly opened field.
//! - Cost: one weak handle per list; no renderer or geometry duplication.
use super::ComboboxState;
use gpui::{App, Window};
impl ComboboxState {
    /// Close and return keys to the opener if this list still owns them.
    /// A newer field/dialog that took focus is preserved.
    pub fn close_and_restore(&mut self, window: &mut Window, cx: &mut App) {
        self.close();
        self.restore_after_choice(window, cx);
    }
    fn owns_focus(&self, window: &Window, cx: &App) -> bool {
        self.focus.is_focused(window) || self.search.focus_handle(cx).is_focused(window)
    }
    pub(super) fn capture_return_focus(&mut self, window: &Window, cx: &App) {
        if !self.is_open() && !self.owns_focus(window, cx) {
            self.return_to = window.focused(cx).map(|focus| focus.downgrade());
        }
    }
    pub(super) fn restore_after_choice(&mut self, window: &mut Window, cx: &mut App) {
        if self.is_open() {
            return;
        }
        let previous = self.return_to.take().and_then(|weak| weak.upgrade());
        if !self.owns_focus(window, cx) {
            return;
        }
        match previous {
            Some(focus) => focus.focus(window, cx),
            None => window.blur(cx),
        }
    }
}
#[cfg(test)]
mod tests;
