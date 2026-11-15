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
//! Hover intent is the pure view model: vertical travel keeps its deadline;
//! horizontal transit postpones activation. A cancelled ticket cannot reopen
//! a preview after the pointer leaves or the session changes.

use vessel_app_kit::ui::{
    AnyElement, App, Context, Div, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px, relative,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, popup};
#[cfg(test)]
use vessel_app_view::timeline::OutlineItem;
use vessel_app_view::timeline::{
    HoverChange, OUTLINE_INTENT, OutlineHover, OutlinePoint, OutlineSlot,
};

use crate::shell::Shell;

mod motion;

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

/// The screen owns focus and motion only; hover intent is the pure view model.
#[derive(Debug)]
pub(crate) struct Pointer {
    hover: OutlineHover,
    /// A numeric hover ticket belongs only to this retained local controller.
    identity: std::rc::Rc<()>,
    focus: gpui::FocusHandle,
    keyboard: Option<usize>,
    shown: Option<usize>,
    before: Option<usize>,
    turn: u64,
}

impl Pointer {
    #[cfg(test)]
    pub(super) const fn settling(&self) -> bool {
        self.hover.waiting()
    }

    /// Pending intent or a displayed preview protects the same retained owner.
    #[cfg(test)]
    pub(super) fn navigation_held(&self) -> bool {
        self.settling() || self.hover.attention().is_some() || self.shown.is_some()
    }

    pub(super) fn new(cx: &App) -> Self {
        Self {
            hover: OutlineHover::default(),
            identity: std::rc::Rc::new(()),
            focus: cx.focus_handle(),
            keyboard: None,
            shown: None,
            before: None,
            turn: 0,
        }
    }

    /// Preserve ticket identity across session changes, so an old timer stays old.
    pub(super) fn clear(&mut self) {
        let _cancelled = self.hover.leave();
        self.keyboard = None;
        self.shown = None;
        self.before = None;
        self.turn = 0;
    }

    pub(super) fn leave(&mut self) {
        let _cancelled = self.hover.leave();
        self.shown = None;
    }
}

impl Shell {
    /// Changing the preference cancels a preview even while the rail is hidden.
    pub(crate) fn conversation_outline_changed(&mut self) {
        self.areas.conversation.outline_pointer.clear();
        self.areas.conversation.clear_prompts();
        if self
            .areas
            .session
            .open
            .as_ref()
            .is_some_and(|open| open.structured)
        {
            self.prompt_index_refresh();
        }
    }

    fn outline_point(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.conversation;
        let Some(slot) = OutlineSlot::new(index, area.prompt_rows().len()) else {
            return;
        };
        let change = area.outline_pointer.hover.point_at(slot);
        self.outline_effect(change, cx);
    }

    fn outline_effect(&mut self, change: HoverChange, cx: &mut Context<'_, Self>) {
        match change {
            HoverChange::None => {}
            HoverChange::Changed => {
                let pointer = &mut self.areas.conversation.outline_pointer;
                pointer.before = pointer.shown;
                pointer.shown = pointer.hover.attention().map(OutlineSlot::index);
                pointer.turn = pointer.turn.saturating_add(1);
                cx.notify();
            }
            HoverChange::Schedule(ticket) => {
                let identity =
                    std::rc::Rc::clone(&self.areas.conversation.outline_pointer.identity);
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(OUTLINE_INTENT).await;
                    let _gone = this.update(cx, |shell, cx| {
                        if !std::rc::Rc::ptr_eq(
                            &shell.areas.conversation.outline_pointer.identity,
                            &identity,
                        ) {
                            return;
                        }
                        let change = shell
                            .areas
                            .conversation
                            .outline_pointer
                            .hover
                            .activate(ticket);
                        if change == HoverChange::Changed
                            && (!shell.prefs.settings.appearance.chat_outline.on()
                                || shell.areas.conversation.list.width() < px(MIN_PANEL))
                        {
                            shell.areas.conversation.outline_pointer.leave();
                            return;
                        }
                        shell.outline_effect(change, cx);
                    });
                })
                .detach();
            }
        }
    }

    /// The pointer left the rail; delayed activations lose their ticket.
    fn outline_left(&mut self, cx: &mut Context<'_, Self>) {
        let change = self.areas.conversation.outline_pointer.hover.leave();
        self.outline_effect(change, cx);
    }

    fn outline_key(&mut self, event: &gpui::KeyDownEvent, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.conversation;
        let count = area.prompt_rows().len();
        if count == 0 {
            return;
        }
        let current = area
            .outline_pointer
            .keyboard
            .unwrap_or(0)
            .min(count.saturating_sub(1));
        match event.keystroke.key.as_str() {
            "up" | "down" => {
                let next =
                    vessel_app_view::timeline::step(count, current, event.keystroke.key == "down")
                        .index;
                area.outline_pointer.keyboard = Some(next);
                cx.notify();
                cx.stop_propagation();
            }
            "enter" | "space" => {
                self.outline_jump(current, cx);
                cx.stop_propagation();
            }
            _ => {}
        }
    }

    /// Go to prompt `item` of the outline.
    fn outline_jump(&mut self, item: usize, cx: &mut Context<'_, Self>) {
        let area = &self.areas.conversation;
        let Some(seq) = area.prompt_rows().get(item).map(|item| item.seq) else {
            return;
        };
        self.prompt_jump(seq, cx);
    }
}

/// The prompt whose turn is at the view's top: the last at or before the
/// entry the view starts in.
#[cfg(test)]
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
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.conversation;
    let rows = area.prompt_rows();
    if !shell.prefs.settings.appearance.chat_outline.on()
        || rows.len() < vessel_app_view::timeline::OUTLINE_MIN
        || area.list.width() < px(MIN_PANEL)
    {
        return None;
    }
    let (top_row, _) = area.list.top();
    // A row of the fork's origin lights nothing (the rail is the fork's).
    let lit = top_row
        .checked_sub(area.head())
        .and_then(|own| area.rows.entry(own))
        .and_then(|top| area.timeline.entries().get(top))
        .and_then(|entry| rows.active(entry.seq));
    let pointer = &area.outline_pointer;
    let attention = pointer.shown.or_else(|| {
        pointer
            .focus
            .is_focused(window)
            .then_some(pointer.keyboard.unwrap_or(0))
    });
    let neutral = tokens.neutral;
    let target = std::rc::Rc::new(area.prompt_target()?.clone());
    let epoch = area.prompt_epoch();
    let slots = rows.iter().map(|(index, item)| {
        let target = std::rc::Rc::clone(&target);
        let seq = item.seq;
        let is_lit = lit == Some(index);
        let to = motion::size(index, is_lit, attention, cx.reduce_motion());
        let from = motion::size(index, is_lit, pointer.before, cx.reduce_motion());
        let color = if attention == Some(index) {
            neutral.foreground
        } else if is_lit {
            neutral.foreground_extra_muted
        } else {
            neutral.border_accent
        };
        let card = (attention == Some(index)).then(|| preview(item.preview.to_owned(), tokens));
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
                shell.prompt_selected(&target, seq, epoch, cx);
            }))
            .child(motion::pill(from, to, color, pointer.turn))
            .children(card)
    });
    Some(
        div()
            .id("outline-rail")
            .debug_selector(|| "outline-rail".to_owned())
            .track_focus(&pointer.focus)
            .tab_group()
            .on_key_down(cx.listener(|shell, event, _window, cx| shell.outline_key(event, cx)))
            .absolute()
            .left(px(RAIL_LEFT))
            .top(relative(0.1))
            .bottom(relative(0.1))
            .w(px(RAIL))
            .flex()
            .flex_col()
            .justify_center()
            .on_hover(cx.listener(|shell, hovered: &bool, window, cx| {
                if *hovered {
                    let position = window.mouse_position();
                    if let Some(point) = OutlinePoint::new(position.x.into(), position.y.into()) {
                        shell.areas.conversation.outline_pointer.hover.enter(point);
                    }
                } else {
                    shell.outline_left(cx);
                }
            }))
            .on_mouse_move(
                cx.listener(|shell, event: &gpui::MouseMoveEvent, _window, cx| {
                    if let Some(point) =
                        OutlinePoint::new(event.position.x.into(), event.position.y.into())
                    {
                        let change = shell.areas.conversation.outline_pointer.hover.moved(point);
                        shell.outline_effect(change, cx);
                    }
                }),
            )
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
