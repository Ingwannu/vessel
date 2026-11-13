//! A slider of discrete steps, as the effort control the owner chose
//! (2026-11-13, captures of another product's composer, described in
//! `docs/plans/backend-m12-spike.md`, not kept here): a pill-shaped track
//! with a dot per step, filled from its start to a round white thumb; the
//! fill a colour or a gradient the caller gives; on top, when the caller
//! asks, small sparkles drifting along the fill and a sheen travelling
//! over it.
//!
//! [Decision Log]
//! - Purpose: choose one of a few ordered steps (an agent's thinking
//!   levels) by dragging, clicking or the arrow keys, with the step's
//!   strength shown by the fill.
//! - Values, measured on the captures: the track 24 tall and round, its
//!   dots 4 across, 13 in from either end; the thumb 28 across, white.
//!   On touch the target is 44 tall (the track and thumb are not larger).
//! - Choice: steps placed by fractions of the track (`relative`), so the
//!   first frame is right before the track was measured; the track's
//!   bounds, recorded each frame, turn a pointer's position into a step.
//!   A drag shows the step under the pointer (`SliderState::previewing`)
//!   and chooses it when released, so a drag across five steps sends one
//!   choice, not five. Drag and release are heard window-wide while
//!   dragging (the pointer may leave the track).
//! - Motion: frame-driven (GPUI's animation, as the switch's slide), and
//!   only while drawn and asked for: nothing moves when the caller says
//!   [`Motion::Still`] or when the slider is not drawn (its card closed).
//! - Differences: no tooltip over the thumb (the caller's word above the
//!   track names the step, as the captures do).

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    Animation, AnimationExt as _, AnyElement, App, Bounds, ElementId, FocusHandle, Hsla,
    MouseButton, MouseMoveEvent, MouseUpEvent, Pixels, Window, canvas, div, linear_color_stop,
    linear_gradient, px, relative, white,
};

use crate::theme::Tokens;

/// The track's height.
const TRACK: f32 = 24.0;
/// A dot's size.
const DOT: f32 = 4.0;
/// Half a dot, to centre it on its place.
const HALF_DOT: f32 = -2.0;
/// The steps' distance from the track's ends.
const INSET: f32 = 13.0;
/// The thumb's size.
const THUMB: f32 = 28.0;
/// Half the thumb, to centre it on its place.
const HALF_THUMB: f32 = -14.0;
/// The fill starts at the track's start, before the line.
const BEFORE_LINE: f32 = -13.0;
/// The pointer's target height.
const POINTER: f32 = 28.0;
/// The touch target's height.
const TOUCH: f32 = 44.0;

/// What the track's fill is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fill {
    /// One colour.
    Solid(Hsla),
    /// From one colour at the start to another at the end.
    Gradient(Hsla, Hsla),
}

/// How the fill moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    /// Not at all.
    Still,
    /// Sparkles drifting slowly.
    Slow,
    /// Sparkles drifting quickly, and a sheen travelling over the fill.
    Fast,
}

/// How a slider looks at its value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// The fill, from the start to the thumb.
    pub fill: Fill,
    /// Whether sparkles show on the fill.
    pub sparkles: bool,
    /// How it moves.
    pub motion: Motion,
}

/// A slider's own state, kept by its screen.
#[derive(Debug, Clone)]
pub struct SliderState {
    focus: FocusHandle,
    /// Where the steps' line was drawn last frame.
    line: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// While dragging: the step under the pointer.
    dragging: Rc<Cell<Option<usize>>>,
}

impl SliderState {
    /// A slider not being dragged.
    pub fn new(cx: &mut App) -> Self {
        Self {
            focus: cx.focus_handle(),
            line: Rc::new(Cell::new(None)),
            dragging: Rc::new(Cell::new(None)),
        }
    }

    /// While dragging, the step under the pointer (the caller names it).
    #[must_use]
    pub fn previewing(&self) -> Option<usize> {
        self.dragging.get()
    }

    /// Give it the keys (the arrow keys move it).
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus.focus(window, cx);
    }
}

/// What choosing a step does.
pub type OnStep = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// The step nearest `x` on a line of `steps` steps drawn in `line`.
fn step_at(x: Pixels, line: Bounds<Pixels>, steps: usize) -> usize {
    let width = f32::from(line.size.width);
    if width <= 0.0 {
        return 0;
    }
    let along = f32::from(line.left())
        .mul_add(-1.0, f32::from(x))
        .clamp(0.0, width);
    let fraction = along.mul_add(width.recip(), 0.0);
    let distance = |index: &usize| fraction.mul_add(-1.0, place(*index, steps)).abs();
    (0..steps)
        .min_by(|a, b| distance(a).total_cmp(&distance(b)))
        .unwrap_or(0)
}

/// Step `index`'s place along the line, a fraction of it.
fn place(index: usize, steps: usize) -> f32 {
    let last = steps.saturating_sub(1);
    if last == 0 {
        return 0.0;
    }
    let index = f32::from(u16::try_from(index).unwrap_or(u16::MAX));
    let last = f32::from(u16::try_from(last).unwrap_or(u16::MAX));
    index.mul_add(last.recip(), 0.0)
}

/// Where a slider's thumb is: `steps` steps, at `at` (`None`: no step,
/// the thumb at the start, nothing filled).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// How many steps.
    pub steps: usize,
    /// The step in effect.
    pub at: Option<usize>,
}

/// A slider at `position`. `on_step` is called with a step clicked,
/// dragged to and released, or reached by a key.
#[must_use]
pub fn slider(
    id: impl Into<ElementId>,
    state: &SliderState,
    position: Position,
    look: Look,
    touch: bool,
    tokens: &Tokens,
    on_step: &OnStep,
) -> AnyElement {
    let id = id.into();
    let Position { steps, at } = position;
    let shown = state.dragging.get().or(at);
    let fraction = shown.map_or(0.0, |index| place(index, steps));
    // The steps' line: the track less its insets; the dots, the fill's
    // end and the thumb are placed on it by fractions.
    let mut line = div()
        .absolute()
        .top_0()
        .bottom_0()
        .left(px(INSET))
        .right(px(INSET))
        .child(drag_listener(state, steps, on_step));
    if shown.is_some() {
        line = line.child(fill(&id, look, fraction));
    }
    for index in 0..steps {
        let filled = shown.is_some_and(|shown| index < shown);
        line = line.child(dot(index, steps, filled, tokens));
    }
    line = line.child(thumb(fraction, tokens));
    // The track's rest: a step above the card (`surface3` in both).
    let track = div()
        .relative()
        .w_full()
        .h(px(TRACK))
        .rounded_full()
        .bg(tokens.neutral.surface3)
        .child(line);
    let press_line = Rc::clone(&state.line);
    let press_drag = Rc::clone(&state.dragging);
    let key_step = Rc::clone(on_step);
    div()
        .id(id)
        .track_focus(&state.focus)
        .w_full()
        .h(px(if touch { TOUCH } else { POINTER }))
        .flex()
        .items_center()
        .cursor_pointer()
        .child(track)
        .on_mouse_down(MouseButton::Left, move |event, window, _cx| {
            if let Some(line) = press_line.get() {
                press_drag.set(Some(step_at(event.position.x, line, steps)));
                window.refresh();
            }
        })
        .on_key_down(move |event, window, cx| {
            let last = steps.saturating_sub(1);
            let from = at.unwrap_or(0);
            let to = match event.keystroke.key.as_str() {
                "left" | "down" => from.saturating_sub(1),
                "right" | "up" => from.saturating_add(1).min(last),
                "home" => 0,
                "end" => last,
                _ => return,
            };
            cx.stop_propagation();
            if Some(to) != at {
                key_step(to, window, cx);
            }
        })
        .into_any_element()
}

/// Records where the steps' line is drawn, and while dragging hears the
/// pointer anywhere in the window: its moves pick the step shown, its
/// release chooses it.
fn drag_listener(state: &SliderState, steps: usize, on_step: &OnStep) -> AnyElement {
    let probe_line = Rc::clone(&state.line);
    let drag = Rc::clone(&state.dragging);
    let on_step = Rc::clone(on_step);
    canvas(
        move |bounds, _window, _cx| probe_line.set(Some(bounds)),
        move |bounds, (), window, _cx| {
            if drag.get().is_none() {
                return;
            }
            let moving = Rc::clone(&drag);
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, _cx| {
                if phase.bubble() && moving.get().is_some() {
                    let step = step_at(event.position.x, bounds, steps);
                    if moving.get() != Some(step) {
                        moving.set(Some(step));
                        window.refresh();
                    }
                }
            });
            let releasing = Rc::clone(&drag);
            let chosen = Rc::clone(&on_step);
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if phase.bubble() && event.button == MouseButton::Left {
                    if let Some(step) = releasing.take() {
                        chosen(step, window, cx);
                    }
                    window.refresh();
                }
            });
        },
    )
    .absolute()
    .size_full()
    .into_any_element()
}

/// The fill, from the track's start to the thumb at `fraction`.
fn fill(id: &ElementId, look: Look, fraction: f32) -> AnyElement {
    let colour = match look.fill {
        Fill::Solid(colour) => colour.into(),
        Fill::Gradient(from, to) => linear_gradient(
            90.0,
            linear_color_stop(from, 0.0),
            linear_color_stop(to, 1.0),
        ),
    };
    div()
        .absolute()
        .top(px(0.0))
        .bottom(px(0.0))
        .left(px(BEFORE_LINE))
        .right(relative(fraction.mul_add(-1.0, 1.0)))
        .rounded_full()
        .bg(colour)
        .overflow_hidden()
        .when(look.sparkles, |fill| fill.child(sparkles(id, look.motion)))
        .when(look.motion == Motion::Fast, |fill| fill.child(sheen(id)))
        .into_any_element()
}

/// Step `index`'s dot, lighter over the fill.
fn dot(index: usize, steps: usize, filled: bool, tokens: &Tokens) -> AnyElement {
    let colour = if filled {
        white().opacity(0.5)
    } else {
        tokens.neutral.foreground_muted.opacity(0.6)
    };
    div()
        .absolute()
        .top(relative(0.5))
        .mt(px(HALF_DOT))
        .left(relative(place(index, steps)))
        .ml(px(HALF_DOT))
        .size(px(DOT))
        .rounded_full()
        .bg(colour)
        .into_any_element()
}

/// The thumb, a white disc at `fraction`.
fn thumb(fraction: f32, tokens: &Tokens) -> AnyElement {
    div()
        .absolute()
        .top(relative(0.5))
        .mt(px(HALF_THUMB))
        .left(relative(fraction))
        .ml(px(HALF_THUMB))
        .size(px(THUMB))
        .rounded_full()
        .bg(white())
        .shadow(crate::popup::shadow_md(tokens))
        .into_any_element()
}

/// Where the sparkles sit, as fractions of the fill's width and height
/// (fixed, so they drift together rather than flicker).
const SPARKS: [(f32, f32, f32); 12] = [
    (0.04, 0.30, 0.9),
    (0.11, 0.70, 0.6),
    (0.19, 0.45, 0.8),
    (0.27, 0.20, 0.5),
    (0.35, 0.65, 0.9),
    (0.43, 0.35, 0.6),
    (0.52, 0.75, 0.8),
    (0.60, 0.25, 0.7),
    (0.68, 0.55, 0.9),
    (0.76, 0.30, 0.5),
    (0.85, 0.70, 0.8),
    (0.93, 0.40, 0.6),
];

/// A sparkle's size.
const SPARK: f32 = 2.0;

/// Sparkles over the fill, drifting at `motion`'s pace.
fn sparkles(id: &ElementId, motion: Motion) -> AnyElement {
    let layer = |shift: f32| {
        let mut layer = div().absolute().top_0().left_0().size_full();
        for (x, y, alpha) in SPARKS {
            let x = x.mul_add(1.0, shift).fract();
            layer = layer.child(
                div()
                    .absolute()
                    .left(relative(x))
                    .top(relative(y))
                    .size(px(SPARK))
                    .rounded_full()
                    .bg(white().opacity(alpha)),
            );
        }
        layer
    };
    let period = match motion {
        Motion::Still => return layer(0.0).into_any_element(),
        Motion::Slow => Duration::from_millis(9000),
        Motion::Fast => Duration::from_millis(2400),
    };
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .with_animation(
            ElementId::NamedChild(std::sync::Arc::new(id.clone()), "sparkles".into()),
            Animation::new(period).repeat(),
            move |frame, delta| frame.child(layer(delta)),
        )
        .into_any_element()
}

/// A sheen's width.
const SHEEN: f32 = 56.0;

/// A soft light band crossing the fill, quickly (fast mode on).
fn sheen(id: &ElementId) -> AnyElement {
    let band = |delta: f32| {
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(relative(delta.mul_add(1.4, -0.3)))
            .w(px(SHEEN))
            .bg(linear_gradient(
                90.0,
                linear_color_stop(white().opacity(0.0), 0.0),
                linear_color_stop(white().opacity(0.28), 1.0),
            ))
    };
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .with_animation(
            ElementId::NamedChild(std::sync::Arc::new(id.clone()), "sheen".into()),
            Animation::new(Duration::from_millis(1300)).repeat(),
            move |frame, delta| frame.child(band(delta)),
        )
        .into_any_element()
}

/// Paseo's activity indicator: three quarters of a ring turning, a turn
/// every 0.8 s, in `colour`, `size` across. Frame-driven while drawn, as
/// the slider's motion.
#[must_use]
pub fn spinner(id: impl Into<ElementId>, size: f32, colour: Hsla) -> AnyElement {
    gpui::svg()
        .path(crate::assets::Glyph::Spinner.path())
        .size(px(size))
        .flex_shrink_0()
        .text_color(colour)
        .with_animation(
            id,
            Animation::new(Duration::from_millis(800)).repeat(),
            |svg, delta| {
                svg.with_transformation(gpui::Transformation::rotate(gpui::percentage(delta)))
            },
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::{place, step_at};

    #[test]
    fn a_pointer_picks_the_nearest_step() {
        let line = Bounds::new(point(px(100.0), px(0.0)), size(px(200.0), px(24.0)));
        assert_eq!(step_at(px(90.0), line, 6), 0);
        assert_eq!(step_at(px(100.0), line, 6), 0);
        assert_eq!(step_at(px(139.0), line, 6), 1);
        assert_eq!(step_at(px(181.0), line, 6), 2);
        assert_eq!(step_at(px(300.0), line, 6), 5);
        assert_eq!(step_at(px(400.0), line, 6), 5);
        assert_eq!(step_at(px(150.0), line, 1), 0);
    }

    #[test]
    fn steps_are_spread_from_end_to_end() {
        assert!((place(0, 6) - 0.0).abs() < 1e-6);
        assert!((place(5, 6) - 1.0).abs() < 1e-6);
        assert!((place(2, 5) - 0.5).abs() < 1e-6);
        assert!((place(0, 1) - 0.0).abs() < 1e-6);
    }
}
