//! Deferred window effects stop when their original Shell leaves the root.
//! A connection lease cannot establish this UI lifetime; the kit owns root
//! identity and the host owner owns window selection.
use crate::shell::Shell;
use gpui::{AnyView, App, Context, WeakEntity, Window};

pub(super) fn update(
    this: &WeakEntity<Shell>,
    root: &AnyView,
    window: &mut Window,
    cx: &mut App,
    run: impl FnOnce(&mut Shell, &mut Window, &mut Context<'_, Shell>),
) -> bool {
    if !vessel_app_kit::root_contains_view(root, this.entity_id(), cx) {
        return false;
    }
    this.update(cx, |shell, cx| {
        if shell.areas.inbox.window != window.window_handle() {
            return false;
        }
        run(shell, window, cx);
        true
    })
    .unwrap_or(false)
}
