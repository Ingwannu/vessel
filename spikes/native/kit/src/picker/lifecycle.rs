//! Private admission for a particular rendered opening of one combobox.
//!
//! [Decision Log]
//! - Purpose: old rendered A cannot close, choose in, highlight or refocus B.
//! - Existing assets: toolkit state identity and shared weak `ReturnFocus` policy.
//! - Choice: each explicit open/drill resets a private `Rc`; callbacks retain only
//!   `Weak` identity and check it before every state/action/focus side effect.
//! - Why/cost: retained `Weak` prevents allocation ABA without a wrapping counter;
//!   close refuses admission, while identity remains for actual-close focus ACK.
use super::ComboboxState;
use gpui::{App, Window};
use std::rc::{Rc, Weak};
#[derive(Debug, Clone)]
pub(super) struct RenderLease(Weak<()>);
impl ComboboxState {
    /// Open it, its search emptied and holding the keys, showing `search`
    /// while empty (or the list holding the keys, with no search row).
    pub fn open(&mut self, search: Option<&str>, window: &mut Window, cx: &mut App) {
        self.capture_return_focus(window, cx);
        self.cycle = Rc::new(());
        self.open = true;
        self.highlighted = None;
        self.highlighted_label = None;
        self.search.clear(window, cx);
        if let Some(placeholder) = search {
            self.search.set_placeholder(placeholder, window, cx);
            self.search.focus(window, cx);
        } else {
            self.focus.focus(window, cx);
        }
    }

    /// Open it as a page beside its menu (a flyout, [`crate::submenu`]): its
    /// rows take presses, while the keys stay with the menu that opened it
    /// (no focus moves, none is returned on close).
    pub fn open_beside(&mut self) {
        self.cycle = Rc::new(());
        self.open = true;
        self.highlighted = None;
        self.highlighted_label = None;
    }

    /// Close it (no longer `const`: it drops the highlighted row's label).
    pub fn close(&mut self) {
        self.open = false;
        self.highlighted = None;
        self.highlighted_label = None;
    }

    /// Start its search over, showing `placeholder` while empty (a way
    /// back, a row's own list).
    pub fn reset_query(&mut self, placeholder: &str, window: &mut Window, cx: &mut App) {
        self.cycle = Rc::new(());
        self.highlighted = None;
        self.highlighted_label = None;
        self.search.clear(window, cx);
        self.search.set_placeholder(placeholder, window, cx);
        self.search.focus(window, cx);
    }

    pub(super) fn render_lease(&self) -> RenderLease {
        RenderLease(Rc::downgrade(&self.cycle))
    }
    pub(super) fn same_cycle(&self, lease: &RenderLease) -> bool {
        Weak::ptr_eq(&Rc::downgrade(&self.cycle), &lease.0)
    }
    pub(super) fn accepts(&self, lease: &RenderLease) -> bool {
        self.is_open() && self.same_cycle(lease)
    }
    pub(super) fn restore_choice_if_current(
        &mut self,
        lease: &RenderLease,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.same_cycle(lease) {
            self.restore_after_choice(window, cx);
        }
    }
}

#[cfg(test)]
mod tests;
