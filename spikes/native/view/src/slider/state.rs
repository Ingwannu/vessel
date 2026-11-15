//! The slider's state through time: what the pointer, the keys and the
//! wheel did, and what each frame draws. The app keeps one per card and
//! hands each frame to the kit's control, which only draws it.
//!
//! [Decision Log] (the coordinator's measure of 0.0.0.0.13, 2026-11-14:
//! under a software renderer the card at its top stop cost about three
//! cores for as long as it stayed open)
//! - Purpose: the decorations cost nothing once they have been seen.
//! - Choice: the field and Fast's dots run on their own clock, which runs
//!   while the pointer is over the card or the slider is held, and for 2 s
//!   after it leaves or after a change (reaching the top stop, Fast
//!   switched), then slows to a stop over 1.5 s. While anything moves the
//!   control asks for the display's every frame (the coordinator,
//!   2026-11-14: the decorations' earlier 30 a second read as choppy on
//!   the owner's GPU; a software renderer holds motion still anyway).
//! - Difference, named: the Codex app's field and dots move for as long
//!   as its menu is open; here they come to rest a few seconds after the
//!   pointer leaves the card (the dots fade as they slow).
#![expect(
    clippy::float_arithmetic,
    reason = "times and positions are float formulas"
)]

use super::drag::{CATCH, DRAG_START, Pointer, RELEASE, magnet, nearest};
use super::flair::{self, Dot, FastStops};
use super::motion::{GLIDE, GROW, SETTLE, Spring, Tween, Wheel};
use super::{THUMB, THUMB_GROWN, commits, fraction};

/// What a frame is drawn for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Input {
    /// How many stops.
    pub stops: usize,
    /// The stop in effect (`None`: the agent's default, no stop).
    pub current: Option<usize>,
    /// The top stop, drawn in the maximum's colours.
    pub maximum: Option<usize>,
    /// Whether Fast is on.
    pub fast: bool,
    /// Whether the person asked for reduced motion.
    pub reduce: bool,
}

/// One frame of the slider.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// The thumb's place, a share of the way between the first and last
    /// stop (`None`: no stop, no thumb fill).
    pub fraction: Option<f32>,
    /// The thumb's scale (1 at rest, 32/28 grown).
    pub thumb: f32,
    /// Each stop's look, and whether it is under the fill.
    pub stops: Vec<(Dot, bool)>,
    /// The burst's particles, each with its share of blue.
    pub burst: Vec<(Dot, f32)>,
    /// Fast's dots ([`flair::fast_dot`]).
    pub dots: Vec<Dot>,
    /// The field, when the thumb is on the top stop: seconds since it
    /// started moving, and since its reveal began.
    pub field: Option<(f32, f32)>,
    /// The usage warning, seconds since it showed.
    pub warning: Option<f32>,
    /// Whether the shown stop is the top one (its colours).
    pub at_maximum: bool,
    /// Whether anything moves: the control asks for the display's next
    /// frame.
    pub moving: bool,
}

/// How long the stops take to come back after Fast is turned off.
const FAST_BACK: f32 = 0.5;
/// How long the stops take to come back after leaving the top stop.
const TOP_BACK: f32 = 0.2;
/// How long the stops' animations at the top stop and with Fast run.
const STOPS_SETTLE: f32 = 1.3;
/// Seconds treated as "long ago" for a state the card opened in.
const LONG_AGO: f32 = 1.0e3;
/// How long the decorations run on after the pointer leaves or a change.
const HOLD: f32 = 2.0;
/// How long they take to come to rest after that.
const STILLING: f32 = 1.5;

/// A press: the stop in effect when it began, and the pointer since.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Press {
    from: Option<usize>,
    /// Where it began, points from the track's start.
    start: f32,
    /// Whether the pointer has moved far enough to drag the thumb.
    dragging: bool,
    /// The pointer, a share of the way between the end stops.
    at: f32,
    /// The line between the end stops, points.
    line: f32,
}

/// A gap the thumb no longer needs a frame to close (a share of the way).
const AT_REST: f32 = 1.0e-4;

/// The slider's state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Motion {
    position: Option<Tween>,
    /// The thumb springing onto its stop after a drag.
    snap: Option<Spring>,
    /// Where the thumb was drawn last, a share of the way.
    place: Option<f32>,
    /// The gap a drag's thumb is closing to the pointer's place.
    catch: f32,
    thumb: Option<Spring>,
    last: Option<f32>,
    pressed: Option<Press>,
    preview: Option<usize>,
    hovered: bool,
    wheel: Wheel,
    top: Option<(bool, f32)>,
    fast: Option<(bool, f32)>,
    dots: Option<(f32, f32)>,
    warning: Option<f32>,
    burst: Option<f32>,
    field: Option<f32>,
    /// The pointer over the card.
    engaged: bool,
    /// When the pointer last left, or the last change.
    active: f32,
    /// The decorations' own clock, seconds.
    clock: f32,
}

impl Motion {
    /// The stop shown: the one dragged to, else the one in effect.
    #[must_use]
    pub fn shown(&self, current: Option<usize>) -> Option<usize> {
        self.preview.or(current)
    }

    /// Whether a drag is in progress.
    #[must_use]
    pub const fn dragging(&self) -> bool {
        self.pressed.is_some()
    }

    /// A press at `pointer` on a slider of `stops`, with `current` in
    /// effect: the nearest stop is shown and the thumb glides there, as a
    /// click, until the pointer moves [`DRAG_START`] away.
    pub fn press(&mut self, pointer: Pointer, stops: usize, current: Option<usize>) {
        let at = pointer.fraction();
        self.pressed = Some(Press {
            from: current,
            start: pointer.x,
            dragging: false,
            at,
            line: pointer.line(),
        });
        self.preview = Some(nearest(at, stops));
    }

    /// The pointer, pressed, at `pointer`: once it has moved far enough
    /// the thumb follows it ([`super::drag`]), from where the thumb is,
    /// and the stop shown is the one whose reach it is in.
    pub fn drag(&mut self, pointer: Pointer, stops: usize) {
        let place = self.place;
        let Some(press) = self.pressed.as_mut() else {
            return;
        };
        press.at = pointer.fraction();
        press.line = pointer.line();
        if !press.dragging && (pointer.x - press.start).abs() >= DRAG_START {
            press.dragging = true;
            let drawn = magnet(press.at, stops, press.line);
            self.catch = place.map_or(0.0, |place| place - drawn);
        }
        self.preview = Some(nearest(press.at, stops));
    }

    /// The release: the stop to send, when it differs from the one in
    /// effect at the press. A dragged thumb springs onto its stop from
    /// where it is.
    pub fn release(&mut self) -> Option<usize> {
        let pressed = self.pressed.take()?;
        let released = self.preview.take()?;
        if pressed.dragging {
            self.snap = self.place.map(Spring::rest);
        }
        self.catch = 0.0;
        commits(pressed.from, released).then_some(released)
    }

    /// The pointer over the thumb or not.
    pub const fn hover(&mut self, over: bool) {
        self.hovered = over;
    }

    /// The pointer onto or off the card at `now`.
    pub const fn engage(&mut self, over: bool, now: f32) {
        self.engaged = over;
        self.active = now;
    }

    /// A wheel's `delta` units at `now`: the stops to move.
    pub fn wheel(&mut self, delta: f32, now: f32) -> i32 {
        self.wheel.add(delta, now)
    }

    /// The card closed: what was moving rests, a drag is dropped, and the
    /// next opening starts from the stop in effect.
    pub fn close(&mut self) {
        *self = Self::default();
    }

    /// The frame at `now` for `input`.
    pub fn frame(&mut self, input: Input, now: f32) -> Frame {
        let elapsed = self.last.map_or(0.0, |last| (now - last).max(0.0));
        if self.last.is_none() {
            // Opening the card is a change: the decorations run a while.
            self.active = now;
        }
        self.last = Some(now);
        let shown = self.shown(input.current);
        let at_maximum = shown.is_some() && shown == input.maximum;
        let (fraction, gliding) = self.place(shown, input, now, elapsed);
        let (thumb, springing) = self.spring(input.reduce, elapsed);
        let top_since = self.top_stop(at_maximum, now);
        let (fast_stops, fast_since) = self.fast_mode(input.fast, fraction, now);
        if top_since <= 0.0 || fast_since <= 0.0 {
            self.active = now;
        }
        let speed = self.pace(input.reduce, elapsed, now);
        if self.pressed.is_some() && at_maximum {
            let _shown = self.warning.get_or_insert(now);
        } else {
            self.warning = None;
        }
        // A stop's look follows the stop chosen; whether it is drawn over
        // the fill follows the thumb, which may be between stops.
        let stops = (0..input.stops)
            .map(|index| {
                let chosen = shown.is_some_and(|shown| index <= shown);
                let under = fraction
                    .is_some_and(|place| super::fraction(index, input.stops) <= place + AT_REST);
                (
                    stop_look(chosen, at_maximum, top_since, fast_stops, input),
                    under,
                )
            })
            .collect();
        let (burst, dots) = self.particles(input.reduce, now, speed);
        let clock = self.clock;
        let field = self.field.map(|reveal| {
            if input.reduce {
                (clock, flair::REVEAL_TIME)
            } else {
                (clock, now - reveal)
            }
        });
        let revealing = field.is_some_and(|(_, reveal)| reveal < flair::REVEAL_TIME);
        let warning = self.warning.map(|since| now - since);
        let stops_moving = !input.reduce && (top_since < STOPS_SETTLE || fast_since < STOPS_SETTLE);
        let decorating = !input.reduce && speed > 0.0 && (!dots.is_empty() || field.is_some());
        let moving = gliding
            || springing
            || !burst.is_empty()
            || decorating
            || (!input.reduce && revealing)
            || warning.is_some_and(|since| since < flair::SHIMMER_TIME)
            || stops_moving;
        Frame {
            fraction,
            thumb,
            stops,
            burst,
            dots,
            field,
            warning,
            at_maximum,
            moving,
        }
    }

    /// Advance the decorations' clock by `elapsed` at its pace now, and
    /// return the pace: 1 while engaged and for [`HOLD`] after, then
    /// slowing to 0 over [`STILLING`]; 0 under reduced motion.
    fn pace(&mut self, reduce: bool, elapsed: f32, now: f32) -> f32 {
        if self.engaged || self.pressed.is_some() {
            self.active = now;
        }
        let since = now - self.active;
        let speed = if reduce {
            0.0
        } else if since <= HOLD {
            1.0
        } else {
            1.0 - super::motion::EASE_OUT.at(((since - HOLD) / STILLING).clamp(0.0, 1.0))
        };
        self.clock += elapsed * speed;
        speed
    }

    /// The thumb's place, a share of the way, and whether it still moves:
    /// following a dragging pointer, springing onto its stop after a drag,
    /// else gliding to `shown`'s stop.
    fn place(
        &mut self,
        shown: Option<usize>,
        input: Input,
        now: f32,
        elapsed: f32,
    ) -> (Option<f32>, bool) {
        let Some(target) = shown.map(|stop| fraction(stop, input.stops)) else {
            self.position = None;
            self.snap = None;
            self.place = None;
            return (None, false);
        };
        let (place, moving) = if let Some(press) = self.pressed.filter(|press| press.dragging)
            && !input.reduce
        {
            self.catch *= (-elapsed / CATCH).exp();
            let place = magnet(press.at, input.stops, press.line) + self.catch;
            self.position = Some(Tween::rest(place));
            (place, self.catch.abs() > AT_REST)
        } else if let Some(spring) = self.snap.as_mut()
            && !input.reduce
        {
            spring.step(target, RELEASE, elapsed);
            if spring.settled(target) {
                self.snap = None;
                self.position = Some(Tween::rest(target));
                (target, false)
            } else {
                let place = spring.value;
                self.position = Some(Tween::rest(place));
                (place, true)
            }
        } else {
            self.snap = None;
            let length = if input.reduce { 0.0 } else { GLIDE };
            let tween = self.position.get_or_insert(Tween::rest(target));
            tween.glide(target, now, length);
            (tween.at(now), tween.moving(now))
        };
        self.place = Some(place);
        (Some(place), moving)
    }

    /// The thumb's scale after `elapsed` seconds; whether it still moves.
    fn spring(&mut self, reduce: bool, elapsed: f32) -> (f32, bool) {
        let target = if self.hovered || self.pressed.is_some() {
            THUMB_GROWN / THUMB
        } else {
            1.0
        };
        let spring = self.thumb.get_or_insert(Spring::rest(1.0));
        if reduce {
            *spring = Spring::rest(target);
        } else {
            let params = if target > 1.0 { GROW } else { SETTLE };
            spring.step(target, params, elapsed);
        }
        (spring.value, !spring.settled(target))
    }

    /// Track the top stop: arriving starts the burst and the field's
    /// reveal (a card opening on it shows the field settled). Seconds
    /// since the last arrival or departure.
    fn top_stop(&mut self, at_maximum: bool, now: f32) -> f32 {
        match self.top {
            None => self.top = Some((at_maximum, now - LONG_AGO)),
            Some((on, _)) if on != at_maximum => {
                self.top = Some((at_maximum, now));
                if at_maximum {
                    self.burst = Some(now);
                    self.field = Some(now);
                }
            }
            Some(_) => {}
        }
        if !at_maximum {
            self.field = None;
        } else if self.field.is_none() {
            self.field = Some(now - LONG_AGO);
        }
        self.top.map_or(LONG_AGO, |(_, since)| now - since)
    }

    /// Track Fast (a card opening with it on shows it settled): how the
    /// stops stand, and seconds since it changed. The dots start from the
    /// thumb's place when it turns on.
    fn fast_mode(&mut self, on: bool, fraction: Option<f32>, now: f32) -> (FastStops, f32) {
        match self.fast {
            None => self.fast = Some((on, now - LONG_AGO)),
            Some((was, _)) if was != on => self.fast = Some((on, now)),
            Some(_) => {}
        }
        let since = self.fast.map_or(LONG_AGO, |(_, since)| now - since);
        if on {
            let clock = self.clock;
            let _started = self.dots.get_or_insert((clock, fraction.unwrap_or(0.0)));
        } else {
            self.dots = None;
        }
        let stops = match (on, since) {
            (true, since) => FastStops::On { since },
            (false, since) if since < FAST_BACK => FastStops::Back { since },
            (false, _) => FastStops::Off,
        };
        (stops, since)
    }

    /// The burst's particles and Fast's dots at `now` (none with reduced
    /// motion).
    fn particles(&mut self, reduce: bool, now: f32, speed: f32) -> (Vec<(Dot, f32)>, Vec<Dot>) {
        let burst: Vec<(Dot, f32)> = match self.burst {
            Some(start) if !reduce => (0..flair::BURST_PARTICLES.len())
                .filter_map(|index| {
                    flair::burst(index, now - start).map(|dot| (dot, flair::burst_blue(index)))
                })
                .collect(),
            _ => Vec::new(),
        };
        if burst.is_empty() {
            self.burst = None;
        }
        let dots = match self.dots {
            Some((start, from)) if !reduce && speed > 0.0 => (0..flair::FAST_DOTS)
                .map(|index| {
                    let mut dot = flair::fast_dot(index, self.clock - start, from);
                    dot.opacity *= speed;
                    dot
                })
                .collect(),
            _ => Vec::new(),
        };
        (burst, dots)
    }
}

/// A stop's look in this frame.
fn stop_look(filled: bool, at_maximum: bool, top_since: f32, fast: FastStops, input: Input) -> Dot {
    let top = (at_maximum && !input.fast).then_some(top_since);
    if !at_maximum && top_since < TOP_BACK && !input.fast {
        flair::stop_leaving_top(top_since)
    } else if input.reduce {
        let fast = match fast {
            FastStops::On { .. } => FastStops::On { since: LONG_AGO },
            FastStops::Back { .. } | FastStops::Off => FastStops::Off,
        };
        flair::stop(filled, top.map(|_| LONG_AGO), fast)
    } else {
        flair::stop(filled, top, fast)
    }
}
