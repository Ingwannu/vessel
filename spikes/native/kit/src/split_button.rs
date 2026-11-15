//! Paseo's git action split button: a primary half, a divider and a
//! caret half opening the kit's menu below its trigger, aligned right.
//!
//! [Decision Log]
//! - Purpose: one split action for git and every later area that needs it.
//! - Constraints: screens use the kit's menu, glyphs, spinner and tokens;
//!   no component-layer theme or container focus ring may appear.
//! - Alternatives: two unrelated outline buttons, or a second menu.
//! - Choice: Paseo's `git/actions-split-button.tsx` values: 26 tall with
//!   a pointer, 44 on touch; a 6-point radius and borderAccent edge; the
//!   primary half inset 8/12, a square caret half with a hairline divider;
//!   surface2 on hover, a disabled primary at 60%, spinner while busy.
//!   Touch height is 44 instead of Paseo’s 32: ADR 0036 requires every
//!   touch target to be at least 44 points.
//! - Why: both halves stay one control, and the menu opens beside its
//!   trigger rather than at whichever point inside it was clicked.
//! - Cost: the caller owns menu state and actions. The callbacks are an
//!   open UI extension point: every area can wire its own owner type.

use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    Bounds, Context, Div, ElementId, MouseButton, MouseDownEvent, Pixels, Point, SharedString,
    Stateful, Window, div, px,
};

use crate::assets::Glyph;
use crate::fonts::Type;
use crate::picker::{anchor_probe, glyph};
use crate::popup;
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

/// The primary half's state; busy and disabled cannot accept a click.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryState {
    /// Ready to run its action.
    Ready,
    /// An operation is running; draw the kit's spinner.
    Busy,
    /// Cannot run; its menu supplies the explanation.
    Disabled,
}

impl PrimaryState {
    /// Whether a primary handler may be attached.
    #[must_use]
    pub const fn active(self) -> bool {
        match self {
            Self::Ready => true,
            Self::Busy | Self::Disabled => false,
        }
    }
}

/// The caret keeps the hover surface while its menu is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuPhase {
    /// The caller's menu is closed.
    Closed,
    /// The caller's menu is open.
    Open,
}

/// The primary half's label and optional glyph.
#[derive(Debug, Clone)]
pub struct SplitContent {
    /// Localized action label.
    pub label: SharedString,
    /// The action's vector glyph, when it has one.
    pub icon: Option<Glyph>,
    /// The primary action's availability.
    pub state: PrimaryState,
    /// Whether the caret keeps its open surface.
    pub menu: MenuPhase,
}

/// A split action's primary callback.
pub type PrimaryAction<V> = Rc<dyn Fn(&mut V, &mut Window, &mut Context<'_, V>)>;

/// A menu callback; its point is the trigger's lower-right corner.
pub type OpenMenu<V> = Rc<dyn Fn(&mut V, Point<Pixels>, &mut Window, &mut Context<'_, V>)>;

/// The two distinct actions of the control.
pub struct SplitHandlers<V> {
    /// Run the primary action; never called while busy or disabled.
    pub primary: PrimaryAction<V>,
    /// Open the caller's kit menu, including any disabled explanations.
    pub menu: OpenMenu<V>,
}

impl<V> std::fmt::Debug for SplitHandlers<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SplitHandlers").finish_non_exhaustive()
    }
}

/// Source geometry, shared by drawing and the pointer/touch checks.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Geometry {
    height: f32,
    inset: f32,
    gap: f32,
}

impl Geometry {
    const fn of(touch: bool) -> Self {
        if touch {
            Self {
                height: 44.0,
                inset: 12.0,
                gap: 8.0,
            }
        } else {
            Self {
                height: 26.0,
                inset: 8.0,
                gap: 4.0,
            }
        }
    }
}

/// Draw both halves. The menu callback calls `MenuState::open` with its point;
/// the area draws `menu_right` in its overlay slot.
pub fn split_button<V: 'static>(
    id: impl Into<ElementId>,
    content: SplitContent,
    handlers: &SplitHandlers<V>,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, V>,
) -> Stateful<Div> {
    let geometry = Geometry::of(touch);
    let menu = content.menu;
    let anchor: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
    div()
        .id(id)
        .relative()
        .flex()
        .flex_none()
        .max_w_full()
        .h(px(geometry.height))
        .border_1()
        .border_color(tokens.neutral.border_accent)
        .rounded(px(popup::ROW_RADIUS))
        .overflow_hidden()
        .child(primary(
            content,
            Rc::clone(&handlers.primary),
            geometry,
            tokens,
            cx,
        ))
        .child(caret(
            Rc::clone(&handlers.menu),
            (menu, Rc::clone(&anchor)),
            geometry,
            tokens,
            cx,
        ))
        .child(anchor_probe(&anchor))
}

fn primary<V: 'static>(
    content: SplitContent,
    action: PrimaryAction<V>,
    geometry: Geometry,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> Stateful<Div> {
    let neutral = &tokens.neutral;
    let hover = neutral.surface2;
    div()
        .id("primary")
        .flex()
        .items_center()
        .justify_center()
        .gap(px(geometry.gap))
        .min_w_0()
        .px(px(geometry.inset))
        .type_(Type::Body)
        .line_height(px(crate::fonts::ui(21.0)))
        .text_color(neutral.foreground)
        .when(content.state == PrimaryState::Disabled, |half| {
            half.opacity(0.6)
        })
        .when(content.state.active(), |half| {
            half.cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_click(cx.listener(move |owner, _event, window, cx| {
                    action(owner, window, cx);
                }))
        })
        .when(content.state == PrimaryState::Busy, |half| {
            half.child(crate::slider::spinner("busy", 16.0, neutral.foreground))
        })
        .when(content.state != PrimaryState::Busy, |half| {
            half.children(
                content
                    .icon
                    .map(|icon| glyph(icon, 16.0, neutral.foreground)),
            )
            .child(div().min_w_0().whitespace_normal().child(content.label))
        })
}

fn caret<V: 'static>(
    action: OpenMenu<V>,
    menu: (MenuPhase, Rc<Cell<Option<Bounds<Pixels>>>>),
    geometry: Geometry,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> Stateful<Div> {
    let neutral = &tokens.neutral;
    let hover = neutral.surface2;
    let (phase, anchor) = menu;
    div()
        .id("menu")
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .w(px(geometry.height))
        .border_l_1()
        .border_color(neutral.border_accent)
        .cursor_pointer()
        .when(phase == MenuPhase::Open, |half| half.bg(hover))
        .hover(move |style| style.bg(hover))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |owner, event: &MouseDownEvent, window, cx| {
                let at = anchor
                    .get()
                    .map_or(event.position, |bounds| bounds.bottom_right());
                action(owner, at, window, cx);
                cx.stop_propagation();
            }),
        )
        .child(glyph(
            Glyph::ChevronDown,
            16.0,
            neutral.foreground_extra_muted,
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_ready_primary_actions_can_run() {
        assert!(PrimaryState::Ready.active());
        assert!(!PrimaryState::Busy.active());
        assert!(!PrimaryState::Disabled.active());
    }

    #[test]
    fn source_geometry_keeps_the_pointer_and_touch_control_heights() {
        assert_eq!(
            Geometry::of(false),
            Geometry {
                height: 26.0,
                inset: 8.0,
                gap: 4.0
            }
        );
        assert_eq!(
            Geometry::of(true),
            Geometry {
                height: 44.0,
                inset: 12.0,
                gap: 8.0
            }
        );
    }
}
