//! The label manager's host choice: Paseo's `HostFilter`
//! (`components/hosts/host-filter.tsx`), as `manager-modal.tsx` 188-197 uses
//! it, shown only when there is more than one host and without "All hosts".
//!
//! [Decision Log]
//! - Purpose: the manager edits one host's catalog; with several hosts it
//!   picks which one from a pill that opens the host list, not from a row of
//!   host-named buttons (the coordinator's RETURN on the manager body,
//!   2026-11-16).
//! - Reference: the trigger is the host's status dot (8 in a 14 slot), its
//!   name in 14 medium `foreground` and a 14 muted chevron, gap 6, padding
//!   6 by 12, radius 6, `surface1` with a `border` hairline, `surface2` on
//!   hover and `surface3` while pressed, sized to its content. The list is
//!   Paseo's host combobox under the trigger (`bottom-start`), one row per
//!   host with its status dot and a check on the one shown, no search.
//! - Alternatives: the kit select (`select.rs`: the outline settings chrome,
//!   not this filled pill); the composer pill (`picker::pill`: transparent,
//!   no border); a kit `host_filter` (Paseo shares it with History,
//!   Schedules and Usage, but no Vessel screen has those filters yet).
//! - Choice: the trigger is drawn here and the list is the kit's combobox
//!   (`vessel_app_kit::combobox`), so no second list exists. When a second
//!   screen needs the same filter, the trigger moves to the kit unchanged.
//! - Impact: a book entry not yet resolved to a node is listed as a disabled
//!   row with the offline dot, as Paseo lists every configured host.

use std::rc::Rc;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::radius::MD;
use vessel_app_kit::theme::spacing::{S1_5, S3};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement as _, ParentElement as _,
    Stateful, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ComboHandlers, ComboHeader, ComboRow, Leading, Side, Tokens, Type, TypeExt as _,
    anchor_probe, combobox, dot, glyph, select,
};

use super::Labels;
use crate::shell::Shell;

/// `HostStatusDot`: 8 across, in an `iconSize.sm` slot.
const DOT: f32 = 8.0;
const DOT_SLOT: f32 = 14.0;
/// The chevron's size.
const CARET: f32 = 14.0;

/// More than one host: Paseo's `hosts.length > 1`, counting book entries
/// not yet resolved to a node as hosts too.
pub(super) fn shown(state: &Labels) -> bool {
    state.hosts.len().saturating_add(state.unresolved.len()) > 1
}

/// The pill that names the host shown and opens the host list.
pub(super) fn trigger(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Stateful<Div> {
    let state = &shell.areas.labels;
    let neutral = tokens.neutral;
    let label = state
        .route
        .as_ref()
        .map_or_else(|| shell.words.labels().host().to_owned(), |route| route.as_str().to_owned());
    let online = state.selected_host().is_some_and(|host| host.state.connected());
    let enabled = !state.manager.state().busy();
    div()
        .id("labels-host-filter")
        .debug_selector(|| "labels-host-filter".to_owned())
        .relative()
        .flex()
        .flex_row()
        .flex_none()
        .self_start()
        .items_center()
        .gap(px(S1_5))
        .py(px(S1_5))
        .px(px(S3))
        .rounded(px(MD))
        .bg(neutral.surface1)
        .border_1()
        .border_color(neutral.border)
        .child(anchor_probe(&state.host_anchor))
        .child(
            div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(DOT_SLOT))
                .child(dot(status(online, tokens)).size(px(DOT))),
        )
        .child(
            div()
                .min_w_0()
                .truncate()
                .type_(Type::Label)
                .text_color(neutral.foreground)
                .child(label),
        )
        .child(glyph(Glyph::ChevronDown, CARET, neutral.foreground_muted))
        .when(enabled, |pill| {
            pill.cursor_pointer()
                .hover(move |style| style.bg(neutral.surface2))
                .active(move |style| style.bg(neutral.surface3))
                .on_click(cx.listener(|shell, _event, window, cx| {
                    let state = &mut shell.areas.labels;
                    if state.manager.state().busy() {
                        return;
                    }
                    state.host_list.set_anchor(state.host_anchor.get());
                    state.host_list.open(None, window, cx);
                    cx.notify();
                }))
        })
}

/// The host list, over the dialog, while it is open.
pub(super) fn list(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.labels;
    if !state.host_list.is_open() {
        return None;
    }
    let mut rows = Vec::new();
    let mut routes = Vec::new();
    for host in &state.hosts {
        let mut row = ComboRow::new(host.route.as_str().to_owned());
        row.leading = Some(Leading::Dot(status(host.state.connected(), tokens)));
        row.checked = state.route.as_ref() == Some(&host.route);
        rows.push(row);
        routes.push(Some((host.route.clone(), host.connection.clone())));
    }
    for route in &state.unresolved {
        let mut row = ComboRow::new(route.as_str().to_owned());
        row.leading = Some(Leading::Dot(status(false, tokens)));
        row.disabled = true;
        rows.push(row);
        routes.push(None);
    }
    let header = ComboHeader {
        side: Side::Below,
        ..select::header()
    };
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.labels.host_list,
        chose: Rc::new(move |shell: &mut Shell, index, window, cx| {
            let state = &mut shell.areas.labels;
            let Some(Some((route, connection))) = routes.get(index) else {
                return;
            };
            // The row's host must still be the one the list was drawn for.
            if state.manager.state().busy()
                || !state
                    .hosts
                    .iter()
                    .any(|host| &host.route == route && &host.connection == connection)
            {
                return;
            }
            state.host_list.close_and_restore(window, cx);
            if let Err(failure) = state.select_route(route) {
                state.error = Some(failure);
            }
            cx.notify();
        }),
        back: None,
    };
    combobox(
        "labels-host-list",
        &state.host_list,
        &header,
        &rows,
        &handlers,
        select::WIDTH,
        tokens,
        window,
        cx,
    )
}

/// Paseo's host status dot: `statusSuccess` online, `statusDanger` otherwise.
fn status(online: bool, tokens: &Tokens) -> vessel_app_kit::ui::Hsla {
    if online { tokens.st_ready } else { tokens.danger }
}
