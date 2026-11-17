//! One shared renderer for point-opened and right-aligned action menus.

use super::input::{KeyMenu, keyboard};
use super::item::{Leading, paint};
use super::{MenuHandlers, MenuItem, MenuState, pointer_choice};
use std::rc::Rc;

use crate::fonts::Type;
use crate::popup::{self, Surfaces};
use super::sub_row::{SubRow, sub_row};
use crate::submenu::Submenus;
use crate::theme::{Tokens, spacing};
use crate::widgets::TypeExt as _;
use gpui::prelude::*;
use gpui::{
    Anchor, AnyElement, Context, ElementId, MouseButton, Pixels, Window, anchored, deferred, div,
    px,
};

/// The narrowest a menu is (Paseo's `minWidth`).
pub(super) const MIN_WIDTH: Pixels = px(180.0);

/// The widest a menu is, before its words wrap.
pub(super) const MAX_WIDTH: Pixels = px(320.0);

/// How far under its trigger a menu opens (Paseo's `offset`).
pub(super) const OFFSET: f32 = 4.0;

/// A row's line height (Paseo's `MENU_ITEM_LINE_HEIGHT`).
pub(super) const ROW_LINE: f32 = 18.0;

pub(super) const LEADING_SLOT: Pixels = px(16.0);
pub(super) const SUCCESS_GLYPH: Pixels = px(16.0);

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
/// Deferred ordering and outside-input boundary for the containing surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// Existing screen menus retain priority1 and outside-click behavior.
    Screen,
    /// A menu inside the existing priority2 dialog, above it at priority3.
    /// Its outside press dismisses only the menu, never the dialog underneath.
    AboveDialog,
}
impl Layer {
    pub(super) const fn priority(self) -> usize {
        match self {
            Self::Screen => 1,
            Self::AboveDialog => 3,
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
    /// Default screen ordering, or an explicit menu inside a dialog.
    pub layer: Layer,
    /// Its submenus, when a row opens a page ([`crate::submenu`]).
    pub submenus: Option<Submenus>,
}
impl<'a> Content<'a> {
    /// Legacy content width, with all labels and states retained.
    #[must_use]
    pub fn new(id: impl Into<ElementId>, items: &'a [MenuItem]) -> Self {
        Self {
            id: id.into(),
            items,
            width: Width::Content,
            layer: Layer::Screen,
            submenus: None,
        }
    }
    /// Rows that open pages open them through `submenus`; a press on an
    /// open page is not outside this menu.
    #[must_use]
    pub fn with_submenus(self, submenus: &Submenus) -> Self {
        Self {
            submenus: Some(submenus.clone()),
            ..self
        }
    }
    /// Choose the containing surface without another menu renderer.
    #[must_use]
    pub fn with_layer(self, layer: Layer) -> Self {
        Self { layer, ..self }
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
        Content::new(id, items),
        MenuPlacement {
            state,
            pointer_corner: Anchor::TopLeft,
        },
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
        Content::new(id, items),
        MenuPlacement {
            state,
            pointer_corner: Anchor::TopRight,
        },
        handlers,
        tokens,
        narrow,
        cx,
    )
}

/// Extended content and width through the same published renderer.
pub fn menu_with<V: 'static>(
    content: Content<'_>,
    state: &MenuState,
    handlers: &MenuHandlers<V>,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, V>,
) -> Option<AnyElement> {
    draw(
        content,
        MenuPlacement {
            state,
            pointer_corner: Anchor::TopLeft,
        },
        handlers,
        tokens,
        narrow,
        cx,
    )
}

// Placement is one named decision, not an argument-count tuple. The existing
// right-entry keeps its TopRight corner; measured end placement still comes
// from MenuState's authoritative inner trigger bounds.
#[derive(Clone, Copy)]
struct MenuPlacement<'a> {
    state: &'a MenuState,
    pointer_corner: Anchor,
}
fn draw<V: 'static>(
    content: Content<'_>,
    placement: MenuPlacement<'_>,
    handlers: &MenuHandlers<V>,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, V>,
) -> Option<AnyElement> {
    let MenuPlacement {
        state,
        pointer_corner,
    } = placement;
    let (at, corner) = state.at?.geometry(pointer_corner);
    let Content {
        id,
        items,
        width,
        layer,
        submenus,
    } = content;
    if let Some(submenus) = &submenus {
        // A new opening of this menu starts at its root page.
        submenus.follow(&Rc::downgrade(&state.cycle));
    }
    let get = handlers.state;
    let keys_items: Rc<[MenuItem]> = items.into();
    let key_menu = KeyMenu {
        handlers: MenuHandlers {
            state: get,
            chose: Rc::clone(&handlers.chose),
        },
        items: keys_items,
        lease: state.render_lease(),
        submenus: submenus.clone(),
    };
    let on_key = cx.listener(move |view, event, window, cx| {
        keyboard(view, event, window, cx, &key_menu);
    });
    let outside_lease = state.render_lease();
    let pages = submenus.clone();
    let on_outside = cx.listener(move |view: &mut V, _event, window, cx| {
        // A press on one of its open pages belongs to the menu.
        if pages
            .as_ref()
            .is_some_and(|pages| pages.contains(window.mouse_position()))
        {
            return;
        }
        if !get(view).accepts(&outside_lease) {
            if layer == Layer::AboveDialog {
                // A stale nested menu still contains this press; forwarding
                // along A's path could dismiss the dialog underneath B.
                cx.stop_propagation();
            }
            return;
        }
        get(view).close_and_restore(window, cx);
        if layer == Layer::AboveDialog {
            cx.stop_propagation();
        }
        cx.notify();
    });
    // track_focus installs GPUI's own automatic pointer-focus listener. Fence
    // its default behavior too, before an obsolete panel can focus old state.
    let focus_lease = state.render_lease();
    let surfaces = Surfaces::of(tokens);
    // Paseo's page: 4 above the first row and below the last, no gap
    // between rows, the rule between groups the full width.
    let mut panel = div()
        .id(id.clone())
        .track_focus(&state.focus)
        .key_context(popup::KEY_CONTEXT)
        .capture_any_mouse_down(cx.listener(move |view: &mut V, _event, window, _cx| {
            if !get(view).accepts(&focus_lease) {
                window.prevent_default();
            }
        }))
        .on_key_down(on_key)
        .on_mouse_down_out(on_outside)
        // The popover takes the pointer and shows the arrow (`popup::layer`).
        .occlude()
        .cursor_default()
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
            panel = panel.child(
                div()
                    .my(px(spacing::S1))
                    .h(px(1.0))
                    .bg(surfaces.border_accent),
            );
        }
        let drawn = match (&item.opens, &submenus) {
            (Some(page), Some(submenus)) => sub_row(
                &SubRow {
                    index,
                    item,
                    page,
                    submenus,
                    state,
                    surfaces: &surfaces,
                    narrow,
                },
                get,
                cx,
            ),
            _ => row(index, item, state, handlers, &surfaces, narrow, cx),
        };
        panel = panel.child(drawn);
    }
    let surface = anchored()
        .anchor(corner)
        .position(at)
        .offset(gpui::point(px(0.0), px(OFFSET)))
        .snap_to_window_with_margin(px(spacing::S2))
        // All entries retain the published QUICK arrival and one panel.
        .child(popup::appear(
            ElementId::NamedChild(std::sync::Arc::new(id), "appear".into()),
            panel,
            crate::motion::QUICK,
        ));
    let overlay = match layer {
        Layer::Screen => surface.into_any_element(),
        Layer::AboveDialog => ModalBoundary {
            surface: surface.into_any_element(),
        }
        .into_any_element(),
    };
    Some(
        deferred(overlay)
            .with_priority(layer.priority())
            .into_any_element(),
    )
}

// A nested deferred element still participates in its parent's layout. Read
// the actual Window here: size_full inside a dialog would cover only its body,
// leaving the scrim and sibling actions reachable by the same outside press.
#[derive(gpui::IntoElement)]
struct ModalBoundary {
    surface: AnyElement,
}
impl RenderOnce for ModalBoundary {
    fn render(self, window: &mut Window, _: &mut gpui::App) -> impl IntoElement {
        let size = window.viewport_size();
        anchored().position(gpui::point(px(0.0), px(0.0))).child(
            popup::layer(div())
                .w(size.width)
                .h(size.height)
                .child(self.surface),
        )
    }
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
    let paint = paint(item, highlighted, surfaces);
    // Paseo's row: hovered, focused and open rows alike on `surface2`,
    // never a ring; the destructive row only in its words' colour.
    let wash = surfaces.surface2;
    let get = handlers.state;
    let chose = Rc::clone(&handlers.chose);
    let mut element = row_body(index, item, paint, narrow);
    if item.selectable() {
        let choice = item.clone();
        let press_lease = state.render_lease();
        let choice_lease = press_lease.clone();
        element = element
            .cursor_pointer()
            .hover(move |style| style.bg(wash))
            .active(move |style| style.bg(wash))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view: &mut V, _event, _window, cx| {
                    if get(view).accepts(&press_lease) {
                        cx.stop_propagation();
                    }
                }),
            )
            .on_click(cx.listener(move |view: &mut V, _event, window, cx| {
                if !get(view).accepts(&choice_lease) {
                    return;
                }
                if let Some(index) = pointer_choice(get(view), &choice, index) {
                    if !get(view).same_cycle(&choice_lease) {
                        return;
                    }
                    get(view).restore_after_choice(window, cx);
                    if !get(view).same_cycle(&choice_lease) {
                        return;
                    }
                    chose(view, index, window, cx);
                    cx.notify();
                }
            }));
    }
    if highlighted && item.selectable() {
        // The keyboard's row fills in as the highlight arrives (word 39).
        return popup::fill_in(
            ElementId::NamedInteger(
                "menu-row-arrive".into(),
                u64::try_from(index).unwrap_or(u64::MAX),
            ),
            element,
            paint.fill,
        )
        .into_any_element();
    }
    element.into_any_element()
}

fn leading(index: usize, item: &MenuItem, ink: gpui::Hsla) -> Option<AnyElement> {
    let content = match item.leading() {
        Leading::None => return None,
        Leading::Glyph(glyph) => ordinary_glyph(item, glyph, ink).into_any_element(),
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

// A caller's ordinary glyph size must not scale feedback or its slot.
pub(super) fn ordinary_glyph(
    item: &MenuItem,
    glyph: crate::assets::Glyph,
    ink: gpui::Hsla,
) -> gpui::Svg {
    crate::picker::glyph(glyph, item.leading_size.points(), ink)
}

pub(super) fn row_body(
    index: usize,
    item: &MenuItem,
    paint: super::item::Paint,
    narrow: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
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
        .text_color(paint.ink)
        .opacity(paint.opacity)
        .border_1()
        .border_color(gpui::transparent_black())
        .children(leading(index, item, paint.glyph))
        .child(
            div()
                .flex_shrink(1.0)
                .min_w_0()
                .truncate()
                .child(crate::picker::one_line(item.words())),
        )
}
