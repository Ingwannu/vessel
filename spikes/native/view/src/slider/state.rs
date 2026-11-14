//! The slider's state through time: what the pointer, the keys and the
//! wheel did, and what each frame draws. The app keeps one per card and
//! hands each frame to the kit's control, which only draws it.
#![expect(
    clippy::float_arithmetic,
    reason = "times and positions are float formulas"
)]

use super::flair::{self, Dot, FastStops};
use super::motion::{DRAG_GLIDE, GLIDE, GROW, SETTLE, Spring, Tween, Wheel};
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
    /// Whether anything moves: the control asks for the next frame.
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

/// A press: the stop in effect when it began.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Press {
    from: Option<usize>,
}

/// The slider's state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Motion {
    position: Option<Tween>,
    thumb: Option<Spring>,
    last: Option<f32>,
    pressed: Option<Press>,
    preview: Option<usize>,
    moved: bool,
    hovered: bool,
    wheel: Wheel,
    top: Option<(bool, f32)>,
    fast: Option<(bool, f32)>,
    dots: Option<(f32, f32)>,
    warning: Option<f32>,
    burst: Option<f32>,
    field: Option<f32>,
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

    /// A press on stop `stop` (the track maps it), with `current` in
    /// effect: the thumb goes there at once, as a click.
    pub fn press(&mut self, stop: usize, current: Option<usize>) {
        self.pressed = Some(Press { from: current });
        self.preview = Some(stop);
        self.moved = false;
    }

    /// The pointer, pressed, over stop `stop`.
    pub fn drag(&mut self, stop: usize) {
        if self.pressed.is_some() && self.preview != Some(stop) {
            self.preview = Some(stop);
            self.moved = true;
        }
    }

    /// The release: the stop to send, when it differs from the one in
    /// effect at the press.
    pub fn release(&mut self) -> Option<usize> {
        let pressed = self.pressed.take()?;
        let released = self.preview.take()?;
        self.moved = false;
        commits(pressed.from, released).then_some(released)
    }

    /// The pointer over the thumb or not.
    pub const fn hover(&mut self, over: bool) {
        self.hovered = over;
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
        self.last = Some(now);
        let shown = self.shown(input.current);
        let at_maximum = shown.is_some() && shown == input.maximum;
        let (fraction, gliding) = self.glide(shown, input, now);
        let (thumb, springing) = self.spring(input.reduce, elapsed);
        let top_since = self.top_stop(at_maximum, now);
        let (fast_stops, fast_since) = self.fast_mode(input.fast, fraction, now);
        if self.pressed.is_some() && at_maximum {
            let _shown = self.warning.get_or_insert(now);
        } else {
            self.warning = None;
        }
        let stops = (0..input.stops)
            .map(|index| {
                let filled = shown.is_some_and(|shown| index <= shown);
                (
                    stop_look(filled, at_maximum, top_since, fast_stops, input),
                    filled,
                )
            })
            .collect();
        let (burst, dots) = self.particles(input.reduce, now);
        let field = self.field.map(|reveal| {
            if input.reduce {
                (0.0, flair::REVEAL_TIME)
            } else {
                (now, now - reveal)
            }
        });
        let warning = self.warning.map(|since| now - since);
        let stops_moving = !input.reduce && (top_since < STOPS_SETTLE || fast_since < STOPS_SETTLE);
        let moving = gliding
            || springing
            || !burst.is_empty()
            || (!input.reduce && (!dots.is_empty() || field.is_some()))
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

    /// The thumb's place gliding to `shown`'s; whether it still glides.
    fn glide(&mut self, shown: Option<usize>, input: Input, now: f32) -> (Option<f32>, bool) {
        let Some(target) = shown.map(|stop| fraction(stop, input.stops)) else {
            self.position = None;
            return (None, false);
        };
        let length = if input.reduce {
            0.0
        } else if self.moved {
            DRAG_GLIDE
        } else {
            GLIDE
        };
        let tween = self.position.get_or_insert(Tween::rest(target));
        tween.glide(target, now, length);
        (Some(tween.at(now)), tween.moving(now))
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
            let _started = self.dots.get_or_insert((now, fraction.unwrap_or(0.0)));
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
    fn particles(&mut self, reduce: bool, now: f32) -> (Vec<(Dot, f32)>, Vec<Dot>) {
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
            Some((start, from)) if !reduce => (0..flair::FAST_DOTS)
                .map(|index| flair::fast_dot(index, now - start, from))
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
