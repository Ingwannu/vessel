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
//!   Touch height is 44 instead of current Paseo’s 28: ADR 0036 requires every
//!   touch target to be at least 44 points.
//! - Why: both halves stay one control, and the menu opens beside its
//!   trigger rather than at whichever point inside it was clicked.
//! - Corrections: typed label density never wraps inside the fixed height;
//!   absent primary actions use the reference's branch/caret-only shape.
//!   The loader reuses the shared ActivityIndicator, with16 ink in20 layout.
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
use crate::loading::{Points, Size, activity_indicator};
use crate::picker::{anchor_probe, glyph, one_line};
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

/// How much room the caller has beside the action's glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Density {
    /// Show the whole one-line label; the caller reserves room for it.
    Labelled,
    /// Paseo's `hideLabels`: retain the glyph and a caller-owned tooltip.
    IconOnly,
}

/// The primary half's label and glyph.
#[derive(Debug, Clone)]
pub struct SplitContent {
    /// Localized action label.
    pub label: SharedString,
    /// The action's vector glyph, including in the icon-only density.
    pub icon: Glyph,
    /// Explicit width decision, made from the panel's available room.
    pub density: Density,
    /// The primary action's availability.
    pub state: PrimaryState,
    /// Whether the caret keeps its open surface.
    pub menu: MenuPhase,
}

/// Distinct shapes: no redundant primary half when nothing is needed.
#[derive(Debug, Clone)]
pub enum Content {
    /// A primary action and its caret half.
    Split(SplitContent),
    /// A branch glyph and small caret opening the same menu.
    MenuOnly(MenuPhase),
}

/// Paseo's borderRadius.md; popup row geometry must not change this button.
const RADIUS: f32 = 6.0;
const GLYPH: f32 = 16.0;
const MENU_CARET: f32 = 12.0;
const BUSY_SLOT: f32 = 20.0;
const MENU_GAP: f32 = 4.0;
const POINTER_MENU_WIDTH: f32 = 42.0;
const TOUCH_MENU_WIDTH: f32 = 48.0;

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
    menu_width: f32,
}

impl Geometry {
    const fn of(touch: bool) -> Self {
        if touch {
            Self {
                height: crate::breakpoint::TOUCH_TARGET,
                inset: 12.0,
                gap: 8.0,
                menu_width: TOUCH_MENU_WIDTH,
            }
        } else {
            Self {
                height: 26.0,
                inset: 8.0,
                gap: 4.0,
                menu_width: POINTER_MENU_WIDTH,
            }
        }
    }
}

/// Draw both halves. The menu callback calls `MenuState::open` with its point;
/// the area draws `menu_right` in its overlay slot.
#[must_use]
pub fn split_button<V: 'static>(
    id: impl Into<ElementId>,
    content: Content,
    handlers: &SplitHandlers<V>,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, V>,
) -> Stateful<Div> {
    let geometry = Geometry::of(touch);
    let anchor = Rc::new(Cell::new(None));
    let frame = div()
        .id(id)
        .relative()
        .flex()
        .flex_none()
        .h(px(geometry.height))
        .border_1()
        .border_color(tokens.neutral.border_accent)
        .rounded(px(RADIUS))
        .overflow_hidden();
    match content {
        Content::Split(content) => {
            let caret = Caret {
                phase: content.menu,
                anchor: Rc::clone(&anchor),
            };
            frame
                .child(primary(
                    content,
                    Rc::clone(&handlers.primary),
                    geometry,
                    tokens,
                    cx,
                ))
                .child(caret_half(
                    Rc::clone(&handlers.menu),
                    caret,
                    geometry,
                    tokens,
                    cx,
                ))
                .child(anchor_probe(&anchor))
        }
        Content::MenuOnly(phase) => menu_only(
            frame,
            handlers,
            Caret { phase, anchor },
            geometry,
            tokens,
            cx,
        ),
    }
}

/// Both menu shapes use the same anchor and event boundary.
struct Caret {
    phase: MenuPhase,
    anchor: Rc<Cell<Option<Bounds<Pixels>>>>,
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
        .flex_none()
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
                .active(move |style| style.bg(hover))
                .on_click(cx.listener(move |owner, _event, window, cx| {
                    cx.stop_propagation();
                    action(owner, window, cx);
                }))
        })
        .when(content.state == PrimaryState::Busy, |half| {
            half.child(busy(neutral.foreground))
        })
        .when(content.state != PrimaryState::Busy, |half| {
            half.child(glyph(content.icon, GLYPH, neutral.foreground))
                .when(content.density == Density::Labelled, |half| {
                    half.child(label(content.label))
                })
        })
}

fn label(text: SharedString) -> Div {
    div().flex_none().whitespace_nowrap().child(one_line(&text))
}

fn busy(colour: gpui::Hsla) -> Div {
    // RN scales small20 by .8 without changing its layout box. Numeric16
    // centered in20 preserves that vector ink/stroke and the layout width.
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(BUSY_SLOT))
        .child(activity_indicator(
            "busy",
            Size::Numeric(Points::SIXTEEN),
            colour,
        ))
}

fn caret_half<V: 'static>(
    action: OpenMenu<V>,
    menu: Caret,
    geometry: Geometry,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> Stateful<Div> {
    let neutral = &tokens.neutral;
    let hover = neutral.surface2;
    let Caret { phase, anchor } = menu;
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
        .active(move |style| style.bg(hover))
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
            GLYPH,
            neutral.foreground_extra_muted,
        ))
}

fn menu_only<V: 'static>(
    frame: Stateful<Div>,
    handlers: &SplitHandlers<V>,
    menu: Caret,
    geometry: Geometry,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let hover = neutral.surface2;
    let Caret { phase, anchor } = menu;
    let action = Rc::clone(&handlers.menu);
    let probe = anchor_probe(&anchor);
    frame
        .items_center()
        .justify_center()
        .gap(px(MENU_GAP))
        .w(px(geometry.menu_width))
        .cursor_pointer()
        .when(phase == MenuPhase::Open, |frame| frame.bg(hover))
        .hover(move |style| style.bg(hover))
        .active(move |style| style.bg(hover))
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
        .child(glyph(Glyph::Branch, GLYPH, neutral.foreground_muted))
        .child(glyph(
            Glyph::ChevronDown,
            MENU_CARET,
            neutral.foreground_extra_muted,
        ))
        .child(probe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_never_wrap_even_for_long_korean_actions() {
        for text in [
            "Pull and push all changes",
            "모든 변경 사항을 가져오고 푸시하기",
        ] {
            let mut label = label(text.into());
            assert_eq!(
                label.style().text.white_space,
                Some(gpui::WhiteSpace::Nowrap)
            );
        }
        assert_eq!(RADIUS.to_bits(), 6.0_f32.to_bits());
        assert_eq!(MENU_CARET.to_bits(), 12.0_f32.to_bits());
        assert_eq!(BUSY_SLOT.to_bits(), 20.0_f32.to_bits());
    }

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
                gap: 4.0,
                menu_width: POINTER_MENU_WIDTH
            }
        );
        assert_eq!(
            Geometry::of(true),
            Geometry {
                height: crate::breakpoint::TOUCH_TARGET,
                inset: 12.0,
                gap: 8.0,
                menu_width: TOUCH_MENU_WIDTH
            }
        );
    }
}
