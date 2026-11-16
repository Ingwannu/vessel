//! Global lifecycle admission requires every live window, not a filtered subset.

use super::{Registry, Shell};
use gpui::{App, WeakEntity};

/// Availability is a UI admission fact, not a storage or transport error.
#[must_use]
pub(crate) enum LiveShells {
    /// Every live window has exactly one current, registered Shell.
    Available(Vec<WeakEntity<Shell>>),
    /// A missing, replaced or unregistered root refuses global admission.
    Unavailable,
}

/// App-level only, after all Shell/Window borrows end. A temporary weak snapshot
/// is not another registry. The caller must act synchronously without awaiting.
pub(crate) fn shells(cx: &mut App) -> LiveShells {
    let live = cx.windows();
    let records = cx
        .try_global::<Registry>()
        .map_or_else(Vec::new, |registry| registry.records.clone());
    let mut shells = Vec::with_capacity(live.len());
    for window in live {
        let mut matching = records.iter().filter(|record| record.window == window);
        let Some(record) = matching.next() else {
            return LiveShells::Unavailable;
        };
        if matching.next().is_some() {
            return LiveShells::Unavailable;
        }
        let recipient = window
            .update(cx, |root, _window, cx| {
                record.shell.upgrade().and_then(|shell| {
                    vessel_app_kit::root_contains_view(&root, shell.entity_id(), cx)
                        .then(|| shell.downgrade())
                })
            })
            .ok()
            .flatten();
        let Some(recipient) = recipient else {
            return LiveShells::Unavailable;
        };
        shells.push(recipient);
    }
    LiveShells::Available(shells)
}
