//! The list's panel and where it is drawn (moved out of `picker.rs` with
//! TRACKER 791): at its trigger, as a phone's sheet from the bottom, or
//! inside a flyout, a page beside its menu ([`crate::submenu`]), which
//! places it and to which a press outside it belongs.

use gpui::prelude::*;
use gpui::{
    Anchor, AnyElement, Bounds, Context, Div, ElementId, MouseButton, Pixels, Stateful, Window,
    anchored, div, point, px,
};

use super::keys::key_handler;
use super::lifecycle::RenderLease;
use super::rows::{note_row, rows_list, title_row};
use super::{
    ComboHandlers, ComboHeader, ComboRow, ComboboxState, EDGE, LIFT, OFFSET, Side, room, search_row,
};
use crate::dialog::sheet_handle;
use crate::fonts::Type;
use crate::popup::{self, Surfaces};
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

/// How a list's panel is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Surface {
    /// At its trigger, closing on a press outside it.
    Popover,
    /// A phone's sheet from the bottom.
    Sheet,
    /// Inside a flyout: placed by it, outside presses its menu's.
    Flyout,
}

/// A list and what it is drawn from.
pub(super) struct List<'a, V> {
    pub(super) id: ElementId,
    pub(super) state: &'a ComboboxState,
    pub(super) header: &'a ComboHeader,
    pub(super) rows: &'a [ComboRow],
    pub(super) handlers: &'a ComboHandlers<V>,
    pub(super) width: f32,
}

/// The list's panel, arriving as Paseo's popover or sheet does.
pub(super) fn panel<V: 'static>(
    list: &List<'_, V>,
    surface: Surface,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let (state, header, rows, handlers, width) = (
        list.state,
        list.header,
        list.rows,
        list.handlers,
        list.width,
    );
    let list_id = list.id.clone();
    if let Some(pages) = &state.submenus {
        // A new opening of the list starts at its root page.
        pages.follow(&std::rc::Rc::downgrade(&state.cycle));
    }
    let compact = surface == Surface::Sheet;
    let surfaces = Surfaces::of(tokens);
    let neutral = tokens.neutral;
    let get = handlers.state;
    let on_key = key_handler(rows, state, handlers, cx);
    let lease = state.render_lease();
    let outside_lease = lease.clone();
    let on_outside = cx.listener(move |view: &mut V, _event, window, cx| {
        // A press on one of its open pages belongs to the list.
        if get(view)
            .submenus
            .as_ref()
            .is_some_and(|pages| pages.contains(window.mouse_position()))
        {
            return;
        }
        if get(view).accepts(&outside_lease) {
            get(view).close_and_restore(window, cx);
            cx.notify();
        } else {
            window.prevent_default();
        }
        cx.stop_propagation();
    });

    let mut panel = div()
        .id(list_id.clone())
        .track_focus(&state.focus)
        .capture_any_mouse_down(cx.listener(move |view: &mut V, _event, window, cx| {
            if !get(view).accepts(&lease) {
                window.prevent_default();
                cx.stop_propagation();
            }
        }))
        .capture_key_down(on_key)
        // The popover takes the pointer and shows the arrow (`popup::layer`).
        .map(popup::layer)
        .flex()
        .flex_col()
        .bg(surfaces.surface0)
        .border_1()
        .border_color(neutral.border)
        .overflow_hidden()
        .type_(Type::Body)
        .text_color(surfaces.foreground);
    let anchor = (surface == Surface::Popover)
        .then(|| state.anchor.get())
        .flatten();
    let room = room(anchor, header.side, window);
    panel = match surface {
        // Gorhom's handle on Paseo's combobox sheet, the kit's sheets' one.
        Surface::Sheet => panel
            .w_full()
            .max_h(gpui::relative(0.9))
            .rounded_t(px(popup::SHEET_RADIUS))
            .pb(px(16.0))
            .child(sheet_handle(tokens.neutral.foreground_extra_muted)),
        Surface::Popover | Surface::Flyout => {
            let popover = panel
                .w(px(width))
                .max_h(px(room))
                .rounded(px(popup::MENU_RADIUS))
                .shadow(popup::shadow_md(tokens));
            // In a flyout a press outside belongs to its menu.
            if surface == Surface::Popover {
                popover.on_mouse_down_out(on_outside)
            } else {
                popover
            }
        }
    };
    panel = heading(panel, list, tokens, cx);
    panel = panel.child(rows_list(
        header, rows, state, handlers, compact, tokens, cx,
    ));

    // Word 39: the list arrives as Paseo's popover does (opacity and a
    // small rise, `popup::appear`); a phone's sheet over the surface
    // length. It leaves at once, as Paseo's desktop popovers do.
    popup::appear(
        ElementId::NamedChild(std::sync::Arc::new(list_id), "appear".into()),
        panel,
        if compact {
            crate::motion::STANDARD
        } else {
            crate::motion::QUICK
        },
    )
    .into_any_element()
}

/// Where the list is drawn: below the compact width a sheet from the
/// bottom over the dimmed page (a press on the dim closes it); wider, at
/// its trigger on the caller's side.
pub(super) struct Placement {
    pub(super) compact: bool,
    pub(super) side: Side,
    pub(super) anchor: Option<Bounds<Pixels>>,
    pub(super) lease: RenderLease,
}
pub(super) fn placed<V: 'static>(
    panel: impl IntoElement,
    placement: Placement,
    get: fn(&mut V) -> &mut ComboboxState,
    window: &Window,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let Placement {
        compact,
        side,
        anchor,
        lease,
    } = placement;
    if compact {
        // A sheet: the page dimmed behind, the list from the bottom.
        let viewport = window.viewport_size();
        let on_scrim = cx.listener(move |view: &mut V, _event, window, cx| {
            if get(view).accepts(&lease) {
                get(view).close_and_restore(window, cx);
                cx.notify();
            } else {
                window.prevent_default();
            }
            cx.stop_propagation();
        });
        anchored()
            .position(point(px(0.0), px(0.0)))
            .child(
                div()
                    .w(viewport.width)
                    .h(viewport.height)
                    .flex()
                    .flex_col()
                    .justify_end()
                    .bg(popup::sheet_scrim())
                    .on_mouse_down(MouseButton::Left, on_scrim)
                    .child(panel),
            )
            .into_any_element()
    } else {
        // Paseo's `offset(5)` from the trigger, on the caller's side.
        let (at, corner, offset) = match (side, anchor) {
            // Not drawn yet (the first frame): near the window's middle.
            (_, None) => (point(px(8.0), px(240.0)), Anchor::BottomLeft, 0.0),
            (Side::Above, Some(bounds)) => (bounds.origin, Anchor::BottomLeft, LIFT),
            (Side::Below, Some(bounds)) => (bounds.bottom_left(), Anchor::TopLeft, OFFSET),
        };
        anchored()
            .position(at)
            .offset(point(px(0.0), px(offset)))
            .anchor(corner)
            .snap_to_window_with_margin(px(EDGE))
            .child(panel)
            .into_any_element()
    }
}

/// The rows above the list's rows: its title (with the way back), its
/// search, its note.
fn heading<V: 'static>(
    mut panel: Stateful<Div>,
    list: &List<'_, V>,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> Stateful<Div> {
    let (state, header) = (list.state, list.header);
    if let Some(title) = &header.title {
        panel = panel.child(title_row(
            title.clone(),
            header.back,
            state,
            list.handlers,
            tokens,
            cx,
        ));
    }
    if header.search {
        panel = panel.child(search_row(state, tokens, cx));
    }
    if let Some(note) = &header.note {
        panel = panel.child(note_row(note.clone(), tokens));
    }
    panel
}
