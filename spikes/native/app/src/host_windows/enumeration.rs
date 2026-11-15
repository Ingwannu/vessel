//! Global lifecycle admission requires every live window, not a filtered subset.

use super::{Registry, Shell};
use gpui::{App, WeakEntity};

/// A missing, replaced or unregistered root cannot grant global admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Unavailable;

/// App-level only, after all Shell/Window borrows end. A temporary weak snapshot
/// is not another registry. The caller must act synchronously without awaiting.
pub(crate) fn shells(cx: &mut App) -> Result<Vec<WeakEntity<Shell>>, Unavailable> {
    let live = cx.windows();
    let records = cx
        .try_global::<Registry>()
        .map_or_else(Vec::new, |registry| registry.records.clone());
    let mut shells = Vec::with_capacity(live.len());
    for window in live {
        let mut matching = records.iter().filter(|record| record.window == window);
        let record = matching.next().ok_or(Unavailable)?;
        if matching.next().is_some() {
            return Err(Unavailable);
        }
        let recipient = window
            .update(cx, |root, _window, cx| {
                let shell = record.shell.upgrade().ok_or(Unavailable)?;
                if !vessel_app_kit::root_contains_view(&root, shell.entity_id(), cx) {
                    return Err(Unavailable);
                }
                Ok(shell.downgrade())
            })
            .map_err(|_closed| Unavailable)??;
        shells.push(recipient);
    }
    Ok(shells)
}
