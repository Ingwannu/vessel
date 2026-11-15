//! The context meter left of the send button: Paseo's ring and its card
//! (`components/context-window-meter.tsx`, `context-window-details.tsx`),
//! with GOAT's "Compact context" in the card (`chat/ContextWindowMeter.tsx`).
//! The figures are the agent's (`view::timeline::meter_of`).
//!
//! [Decision Log]
//! - Purpose: how full the agent's context is, at a glance where the
//!   person writes, and a way to make room (`/compact`) before it fills.
//! - Constraint: the composer's card is the lead's; the meter is drawn in
//!   the slot the lead's `composer()` keeps left of the send button
//!   (`conversation::meter`). GPUI draws no SVG built at run time, so the
//!   ring is painted (`canvas`).
//! - The references: Paseo's ring, 14 in a 28 control, a stroke of 2, the
//!   track `surface3`, the arc clockwise from twelve in `foregroundMuted`,
//!   amber from 70%, red past 90%; its card on hover (a sheet on a phone):
//!   "Context window", "N% used", "used / max tokens", the session's cost,
//!   or "No context data". GOAT adds "Compact context" to the card,
//!   disabled with its reason while it cannot run.
//! - Choice: Paseo's ring and card; GOAT's button when the agent offers
//!   `/compact`, sending it as a prompt (the agent's own command), and
//!   disabled, saying why, while a turn runs or once the session has
//!   ended (a prompt to an ended session starts it again). The card opens on hover and on a press,
//!   so nothing depends on hover (a phone's press opens it, a press
//!   outside closes it); leaving the ring closes it after 150 ms, time to
//!   reach the button in it (GOAT's close delay).
//! - Differences, kept: the card, not a sheet, on a phone; the arc ends
//!   square (Paseo's are round).

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{Anchor, Bounds, PathBuilder, Pixels, Point, anchored, canvas, deferred, point};
use vessel_app_kit::theme::Scheme;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, Hsla, InteractiveElement as _, IntoElement as _,
    MouseButton, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, popup};
use vessel_app_link::Command;
use vessel_app_view::session::is_live;
use vessel_app_view::timeline::{Fullness, Meter, meter_of};
use vessel_proto::Delivery;

use crate::shell::Shell;

/// The ring, its stroke, and the control it sits in.
const RING: f32 = 14.0;
const STROKE: f32 = 2.0;
const CONTROL: f32 = 28.0;

/// The straight pieces a whole ring is drawn with.
const SEGMENTS: u16 = 48;

/// How long the card stays after the pointer leaves the ring (GOAT's).
const CLOSE_DELAY: Duration = Duration::from_millis(150);

/// Whether the pointer is over the ring, over the card.
#[derive(Debug, Default, Clone, Copy)]
struct Over {
    ring: bool,
    card: bool,
}

/// Where the meter is and whether its card shows.
#[derive(Debug, Default)]
pub(crate) struct MeterState {
    /// Where the ring was last drawn: the card's anchor.
    place: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// Where the pointer is.
    over: Over,
    /// Opened by a press: stays until a press elsewhere.
    pinned: bool,
    shown: bool,
    /// Bumped by every hover change; a close waits and checks it.
    wait: u64,
}

impl Shell {
    fn meter_hover(&mut self, ring: Option<bool>, card: Option<bool>, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.conversation.meter;
        if let Some(over) = ring {
            state.over.ring = over;
        }
        if let Some(over) = card {
            state.over.card = over;
        }
        state.wait = state.wait.wrapping_add(1);
        if state.over.ring || state.over.card {
            state.shown = true;
            cx.notify();
            return;
        }
        let wait = state.wait;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(CLOSE_DELAY).await;
            let _gone = this.update(cx, |shell, cx| {
                let state = &mut shell.areas.conversation.meter;
                if state.wait == wait && !state.pinned && !state.over.ring && !state.over.card {
                    state.shown = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn meter_press(&mut self, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.conversation.meter;
        state.pinned = !state.pinned;
        state.shown = state.pinned || state.over.ring;
        cx.notify();
    }

    fn meter_close(&mut self, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.conversation.meter;
        if state.shown || state.pinned {
            *state = MeterState {
                place: Rc::clone(&state.place),
                wait: state.wait.wrapping_add(1),
                ..MeterState::default()
            };
            cx.notify();
        }
    }

    /// Send the agent's `/compact`.
    fn meter_compact(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(open) = &self.areas.session.open {
            self.command(Command::Prompt {
                machine: open.machine.clone(),
                session: open.session,
                text: "/compact".to_owned(),
                // Offered only while no turn runs (the button waits
                // otherwise), so it starts one.
                delivery: Delivery::Start,
            });
        }
        self.meter_close(cx);
    }
}

/// The ring and, while open, its card.
pub(crate) fn meter(
    shell: &Shell,
    tokens: &Tokens,
    _window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.conversation;
    // Only under a conversation that is shown.
    let _shown = area.shown.as_ref()?;
    let facts = area.timeline.facts();
    let meter = meter_of(facts.context.as_ref());
    let neutral = tokens.neutral;
    let color = match meter.map(|meter| meter.fullness) {
        Some(Fullness::Full) => neutral.destructive,
        // Paseo's `palette.amber[500]`, the kit's status signal.
        Some(Fullness::Low) => tokens.status().amber_500,
        Some(Fullness::Room) | None => neutral.foreground_muted,
    };
    let turns = meter.map(|meter| f32::from(meter.percent));
    let track = neutral.surface3;
    let ring = canvas(
        |_, _, _| (),
        move |bounds, (), window, _cx| {
            if let Some(path) = ring_path(bounds, 100.0) {
                window.paint_path(path, track);
            }
            if let Some(path) = turns.and_then(|percent| ring_path(bounds, percent)) {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(RING));
    let control = div()
        .id("context-meter")
        .relative()
        .size(px(CONTROL))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded_full()
        .cursor_pointer()
        .child(ring)
        .child(vessel_app_kit::picker::anchor_probe(&area.meter.place))
        .on_hover(cx.listener(|shell, over: &bool, _window, cx| {
            shell.meter_hover(Some(*over), None, cx);
        }))
        .on_click(cx.listener(|shell, _event, _window, cx| shell.meter_press(cx)));
    let card = area
        .meter
        .shown
        .then(|| card(shell, meter, tokens, cx))
        .zip(area.meter.place.get())
        .map(|(card, bounds)| {
            deferred(
                anchored()
                    .position(bounds.top_right())
                    .anchor(Anchor::BottomRight)
                    .offset(point(px(0.0), px(-8.0)))
                    .snap_to_window_with_margin(px(8.0))
                    .child(card),
            )
            .with_priority(4)
        });
    Some(div().child(control).children(card).into_any_element())
}

/// The card: Paseo's details, GOAT's compact button.
fn card(
    shell: &Shell,
    meter: Option<Meter>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.conversation();
    let facts = shell.areas.conversation.timeline.facts();
    let neutral = tokens.neutral;
    let surface = match tokens.scheme {
        Scheme::Light => neutral.surface0,
        Scheme::Dark => neutral.surface2,
    };
    let summary = summary(shell, meter, tokens);
    let offers_compact = facts.commands.as_ref().is_some_and(|commands| {
        commands
            .iter()
            .any(|command| command.name.as_str() == "compact")
    });
    let held = compact_held(shell);
    div()
        .id("context-meter-card")
        // A press on the card stays on it: without this it reached what
        // lies under it (the ended banner's Start again).
        .occlude()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .py(px(4.0))
        .px(px(8.0))
        .rounded(px(12.0))
        .bg(surface)
        .border_1()
        .border_color(neutral.border_accent)
        .shadow(popup::shadow_md(tokens))
        .face(Type::Body)
        .text_size(px(14.0))
        .line_height(px(20.0))
        .on_hover(cx.listener(|shell, over: &bool, _window, cx| {
            shell.meter_hover(None, Some(*over), cx);
        }))
        .on_mouse_down_out(cx.listener(|shell, _event, _window, cx| {
            if !shell.areas.conversation.meter.over.ring {
                shell.meter_close(cx);
            }
        }))
        .child(summary)
        .when(offers_compact, |card| {
            card.child(compact(words.meter_compact(), held.is_none(), tokens, cx))
                .children(held.map(|why| {
                    div()
                        .pb(px(4.0))
                        .text_size(px(12.0))
                        .text_color(neutral.foreground_muted)
                        .child(why)
                }))
        })
        .into_any_element()
}

/// The card's words: the title, how much of the context is used and in
/// tokens (or that there is no figure yet), and the cost when the agent
/// reports one.
fn summary(shell: &Shell, meter: Option<Meter>, tokens: &Tokens) -> Div {
    let words = shell.t.lang.conversation();
    let facts = shell.areas.conversation.timeline.facts();
    let neutral = tokens.neutral;
    let detail = |text: String| div().text_color(neutral.foreground_muted).child(text);
    let mut summary = div().flex().flex_col().gap(px(6.0)).min_w(px(200.0)).child(
        div()
            .text_color(neutral.foreground)
            .child(words.meter_title()),
    );
    summary = match meter {
        Some(meter) => summary
            .child(
                div()
                    .text_color(neutral.foreground)
                    .child(words.meter_used(&meter)),
            )
            .child(detail(words.meter_tokens_line(&meter))),
        None => summary.child(detail(words.meter_no_data().to_owned())),
    };
    if let Some(cost) = facts
        .context
        .as_ref()
        .and_then(|context| context.cost.as_ref())
    {
        summary = summary.child(detail(
            words.meter_cost(cost.amount.get(), cost.unit.as_str()),
        ));
    }
    summary
}

/// Why `/compact` cannot go now, said under the button (GOAT's disabled
/// reason): the session has ended (a prompt to it would start it again,
/// not compact it), or a turn runs. Before the node describes the
/// session, the session area's word for whether it runs.
fn compact_held(shell: &Shell) -> Option<&'static str> {
    let words = shell.t.lang.conversation();
    let ended = shell.areas.conversation.info.as_deref().map_or_else(
        || {
            shell
                .areas
                .session
                .open
                .as_ref()
                .is_some_and(|open| !open.live)
        },
        |info| !is_live(info),
    );
    if ended {
        Some(words.meter_compact_ended())
    } else if shell.areas.session.conversation.turn_running() {
        Some(words.meter_compact_busy())
    } else {
        None
    }
}

/// GOAT's outline button, the card's width: sends `/compact`.
fn compact(
    label: &'static str,
    enabled: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> impl gpui::IntoElement {
    let neutral = tokens.neutral;
    let hover: Hsla = neutral.surface3;
    div()
        .id("context-meter-compact")
        .flex()
        .items_center()
        .justify_center()
        .mb(px(4.0))
        .py(px(4.0))
        .px(px(8.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(neutral.border_accent)
        .text_size(px(13.0))
        .text_color(neutral.foreground)
        .when(!enabled, |button| button.opacity(0.5))
        .when(enabled, |button| {
            button
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                    cx.stop_propagation();
                })
                .on_click(cx.listener(|shell, _event, _window, cx| shell.meter_compact(cx)))
        })
        .child(label)
}

/// A ring's stroke from twelve o'clock, clockwise, `percent` of the way
/// round (100: the whole track), in `bounds`.
fn ring_path(bounds: Bounds<Pixels>, percent: f32) -> Option<gpui::Path<Pixels>> {
    let mut points = ring_points(bounds, percent);
    let whole = percent >= 100.0;
    if whole {
        // The last point is the first again: drawn and then closed, the
        // joint at twelve was stroked twice and the translucent track
        // showed a brighter tick there (0.0.0.0.19 smoke).
        points.truncate(points.len().saturating_sub(1));
    }
    let (first, rest) = points.split_first()?;
    let mut path = PathBuilder::stroke(px(STROKE));
    path.move_to(*first);
    for next in rest {
        path.line_to(*next);
    }
    // The whole track is Paseo's closed circle. Left open, its two square
    // ends met at twelve with a notch, and with no context data (the track
    // alone, as an ended session shows it) the ring read as a loading
    // spinner beside Send (the coordinator's 0.0.0.0.18 smoke).
    if whole {
        path.close();
    }
    path.build().ok()
}

#[expect(
    clippy::float_arithmetic,
    clippy::arithmetic_side_effects,
    reason = "a ring's points are trigonometry on the canvas's floats"
)]
fn ring_points(bounds: Bounds<Pixels>, percent: f32) -> Vec<Point<Pixels>> {
    let center = bounds.center();
    let radius = (RING - STROKE) / 2.0;
    let turn = std::f32::consts::TAU * percent.clamp(0.0, 100.0) / 100.0;
    (0..=SEGMENTS)
        .map(|step| {
            let angle = turn * f32::from(step) / f32::from(SEGMENTS);
            point(
                center.x + px(radius * angle.sin()),
                center.y - px(radius * angle.cos()),
            )
        })
        .collect()
}
