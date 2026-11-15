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
//! - Cost: G4's shared `ReturnFocus` policy; no renderer or geometry duplication.
use super::ComboboxState;
use gpui::{App, Window};
impl ComboboxState {
    /// Close and return keys to the opener if this list still owns them.
    /// A newer field/dialog that took focus is preserved.
    pub fn close_and_restore(&mut self, window: &mut Window, cx: &mut App) {
        self.close();
        self.restore_after_choice(window, cx);
    }
    pub(super) fn capture_return_focus(&mut self, window: &Window, cx: &App) {
        if !self.is_open() {
            // A new opening after a state-only legacy close starts a new cycle.
            self.return_to = crate::focus::ReturnFocus::default();
            self.return_to.remember(window, cx);
        }
    }
    pub(super) fn restore_after_choice(&mut self, window: &mut Window, cx: &mut App) {
        if self.is_open() {
            return;
        }
        let search = self.search.focus_handle(cx);
        let owner = if search.contains_focused(window, cx) {
            search
        } else {
            self.focus.clone()
        };
        self.return_to.restore_if_owned(&owner, window, cx);
    }
}
#[cfg(test)]
mod tests;
