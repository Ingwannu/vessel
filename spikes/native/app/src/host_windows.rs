//! The sole process window registry, independent of desktop persistence or nodes.
//!
//! [Decision Log]
//! - Purpose: OS callbacks and desktop controls must discover the same live Shell.
//! - Constraint: windows can close or replace their root while an old Shell lives.
//! - Alternatives: per-feature lists, retaining strong Shells, or trusting handles.
//! - Choice: one weak record per handle, checked against the current kit root.
//! - Why: recipients cannot silently become a successor window or retained view.
//! - Cost: App-level dispatch returns Unavailable when no valid recipient exists.

use crate::shell::Shell;
use gpui::{AnyWindowHandle, App, Entity, Global, WeakEntity, Window};
use std::rc::Rc;

#[derive(Clone)]
pub(crate) struct WindowRecord {
    pub(crate) window: AnyWindowHandle,
    pub(crate) shell: WeakEntity<Shell>,
    pub(crate) close: WindowClose,
}

#[derive(Clone)]
pub(crate) enum WindowClose {
    Idle,
    Checking(Rc<()>),
}

impl WindowClose {
    #[must_use]
    pub(crate) fn begin() -> Self {
        Self::Checking(Rc::new(()))
    }

    #[must_use]
    pub(crate) const fn is_checking(&self) -> bool {
        matches!(self, Self::Checking(_))
    }

    /// Re-registering the same Shell after replacement must not revive its old
    /// close request. Value/handle identity alone cannot distinguish that cycle.
    #[must_use]
    pub(crate) fn same_check(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Checking(current), Self::Checking(captured)) => Rc::ptr_eq(current, captured),
            _ => false,
        }
    }
}

#[derive(Default)]
pub(crate) struct Registry {
    pub(crate) records: Vec<WindowRecord>,
    factory: Option<dispatch::Factory>,
}
impl Global for Registry {}

pub(crate) fn init(cx: &mut App) {
    if cx.has_global::<Registry>() {
        return;
    }
    cx.set_global(Registry::default());
    cx.on_window_closed(|cx, closed| {
        cx.global_mut::<Registry>()
            .records
            .retain(|record| record.window.window_id() != closed);
    })
    .detach();
}

/// Register the actual root content once during creation; never keep it alive.
pub(crate) fn register(shell: &Entity<Shell>, window: &Window, cx: &mut App) {
    init(cx);
    let handle = window.window_handle();
    let records = &mut cx.global_mut::<Registry>().records;
    if records
        .iter()
        .any(|record| record.window == handle && record.shell.entity_id() == shell.entity_id())
    {
        return;
    }
    // A deliberate replacement is a new recipient, even in the same window.
    records.retain(|record| record.window != handle);
    records.push(WindowRecord {
        window: handle,
        shell: shell.downgrade(),
        close: WindowClose::Idle,
    });
}

// These host operations gain product callers in the W7/E3 owner unions. The
// literal staging attributes are removed only with those real callers.
mod dispatch;
pub(crate) use dispatch::{Delivery, open_additional, set_factory, with_focused_shell};
mod enumeration;
pub(crate) use enumeration::{LiveShells, shells};
#[cfg(test)]
mod tests;
