//! A list row that opens a page beside the list (TRACKER 791,
//! [`crate::submenu`]): Paseo's `MenuSubTrigger` in the Display menu, the
//! decision's value and a chevron at its right (the row's own `trailing`
//! and `opens`), the fill while its page is open.

use super::ComboboxState;
use crate::submenu::Submenus;
use gpui::prelude::*;
use gpui::{Context, Div, Hsla, MouseButton, Stateful};

/// Bind `element`, the row for `page`: the pointer at rest opens the page,
/// leaving closes it after the grace, a press opens it at once; the screen's
/// `chose` is not called.
pub(super) fn page_row<V: 'static>(
    element: Stateful<Div>,
    page: &str,
    pages: &Submenus,
    state: &ComboboxState,
    get: fn(&mut V) -> &mut ComboboxState,
    wash: Hsla,
    cx: &mut Context<'_, V>,
) -> Stateful<Div> {
    let open = pages.is_open(page, 0);
    let page = crate::submenu::PageId::from(page.to_owned());
    let hover_lease = state.render_lease();
    let click_lease = hover_lease.clone();
    let hovered = pages.clone();
    let pressed = pages.clone();
    let hovered_page = page.clone();
    element
        .relative()
        .when(open, move |element| element.bg(wash))
        .child(pages.trigger_probe(&page))
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
        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .on_click(cx.listener(move |view: &mut V, _event, _window, cx| {
            if get(view).accepts(&click_lease) {
                pressed.open(&page, 0);
                cx.notify();
            }
        }))
}

#[cfg(test)]
mod tests;
