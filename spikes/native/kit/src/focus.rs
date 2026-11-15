//! Shared actual-close focus restoration, with weak ownership and no domain decisions.
//!
//! [Decision Log]
//! - Purpose: closing a dialog should return keyboard input to its opener.
//! - Constraint: unit close callbacks may refuse pending or stale dismissal;
//!   the shared renderer cannot infer whether a feature actually closed.
//! - Alternatives: always restore after close, retain a strong handle, or
//!   capture a new opener whenever a pending dialog takes focus again.
//! - Choice: capture once per open cycle and restore only on explicit
//!   dismissed acknowledgement, while this dialog still owns current focus.
//! - Why: old cleanup cannot steal focus from a newer field or dialog, and
//!   deleting an opener cannot keep its input alive or create a focus loop.
//! - Cost: consumers call dismissed only after their actual-close decision.
use gpui::{App, FocusHandle, WeakFocusHandle, Window};
use std::cell::RefCell;

#[derive(Debug, Default)]
enum OpenCycle {
    #[default]
    Unopened,
    Captured(Option<WeakFocusHandle>),
}

/// One overlay's weak opener, remembered once and consumed only on actual close.
/// Controls retain their own focus handle and dismissal/identity policy.
#[derive(Debug, Default)]
pub struct ReturnFocus(RefCell<OpenCycle>);
impl ReturnFocus {
    /// Remember before taking focus. Captured absence is still a capture:
    /// repeated focus during this cycle never replaces its original opener.
    pub fn remember(&self, window: &Window, cx: &App) {
        let unopened = matches!(*self.0.borrow(), OpenCycle::Unopened);
        if unopened {
            *self.0.borrow_mut() =
                OpenCycle::Captured(window.focused(cx).map(|focus| focus.downgrade()));
        }
    }
    /// Consume an actual-close cycle and restore only while `owner` or one
    /// of its children still owns focus. A newer focus is preserved, even
    /// though this cycle is cleared. A dead/missing opener blurs the owner.
    /// Pending/stale close refusals must not call this acknowledgement.
    pub fn restore_if_owned(&self, owner: &FocusHandle, window: &mut Window, cx: &mut App) {
        let old = std::mem::replace(&mut *self.0.borrow_mut(), OpenCycle::Unopened);
        let OpenCycle::Captured(opener) = old else {
            return;
        };
        if !owner.contains_focused(window, cx) {
            return;
        }
        match opener.and_then(|weak| weak.upgrade()) {
            Some(opener) if !owner.contains(&opener, window) => opener.focus(window, cx),
            // An owner/child accidentally focused before remember is not an
            // external opener. Do not return focus into a dismissed subtree.
            Some(_) | None => window.blur(cx),
        }
    }
}
