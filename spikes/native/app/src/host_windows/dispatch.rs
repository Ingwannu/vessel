//! Host UI routing uses the same registry and rechecks the original root at use.

use super::{Registry, Shell, WindowRecord, init};
use gpui::{App, Context, Window};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(crate) enum Delivery {
    Delivered,
    Unavailable,
}

pub(super) type Factory = fn(&mut App) -> Delivery;

/// Only the desktop owner installs a factory; it creates a window directly,
/// never recursively calls this module's open_additional entry.
pub(crate) fn set_factory(factory: Factory, cx: &mut App) {
    init(cx);
    cx.global_mut::<Registry>().factory = Some(factory);
}

/// An absent desktop factory is an explicit native capability refusal.
pub(crate) fn open_additional(cx: &mut App) -> Delivery {
    match cx
        .try_global::<Registry>()
        .and_then(|registry| registry.factory)
    {
        Some(factory) => factory(cx),
        None => Delivery::Unavailable,
    }
}

/// App-level only, after Shell borrows end. The callback is generic UI work;
/// notice/session authority remains with the caller and is not transferred here.
pub(crate) fn with_focused_shell(
    cx: &mut App,
    run: impl FnOnce(&mut Shell, &mut Window, &mut Context<'_, Shell>),
) -> Delivery {
    let Some(record) = target(cx) else {
        return Delivery::Unavailable;
    };
    record
        .window
        .update(cx, |root, window, cx| {
            let Some(shell) = record.shell.upgrade() else {
                return Delivery::Unavailable;
            };
            if !vessel_app_kit::root_contains_view(&root, shell.entity_id(), cx) {
                return Delivery::Unavailable;
            }
            shell.update(cx, |shell, cx| run(shell, window, cx));
            Delivery::Delivered
        })
        .unwrap_or(Delivery::Unavailable)
}

fn target(cx: &mut App) -> Option<WindowRecord> {
    let active = cx.active_window();
    let live = cx.windows();
    let records = cx.try_global::<Registry>()?.records.clone();
    let mut valid = Vec::new();
    for record in records {
        if !live.contains(&record.window) {
            continue;
        }
        let visible = record
            .window
            .update(cx, |root, window, cx| {
                record.shell.upgrade().and_then(|shell| {
                    vessel_app_kit::root_contains_view(&root, shell.entity_id(), cx)
                        .then_some(window.is_visible())
                })
            })
            .ok()
            .flatten();
        if let Some(visible) = visible {
            valid.push((record, visible));
        }
    }
    valid
        .iter()
        .find(|(record, _)| Some(record.window) == active)
        .or_else(|| valid.iter().find(|(_, visible)| *visible))
        .or_else(|| valid.first())
        .map(|(record, _)| record.clone())
}
