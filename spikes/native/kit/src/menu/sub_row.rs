//! A menu row that opens a page (TRACKER 791, [`crate::submenu`]), drawn
//! by the one menu renderer beside its ordinary rows.

use super::MenuState;
use super::draw::row_body;
use super::item::paint;
use crate::popup::Surfaces;
use crate::submenu::Submenus;
use crate::theme::spacing;
use gpui::prelude::*;
use gpui::{AnyElement, Context, MouseButton, div, px};

/// A row that opens a page, and what it is drawn from.
pub(super) struct SubRow<'a> {
    pub(super) index: usize,
    pub(super) item: &'a super::MenuItem,
    pub(super) page: &'a str,
    pub(super) submenus: &'a Submenus,
    pub(super) state: &'a MenuState,
    pub(super) surfaces: &'a Surfaces,
    pub(super) narrow: bool,
}

// Paseo's `MenuSubTrigger`: the decision's value and a chevron at the right;
// while its page is open the row keeps the fill (`active`) and no check.
// Pointer rest opens the page (hover intent), a press or Enter opens it at
// once; neither closes the menu nor reaches the caller's `chose`.
pub(super) fn sub_row<V: 'static>(
    row: &SubRow<'_>,
    get: fn(&mut V) -> &mut MenuState,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let SubRow {
        index,
        item,
        page,
        submenus,
        state,
        surfaces,
        narrow,
    } = *row;
    let open = submenus.is_open(page, 0);
    let highlighted = state.highlighted == Some(index);
    let paint = paint(item, highlighted || open, surfaces);
    let wash = surfaces.surface2;
    let trailing = div()
        .ml_auto()
        .flex()
        .items_center()
        .gap(px(spacing::S1_5))
        .flex_shrink(1.0)
        .min_w_0()
        .children(item.value.clone().map(|value| {
            div()
                .min_w_0()
                .truncate()
                .text_color(surfaces.foreground_muted)
                .child(value)
        }))
        .child(crate::picker::glyph(
            crate::assets::Glyph::ChevronRight,
            14.0,
            surfaces.foreground_muted,
        ));
    let mut element = row_body(index, item, paint, narrow)
        .relative()
        .when(open, move |element| element.bg(wash))
        .child(trailing)
        .child(submenus.trigger_probe(page));
    if item.selectable() {
        let page = crate::submenu::PageId::from(page.to_owned());
        let hover_lease = state.render_lease();
        let press_lease = hover_lease.clone();
        let click_lease = hover_lease.clone();
        let hovered = submenus.clone();
        let pressed = submenus.clone();
        let hovered_page = page.clone();
        element = element
            .cursor_pointer()
            .hover(move |style| style.bg(wash))
            .on_hover(cx.listener(move |view: &mut V, inside: &bool, window, cx| {
                if !get(view).accepts(&hover_lease) {
                    return;
                }
                if *inside {
                    hovered.hover_open(&hovered_page, 0, window, cx);
                } else {
                    hovered.hover_close(0, window, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view: &mut V, _event, _window, cx| {
                    if get(view).accepts(&press_lease) {
                        cx.stop_propagation();
                    }
                }),
            )
            .on_click(cx.listener(move |view: &mut V, _event, _window, cx| {
                if get(view).accepts(&click_lease) {
                    pressed.open(&page, 0);
                    cx.notify();
                }
            }));
    }
    element.into_any_element()
}
