//! One process callback, retaining only bounded per-notice recipients. A later
//! window must not replace the callback or inherit an earlier window's source.
use crate::{routes::Route, shell::Shell};
use gpui::{AnyWindowHandle, App, Context, Global, WeakEntity, Window};
use std::collections::VecDeque;
use vessel_app_link::inbox::Target;
use vessel_app_view::inbox::{
    Source,
    notices::{DeliveryTag, Destination, MAX_PENDING_NOTICES},
};

struct Recipient {
    window: AnyWindowHandle,
    shell: WeakEntity<Shell>,
    source: Source,
    destination: Destination,
}
#[derive(Default)]
struct Routes {
    installed: bool,
    entries: VecDeque<(DeliveryTag, Recipient)>,
}
impl Global for Routes {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opened {
    Original,
    /// A current host receives only generic Inbox activation, never this source.
    #[cfg(not(target_arch = "wasm32"))]
    Generic,
    GenericUnavailable,
    OtherNotification,
}

pub(super) fn listen(cx: &mut App) {
    let routes = cx.default_global::<Routes>();
    if std::mem::replace(&mut routes.installed, true) {
        return;
    }
    cx.on_system_notification_response(|response, cx| {
        if open(response.tag.as_ref(), cx) == Opened::GenericUnavailable {
            tracing::debug!("notification inbox fallback is unavailable");
        }
    });
}

pub(super) fn remember(
    tag: DeliveryTag,
    source: Source,
    destination: impl Into<Destination>,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) {
    listen(cx);
    let recipient = Recipient {
        window: window.window_handle(),
        shell: cx.entity().downgrade(),
        source,
        destination: destination.into(),
    };
    let routes = cx.default_global::<Routes>();
    routes.entries.retain(|(known, _)| *known != tag);
    if routes.entries.len() == MAX_PENDING_NOTICES {
        let _evicted = routes.entries.pop_front();
    }
    routes.entries.push_back((tag, recipient));
}

fn open(tag: &str, cx: &mut App) -> Opened {
    let Some(tag) = super::notifications::parse_tag(tag) else {
        return if tag == super::notifications::SUMMARY_TAG {
            generic(cx)
        } else {
            Opened::OtherNotification
        };
    };
    // End the global borrow before entering a window or updating an entity.
    let recipient = {
        let routes = cx.default_global::<Routes>();
        routes
            .entries
            .iter()
            .position(|(known, _)| *known == tag)
            .and_then(|index| routes.entries.remove(index))
            .map(|(_, recipient)| recipient)
    };
    let Some(recipient) = recipient else {
        return generic(cx);
    };
    let opened = recipient.window.update(cx, |root, window, cx| {
        if !vessel_app_kit::root_contains_view(&root, recipient.shell.entity_id(), cx) {
            return false;
        }
        recipient
            .shell
            .update(cx, |shell, cx| {
                if shell.areas.inbox.window != window.window_handle() {
                    return false;
                }
                shell.route_to(Route::Inbox);
                if shell.areas.inbox.targets.iter().any(
                |target| matches!(target, Target::Online(source) if *source == recipient.source),
            ) {
                match recipient.destination {
                    Destination::Session(key) if key.node == recipient.source.node() => {
                        shell.inbox_select(key, cx);
                    }
                    Destination::Terminal(key) if key.node == recipient.source.node() => {
                        shell.areas.inbox.terminals.selected = Some(key);
                    }
                    Destination::Session(_) | Destination::Terminal(_) => {}
                }
            }
                // Retired connection: open only the original Inbox, without an
                // exact-source action on an alias's replacement admission.
                cx.notify();
                cx.activate(true);
                window.activate_window();
                true
            })
            .unwrap_or(false)
    });
    if matches!(opened, Ok(true)) {
        Opened::Original
    } else {
        generic(cx)
    }
}

// The host registry owns window selection. No notice source or destination is
// passed through this boundary, so a successor cannot inherit its authority.
#[cfg(not(target_arch = "wasm32"))]
fn generic(cx: &mut App) -> Opened {
    use crate::host_windows::{Delivery, open_additional, with_focused_shell};
    if with_focused_shell(cx, show_inbox) == Delivery::Delivered {
        return Opened::Generic;
    }
    if open_additional(cx) == Delivery::Delivered
        && with_focused_shell(cx, show_inbox) == Delivery::Delivered
    {
        Opened::Generic
    } else {
        Opened::GenericUnavailable
    }
}

// The browser has no desktop host registry or additional-window factory.
// Preserve an explicit unavailable fallback instead of selecting another owner.
#[cfg(target_arch = "wasm32")]
fn generic(_: &mut App) -> Opened {
    Opened::GenericUnavailable
}

#[cfg(not(target_arch = "wasm32"))]
fn show_inbox(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    shell.route_to(Route::Inbox);
    cx.notify();
    cx.activate(true);
    window.activate_window();
}

#[cfg(test)]
mod tests;
