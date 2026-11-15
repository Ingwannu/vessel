//! One shared renderer for point-opened and right-aligned action menus.

use super::{MenuHandlers, MenuItem, MenuState, Tone};
use std::rc::Rc;

use crate::fonts::Type;
use crate::popup::{self, Surfaces};
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;
use gpui::prelude::*;
use gpui::{
    Anchor, AnyElement, Context, ElementId, KeyDownEvent, MouseButton, Window, anchored, deferred,
    div, px,
};

/// The narrowest a menu is (Paseo's `minWidth`).
const MIN_WIDTH: f32 = 180.0;

/// The widest a menu is, before its words wrap.
const MAX_WIDTH: f32 = 320.0;

/// How far under its trigger a menu opens (Paseo's `offset`).
const OFFSET: f32 = 4.0;

/// A row's line height (Paseo's `MENU_ITEM_LINE_HEIGHT`).
const ROW_LINE: f32 = 18.0;

/// The menu, when open: drawn over everything at the point it was opened
/// from. `narrow`: a touch window (44-pixel rows).
pub fn menu<V: 'static>(
    id: impl Into<ElementId>,
    state: &MenuState,
    items: &[MenuItem],
    handlers: &MenuHandlers<V>,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, V>,
) -> Option<AnyElement> {
    draw(
        id.into(),
        (state, Anchor::TopLeft),
        items,
        handlers,
        tokens,
        narrow,
        cx,
    )
}

/// The same menu aligned to its trigger's right edge. Paseo's split
/// button opens from the caret half: aligning the menu's right edge to
/// that half keeps the actions beside their trigger as their words grow.
/// Existing point-opened menus keep their left-edge placement.
pub fn menu_right<V: 'static>(
    id: impl Into<ElementId>,
    state: &MenuState,
    items: &[MenuItem],
    handlers: &MenuHandlers<V>,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, V>,
) -> Option<AnyElement> {
    draw(
        id.into(),
        (state, Anchor::TopRight),
        items,
        handlers,
        tokens,
        narrow,
        cx,
    )
}

fn draw<V: 'static>(
    id: ElementId,
    placement: (&MenuState, Anchor),
    items: &[MenuItem],
    handlers: &MenuHandlers<V>,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, V>,
) -> Option<AnyElement> {
    let (state, corner) = placement;
    let at = state.at?;
    let get = handlers.state;
    let keys_items: Rc<[MenuItem]> = items.into();
    let chose = Rc::clone(&handlers.chose);
    let on_key = cx.listener(move |view: &mut V, event: &KeyDownEvent, window, cx| {
        let menu = get(view);
        match event.keystroke.key.as_str() {
            "escape" => menu.close(),
            "down" | "tab" => menu.step(&keys_items, true),
            "up" => menu.step(&keys_items, false),
            "home" => {
                menu.highlighted = None;
                menu.step(&keys_items, true);
            }
            "end" => {
                menu.highlighted = None;
                menu.step(&keys_items, false);
            }
            "enter" | "space" => {
                if let Some(index) = menu.highlighted {
                    menu.close();
                    chose(view, index, window, cx);
                }
            }
            _other => return,
        }
        cx.stop_propagation();
        cx.notify();
    });
    let on_outside = cx.listener(move |view: &mut V, _event, _window, cx| {
        get(view).close();
        cx.notify();
    });
    let surfaces = Surfaces::of(tokens);
    // Paseo's page: 4 above the first row and below the last, no gap
    // between rows, the rule between groups the full width.
    let mut panel = div()
        .id(id.clone())
        .track_focus(&state.focus)
        .on_key_down(on_key)
        .on_mouse_down_out(on_outside)
        // The popover takes the pointer and shows the arrow (`popup::layer`).
        .occlude()
        .cursor_default()
        .flex()
        .flex_col()
        .min_w(px(MIN_WIDTH))
        .max_w(px(MAX_WIDTH))
        .py(px(4.0))
        .bg(surfaces.surface1)
        .border_1()
        .border_color(surfaces.border_accent)
        .rounded(px(popup::MENU_RADIUS))
        .overflow_hidden()
        .shadow(popup::shadow_md(tokens));
    for (index, item) in items.iter().enumerate() {
        if item.starts_group && index > 0 {
            panel = panel.child(div().my(px(4.0)).h(px(1.0)).bg(surfaces.border_accent));
        }
        panel = panel.child(row(index, item, state, handlers, &surfaces, narrow, cx));
    }
    Some(
        deferred(
            anchored()
                .anchor(corner)
                .position(at)
                .offset(gpui::point(px(0.0), px(OFFSET)))
                .snap_to_window_with_margin(px(8.0))
                // Word 39: the menu arrives as Paseo's does (150 ms,
                // `menu-overlay.tsx`): the kit's quick length.
                .child(popup::appear(
                    ElementId::NamedChild(std::sync::Arc::new(id), "appear".into()),
                    panel,
                    crate::motion::QUICK,
                )),
        )
        .with_priority(1)
        .into_any_element(),
    )
}

fn row<V: 'static>(
    index: usize,
    item: &MenuItem,
    state: &MenuState,
    handlers: &MenuHandlers<V>,
    surfaces: &Surfaces,
    narrow: bool,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let highlighted = state.highlighted == Some(index);
    // Paseo's row: hovered, focused and open rows alike on `surface2`,
    // never a ring; the destructive row only in its words' colour.
    let ink = match item.tone {
        Tone::Plain => surfaces.foreground,
        Tone::Danger => surfaces.destructive,
    };
    let wash = surfaces.surface2;
    let get = handlers.state;
    let chose = Rc::clone(&handlers.chose);
    let mut element = div()
        .id(ElementId::NamedInteger(
            "menu-row".into(),
            u64::try_from(index).unwrap_or(u64::MAX),
        ))
        .flex()
        .items_center()
        .gap(px(8.0))
        .min_h(px(popup::row_height(narrow)))
        .mx(px(4.0))
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(popup::ROW_RADIUS))
        .type_(Type::Body)
        .line_height(px(crate::fonts::ui(ROW_LINE)))
        .text_color(ink)
        .when(item.disabled, |row| row.opacity(0.5))
        .child(item.label.clone());
    if !item.disabled {
        element = element
            .cursor_pointer()
            .hover(move |style| style.bg(wash))
            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                cx.stop_propagation();
            })
            .on_click(cx.listener(move |view: &mut V, _event, window, cx| {
                get(view).close();
                chose(view, index, window, cx);
                cx.notify();
            }));
    }
    if highlighted && !item.disabled {
        // The keyboard's row fills in as the highlight arrives (word 39).
        return popup::fill_in(
            ElementId::NamedInteger(
                "menu-row-arrive".into(),
                u64::try_from(index).unwrap_or(u64::MAX),
            ),
            element,
            wash,
        )
        .into_any_element();
    }
    element.into_any_element()
}
