//! The shared menu surface and row drawing, after Paseo's menu engine.
use super::item::{Leading, paint};
use super::{Direction, MenuHandlers, MenuItem, MenuState, keyboard_choice, pointer_choice};
use crate::{
    fonts::Type,
    popup::{self, Surfaces},
    theme::{Tokens, spacing},
    widgets::TypeExt as _,
};
use gpui::prelude::*;
use gpui::{
    AnyElement, Context, ElementId, KeyDownEvent, MouseButton, Pixels, anchored, deferred, div, px,
};
use std::rc::Rc;

pub(super) const MIN_WIDTH: Pixels = px(180.0);
pub(super) const MAX_WIDTH: Pixels = px(320.0);
pub(super) const LEADING_SLOT: Pixels = px(16.0);
pub(super) const LEADING_GLYPH: Pixels = px(14.0);
pub(super) const SUCCESS_GLYPH: Pixels = px(16.0);
pub(super) const OFFSET: f32 = 4.0;
pub(super) const ROW_LINE: f32 = 18.0;

/// Content-sized legacy menu, or a caller-specified reference width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Width {
    /// Preserve the legacy min180/max320 content size.
    Content,
    /// Explicit width in logical pixels (the schedule menu uses 220).
    Fixed(Pixels),
}
impl Width {
    fn fixed(self) -> Option<Pixels> {
        match self {
            Self::Content => None,
            Self::Fixed(width) => Some(width),
        }
    }
}
/// Menu data and width; grouped so the extended entry stays small.
#[derive(Debug)]
pub struct Content<'a> {
    /// Unique surface identity.
    pub id: ElementId,
    /// Caller-owned labels and states in display order.
    pub items: &'a [MenuItem],
    /// Content size or explicit reference width.
    pub width: Width,
}
impl<'a> Content<'a> {
    /// Legacy content width, with all labels and states retained.
    #[must_use]
    pub fn new(id: impl Into<ElementId>, items: &'a [MenuItem]) -> Self {
        Self {
            id: id.into(),
            items,
            width: Width::Content,
        }
    }
    /// Set the menu's reference width, without affecting the legacy entry.
    #[must_use]
    pub fn with_width(self, width: Pixels) -> Self {
        Self {
            width: Width::Fixed(width),
            ..self
        }
    }
}
/// Compatibility entry: existing callers and content sizing are unchanged.
pub fn menu<V: 'static>(
    id: impl Into<ElementId>,
    state: &MenuState,
    items: &[MenuItem],
    handlers: &MenuHandlers<V>,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, V>,
) -> Option<AnyElement> {
    menu_with(Content::new(id, items), state, handlers, tokens, narrow, cx)
}

/// The menu, when open: drawn over everything at the point it was opened
/// from. `narrow`: a touch window (44-pixel rows).
pub fn menu_with<V: 'static>(
    content: Content<'_>,
    state: &MenuState,
    handlers: &MenuHandlers<V>,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, V>,
) -> Option<AnyElement> {
    let (at, corner) = state.at?.geometry();
    let Content { id, items, width } = content;
    let get = handlers.state;
    let keys_items: Rc<[MenuItem]> = items.into();
    let chose = Rc::clone(&handlers.chose);
    let on_key = cx.listener(move |view: &mut V, event: &KeyDownEvent, window, cx| {
        let menu = get(view);
        match event.keystroke.key.as_str() {
            "escape" => menu.close(),
            "down" | "tab" => menu.step(&keys_items, Direction::Next),
            "up" => menu.step(&keys_items, Direction::Previous),
            "home" => {
                menu.highlighted = None;
                menu.step(&keys_items, Direction::Next);
            }
            "end" => {
                menu.highlighted = None;
                menu.step(&keys_items, Direction::Previous);
            }
            "enter" | "space" => {
                if let Some(index) = keyboard_choice(menu, &keys_items) {
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
        .id(id)
        .track_focus(&state.focus)
        .on_key_down(on_key)
        .on_mouse_down_out(on_outside)
        .occlude()
        .flex()
        .flex_col()
        .min_w(MIN_WIDTH)
        .when(width == Width::Content, |panel| panel.max_w(MAX_WIDTH))
        .when_some(width.fixed(), Styled::w)
        .py(px(spacing::S1))
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
                .child(panel),
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
    let paint = paint(item, highlighted, surfaces);
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
        .gap(px(spacing::S2))
        .min_h(px(popup::row_height(narrow)))
        .mx(px(spacing::S1))
        .px(px(spacing::S2))
        .py(px(spacing::S1))
        .rounded(px(popup::ROW_RADIUS))
        .type_(Type::Body)
        .line_height(px(crate::fonts::ui(ROW_LINE)))
        .border_1()
        .border_color(gpui::transparent_black())
        .text_color(paint.ink)
        .opacity(paint.opacity)
        .bg(paint.fill)
        .children(leading(index, item, paint.glyph))
        .child(
            div()
                .flex_shrink(1.0)
                .min_w_0()
                // Paseo's numberOfLines=1: keep every menu row at one pitch,
                // including a pending label received after it was opened.
                .truncate()
                .child(crate::picker::one_line(item.words())),
        );
    if item.selectable() {
        let choice = item.clone();
        element = element
            .cursor_pointer()
            .hover(move |style| style.bg(wash))
            .active(move |style| style.bg(wash))
            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                cx.stop_propagation();
            })
            .on_click(cx.listener(move |view: &mut V, _event, window, cx| {
                if let Some(index) = pointer_choice(get(view), &choice, index) {
                    chose(view, index, window, cx);
                    cx.notify();
                }
            }));
    }
    element.into_any_element()
}

fn leading(index: usize, item: &MenuItem, ink: gpui::Hsla) -> Option<AnyElement> {
    let content = match item.leading() {
        Leading::None => return None,
        Leading::Glyph(glyph) => {
            crate::picker::glyph(glyph, f32::from(LEADING_GLYPH), ink).into_any_element()
        }
        Leading::SuccessCheck => crate::picker::glyph(
            crate::assets::Glyph::CircleCheck,
            f32::from(SUCCESS_GLYPH),
            ink,
        )
        .into_any_element(),
        Leading::Spinner => crate::loading::activity_indicator(
            ElementId::NamedInteger(
                "menu-pending".into(),
                u64::try_from(index).unwrap_or(u64::MAX),
            ),
            crate::loading::Size::Numeric(crate::loading::Points::SIXTEEN),
            ink,
        ),
    };
    Some(
        div()
            .w(LEADING_SLOT)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(content)
            .into_any_element(),
    )
}
