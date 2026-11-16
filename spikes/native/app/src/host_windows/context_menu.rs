//! Original-window editing delivery uses the sole weak registry, never fallback routing.

use super::{Registry, Shell, WindowRecord};
use gpui::{App, Context, Window};
use std::rc::Rc;
use vessel_app_kit::field::context_menu::{Handler, Request};

#[derive(Clone, Copy)]
pub(crate) struct Callbacks {
    pub(crate) admitted: fn(&Shell) -> bool,
    pub(crate) open: fn(&mut Shell, Request, &mut Window, &mut Context<'_, Shell>) -> bool,
}

/// E3 supplies its normal recorder predicate and existing menu operation. This
/// adapter adds no keyboard listener, clipboard engine or window registry.
pub(crate) fn handler(callbacks: Callbacks) -> Handler {
    Rc::new(move |request, window, cx| {
        if !request.current(window, cx) {
            return;
        }
        let Some(record) = original(window, cx) else {
            return;
        };
        // The input callback already owns Window. Enter its handle only after
        // that borrow ends, and never substitute the currently focused window.
        cx.defer(move |cx| {
            let _closed = record.window.update(cx, |root, window, cx| {
                let Some(shell) = record.shell.upgrade() else {
                    return;
                };
                if !registered(&record, cx)
                    || !vessel_app_kit::root_contains_view(&root, shell.entity_id(), cx)
                    || !request.current(window, cx)
                {
                    return;
                }
                let selection = request.clone();
                let opened = shell.update(cx, |shell, cx| {
                    if (callbacks.admitted)(shell) {
                        (callbacks.open)(shell, request, window, cx)
                    } else {
                        false
                    }
                });
                if opened {
                    selection.retain_selection(window, cx);
                }
            });
        });
    })
}

fn original(window: &Window, cx: &App) -> Option<WindowRecord> {
    let mut matching = cx
        .try_global::<Registry>()?
        .records
        .iter()
        .filter(|record| record.window == window.window_handle());
    let record = matching.next()?;
    matching.next().is_none().then(|| record.clone())
}

fn registered(record: &WindowRecord, cx: &App) -> bool {
    cx.try_global::<Registry>().is_some_and(|registry| {
        let mut matching = registry
            .records
            .iter()
            .filter(|current| current.window == record.window);
        matching
            .next()
            .is_some_and(|current| current.shell.entity_id() == record.shell.entity_id())
            && matching.next().is_none()
    })
}

#[cfg(test)]
mod tests;
