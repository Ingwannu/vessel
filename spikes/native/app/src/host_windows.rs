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

#[derive(Clone)]
pub(crate) struct WindowRecord {
    pub(crate) window: AnyWindowHandle,
    pub(crate) shell: WeakEntity<Shell>,
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
    });
}

// W7 supplies real generic Inbox callers. Only the desktop factory setter
// remains staged until the E3 startup owner installs it.
mod dispatch;
#[cfg(test)]
pub(crate) use dispatch::set_factory;
pub(crate) use dispatch::{Delivery, open_additional, with_focused_shell};
#[cfg(test)]
mod tests;
