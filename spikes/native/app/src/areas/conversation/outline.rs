//! The chat outline: Paseo's rail (`agent-stream/chat-outline/rail.web.tsx`,
//! `model.ts`, `hover-intent.ts`): one pill per prompt in a column at the
//! conversation's left, the prompt whose turn is at the view's top lit,
//! the pills near the pointer magnified as a dock's, a card with the
//! prompt beside the one pointed at, a click going to that prompt. The
//! prompts are the view's (`view::timeline::outline`).
//!
//! Paseo's values: the rail 36 wide, 8 in from the left, from 10% to 90%
//! of the height; slots 8 high that shrink together on a long
//! conversation; a pill 10 by 2 at rest (`borderAccent`), 18 wide for the
//! lit prompt (`foregroundExtraMuted`), 26 by 4 under the pointer
//! (`foreground`), a raised cosine over 3 slots between; the card 260 by
//! 48, 4 from the rail, on `surface2` with a hairline and the medium
//! shadow, 12-point text on two lines; 150 ms before the first pill
//! answers the pointer, none after; shown from two prompts, and only in
//! a panel at least 918 wide.
//!
//! Differences, kept: the pills change size at once (Paseo eases over 140
//! ms); the pointer's sideways sweep that also starts the intent is not
//! followed, only its entering a slot.

use std::time::Duration;

use vessel_app_kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px, relative,
};
use vessel_app_kit::{Reveal, Tokens, Type, TypeExt as _, popup};
use vessel_app_view::timeline::OutlineItem;

use crate::shell::Shell;

/// The rail's width and its distance from the conversation's left.
const RAIL: f32 = 36.0;
const RAIL_LEFT: f32 = 8.0;

/// A slot's height at rest.
const SLOT: f32 = 8.0;

/// The narrowest conversation that shows the rail.
const MIN_PANEL: f32 = 918.0;

/// A pill's size by its distance from the pointer, 0 to 2 slots (the
/// raised cosine `(1 + cos(πd/3)) / 2` = 1, 0.75, 0.25 between the size at
/// rest and 26 by 4); at 3 slots and beyond, at rest.
const WIDE_FROM_REST: [f32; 3] = [26.0, 22.0, 14.0];
const WIDE_FROM_LIT: [f32; 3] = [26.0, 24.0, 20.0];
const HIGH: [f32; 3] = [4.0, 3.5, 2.5];
const REST_WIDE: f32 = 10.0;
const LIT_WIDE: f32 = 18.0;
const REST_HIGH: f32 = 2.0;

/// The card beside the pill pointed at.
const CARD_WIDTH: f32 = 260.0;
const CARD_HEIGHT: f32 = 48.0;
/// Its left (the rail's width and a gap of 4) and its top (half its
/// height above the slot's middle).
const CARD_LEFT: f32 = 40.0;
const CARD_TOP: f32 = -24.0;

/// How long the pointer rests on the rail before the pills answer it.
const INTENT: Duration = Duration::from_millis(150);

/// Where the pointer is on the rail.
#[derive(Debug, Default)]
pub(crate) struct Pointer {
    /// The slot under the pointer, answered or not.
    candidate: Option<usize>,
    /// The slot the pills answer: magnified around, its card shown.
    attention: Option<usize>,
    /// Whether the pills answer at once (the first wait is over).
    active: bool,
    /// Bumped whenever the wait is cancelled or started again.
    wait: u64,
}

impl Pointer {
    /// Both a pending intent and a visible hover protect this resource.
    #[cfg(test)]
    pub(super) fn navigation_held(&self) -> bool {
        self.candidate.is_some() || self.attention.is_some()
    }

    /// Same cancellation used by pointer leave and exact-resource retirement.
    pub(super) fn leave(&mut self) -> bool {
        let was_visible = self.attention.is_some();
        *self = Self {
            wait: self.wait.wrapping_add(1),
            ..Self::default()
        };
        was_visible
    }
}

impl Shell {
    /// The pointer came onto slot `index`.
    fn outline_point(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let pointer = &mut self.areas.conversation.outline_pointer;
        pointer.candidate = Some(index);
        if pointer.active {
            pointer.attention = Some(index);
            cx.notify();
            return;
        }
        pointer.wait = pointer.wait.wrapping_add(1);
        let wait = pointer.wait;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(INTENT).await;
            let _gone = this.update(cx, |shell, cx| {
                let pointer = &mut shell.areas.conversation.outline_pointer;
                if pointer.wait == wait && pointer.candidate.is_some() {
                    pointer.active = true;
                    pointer.attention = pointer.candidate;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// The pointer left the rail.
    fn outline_left(&mut self, cx: &mut Context<'_, Self>) {
        let pointer = &mut self.areas.conversation.outline_pointer;
        let was = pointer.leave();
        if was {
            cx.notify();
        }
    }

    /// Go to prompt `item` of the outline.
    fn outline_jump(&mut self, item: usize, cx: &mut Context<'_, Self>) {
        let area = &self.areas.conversation;
        let Some(entry) = area.outline.get(item).map(|item| item.entry) else {
            return;
        };
        if let Some(row) = area.rows.row_index(entry, area.folds.as_ref(), &area.logs) {
            area.list.reveal(area.at(row), Reveal::Top);
        }
        cx.notify();
    }
}

/// The prompt whose turn is at the view's top: the last at or before the
/// entry the view starts in.
fn lit(outline: &[OutlineItem], top_entry: usize) -> Option<usize> {
    outline
        .partition_point(|item| item.entry <= top_entry)
        .checked_sub(1)
}

/// Whether the rail fits the conversation's width this frame, measured
/// where the list is drawn.
///
/// [Decision Log]
/// - Purpose: [`rail`] decides from the list's width at its last layout,
///   and the timeline is a cached region: after the window narrowed past
///   918 the region was not drawn again, so a rail Paseo would hide stayed
///   in the gutter (the coordinator's 0.0.0.0.19 smoke at 800x600).
/// - Choice: a canvas over the list reads the list's width once this
///   frame laid it out (the same width [`rail`] reads) and, only when it
///   falls on the other side of 918 from the decision the rail was drawn
///   with, marks the timeline region dirty: one more frame on the
///   crossing, nothing on an ordinary frame.
/// - Constraint: GPUI drops a view's notification made while the window
///   draws (`WindowInvalidator::invalidate_view` leaves the window clean
///   outside `DrawPhase::None`, and `Window::draw` then clears the dirty
///   views), so the probe defers it to the effects that run after the
///   frame. The 0.0.0.0.20 probe notified from paint directly and was
///   lost: a resize from 1280 to 800 kept the rail, and a session opened
///   at 1280 never showed it (the first layout's width is zero).
/// - Alternatives: drawing the region on every resize (loses the cache),
///   or the window's width less the panels (repeats the frame's layout);
///   the canvas's own box would be a second measure that could fall on
///   the other side of 918 from the list's and notify every frame.
pub(super) fn width_probe(shell: &Shell, cx: &Context<'_, Shell>) -> AnyElement {
    let list = shell.areas.conversation.list.clone();
    let drawn = list.width() >= px(MIN_PANEL);
    let target = shell
        .areas
        .regions
        .timeline
        .as_ref()
        .map_or_else(|| cx.entity_id(), gpui::Entity::entity_id);
    gpui::canvas(
        |_, _, _| (),
        move |_bounds, (), _window, cx| {
            if (list.width() >= px(MIN_PANEL)) != drawn {
                cx.defer(move |cx| cx.notify(target));
            }
        },
    )
    .absolute()
    .inset_0()
    .into_any_element()
}

/// The rail, when the conversation has two prompts and room for it.
pub(super) fn rail(
    shell: &Shell,
    tokens: &Tokens,
    _window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.conversation;
    if area.outline.len() < 2 || area.list.width() < px(MIN_PANEL) {
        return None;
    }
    let (top_row, _) = area.list.top();
    // A row of the fork's origin lights nothing (the rail is the fork's).
    let lit = top_row
        .checked_sub(area.head())
        .and_then(|own| area.rows.entry(own))
        .and_then(|top| lit(&area.outline, top));
    let attention = area.outline_pointer.attention;
    let neutral = tokens.neutral;
    let slots = area.outline.iter().enumerate().map(|(index, item)| {
        let near = attention.map(|at| at.abs_diff(index));
        let is_lit = lit == Some(index);
        let (widths, rest) = if is_lit {
            (&WIDE_FROM_LIT, LIT_WIDE)
        } else {
            (&WIDE_FROM_REST, REST_WIDE)
        };
        let (wide, high) = near
            .and_then(|near| Some((*widths.get(near)?, *HIGH.get(near)?)))
            .unwrap_or((rest, REST_HIGH));
        let color = if attention == Some(index) {
            neutral.foreground
        } else if is_lit {
            neutral.foreground_extra_muted
        } else {
            neutral.border_accent
        };
        let card = (attention == Some(index)).then(|| preview(item.prompt.clone(), tokens));
        div()
            .id(("outline-slot", index))
            .relative()
            .w(px(RAIL))
            .flex_basis(px(SLOT))
            .flex_shrink(1.0)
            .min_h(px(0.0))
            .flex()
            .items_center()
            .pl(px(4.0))
            .cursor_pointer()
            .on_hover(cx.listener(move |shell, hovered: &bool, _window, cx| {
                if *hovered {
                    shell.outline_point(index, cx);
                }
            }))
            .on_click(cx.listener(move |shell, _event, _window, cx| {
                shell.outline_jump(index, cx);
            }))
            .child(div().w(px(wide)).h(px(high)).rounded_full().bg(color))
            .children(card)
    });
    Some(
        div()
            .id("outline-rail")
            .debug_selector(|| "outline-rail".to_owned())
            .absolute()
            .left(px(RAIL_LEFT))
            .top(relative(0.1))
            .bottom(relative(0.1))
            .w(px(RAIL))
            .flex()
            .flex_col()
            .justify_center()
            .on_hover(cx.listener(|shell, hovered: &bool, _window, cx| {
                if !*hovered {
                    shell.outline_left(cx);
                }
            }))
            .children(slots)
            .into_any_element(),
    )
}

/// The prompt under the pointer, in a card at the rail's right: two
/// lines at most.
fn preview(prompt: String, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    div()
        .absolute()
        .left(px(CARD_LEFT))
        .top(relative(0.5))
        .mt(px(CARD_TOP))
        .w(px(CARD_WIDTH))
        .h(px(CARD_HEIGHT))
        .flex()
        .items_center()
        .px(px(12.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(neutral.border)
        .bg(neutral.surface2)
        .shadow(popup::shadow_md(tokens))
        .face(Type::Body)
        .text_size(px(12.0))
        .line_height(px(16.0))
        .text_color(neutral.foreground)
        .child(div().line_clamp(2).text_ellipsis().child(prompt))
}

#[cfg(test)]
mod resize_tests;

#[cfg(test)]
mod tests {
    use vessel_app_view::timeline::OutlineItem;

    use super::lit;

    fn item(entry: usize) -> OutlineItem {
        OutlineItem {
            entry,
            prompt: String::new(),
            reply: None,
        }
    }

    #[gpui::test]
    fn navigation_hover_in_a_nested_origin_holds_and_retires_locally(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::super::{ConversationArea, origin::Origin};
        cx.update(|cx| {
            let mut root = ConversationArea::new(cx);
            let mut leaf = ConversationArea::new(cx);
            leaf.outline_pointer.candidate = Some(2);
            leaf.outline_pointer.attention = Some(2);
            leaf.outline_pointer.active = true;
            let wait = leaf.outline_pointer.wait;
            let mut middle = ConversationArea::new(cx);
            middle.origin = Origin::Shown(Box::new(leaf));
            root.origin = Origin::Shown(Box::new(middle));
            assert!(root.navigation_held());
            root.navigation_retired();
            assert!(!root.navigation_held());
            let pointer = &root
                .origin
                .area()
                .unwrap()
                .origin
                .area()
                .unwrap()
                .outline_pointer;
            assert!(pointer.candidate.is_none() && pointer.attention.is_none());
            assert!(!pointer.active);
            assert_ne!(pointer.wait, wait);
        });
    }

    // Paseo's `resolveActivePromptSeq`: the last prompt at or before the
    // entry under the view's top; none before the first prompt.
    #[test]
    fn the_lit_prompt_is_the_last_at_or_before_the_top() {
        let outline = [item(3), item(10), item(40)];
        assert_eq!(lit(&outline, 0), None);
        assert_eq!(lit(&outline, 3), Some(0));
        assert_eq!(lit(&outline, 39), Some(1));
        assert_eq!(lit(&outline, 400), Some(2));
    }
}
