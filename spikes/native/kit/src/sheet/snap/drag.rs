//! Native pointer sampling and content-scroll handoff. GPUI supplies no
//! recognizer velocity, so a short bounded history supplies the projection
//! input without retaining an event stream for the lifetime of a sheet.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use gpui::{Pixels, Point};

use super::{Origin, Stop, positions::Positions};

const HISTORY: usize = 8;
const VELOCITY_WINDOW: Duration = Duration::from_millis(100);
// GPUI Div's native drag threshold, in logical pointer pixels.
const POINTER_SLOP: f32 = 2.0;
// Gorhom's default over-drag resistance, after either travel limit.
const RESISTANCE: f32 = 2.5;

#[derive(Debug)]
struct Sample {
    y: f32,
    at: Instant,
}

#[derive(Debug)]
pub(super) struct Drag {
    origin: Origin,
    top: f32,
    start: Point<Pixels>,
    start_position: f32,
    start_scroll: f32,
    scroll: f32,
    moved: bool,
    samples: VecDeque<Sample>,
}

impl Drag {
    pub(super) fn new(
        origin: Origin,
        point: Point<Pixels>,
        position: f32,
        scroll: f32,
        now: Instant,
    ) -> Self {
        let mut samples = VecDeque::with_capacity(HISTORY);
        samples.push_back(Sample {
            y: f32::from(point.y),
            at: now,
        });
        Self {
            origin,
            top: Stop::Expanded.position(),
            start: point,
            start_position: position,
            start_scroll: scroll,
            scroll,
            moved: false,
            samples,
        }
    }

    pub(super) fn with_points(
        origin: Origin,
        point: Point<Pixels>,
        position: f32,
        scroll: f32,
        now: Instant,
        points: impl Into<Positions>,
    ) -> Self {
        Self {
            top: Stop::Expanded.position_in(points),
            ..Self::new(origin, point, position, scroll, now)
        }
    }

    pub(super) const fn moved(&self) -> bool {
        self.moved
    }

    pub(super) fn scrolled(&self) -> bool {
        self.origin == Origin::Content && self.scroll > 0.0
    }

    pub(super) fn pan(
        &mut self,
        point: Point<Pixels>,
        height: f32,
        maximum_scroll: f32,
        current_position: f32,
        now: Instant,
    ) -> Option<(f32, f32)> {
        self.samples.push_back(Sample {
            y: f32::from(point.y),
            at: now,
        });
        while self.samples.len() > HISTORY {
            if self.samples.pop_front().is_none() {
                break;
            }
        }
        if !self.moved {
            if f32::from(point.x)
                .mul_add(-1.0, f32::from(self.start.x))
                .hypot(f32::from(point.y).mul_add(-1.0, f32::from(self.start.y)))
                <= POINTER_SLOP
            {
                return None;
            }
            // Until native pointer slop is crossed, an opening animation
            // may continue. Grab its current location without jumping back.
            self.start_position = current_position;
            self.moved = true;
        }
        let translation = f32::from(self.start.y).mul_add(-1.0, f32::from(point.y));
        let initial = self.start_position.mul_add(height, 0.0);
        let top = height.mul_add(self.top, 0.0);
        let mut position = initial.mul_add(1.0, translation);
        self.scroll = self.start_scroll;
        if self.origin == Origin::Content {
            if translation < 0.0 {
                // Reach the top open stop first (default90%); only extra
                // travel scrolls rows. A fitted sheet already starts there.
                self.scroll = self
                    .start_scroll
                    .mul_add(1.0, position.mul_add(-1.0, top).max(0.0))
                    .clamp(0.0, maximum_scroll);
                position = position.max(top);
            } else {
                // Pull rows back to their top before collapsing the sheet.
                self.scroll = translation.mul_add(-1.0, self.start_scroll).max(0.0);
                position =
                    initial.mul_add(1.0, self.start_scroll.mul_add(-1.0, translation).max(0.0));
            }
        }
        let position = if position < top {
            position
                .mul_add(-1.0, top.mul_add(1.0, 1.0))
                .sqrt()
                .mul_add(RESISTANCE.mul_add(-1.0, 0.0), top)
        } else if position > height {
            height
                .mul_add(-1.0, position.mul_add(1.0, 1.0))
                .sqrt()
                .mul_add(RESISTANCE, height)
        } else {
            position
        };
        Some((position.mul_add(height.recip(), 0.0), self.scroll))
    }

    pub(super) fn velocity(&self, now: Instant) -> f32 {
        let mut recent = self
            .samples
            .iter()
            .filter(|sample| now.saturating_duration_since(sample.at) <= VELOCITY_WINDOW);
        let Some(first) = recent.next() else {
            return 0.0;
        };
        let last = recent.next_back().unwrap_or(first);
        let seconds = last.at.saturating_duration_since(first.at).as_secs_f32();
        if seconds <= f32::EPSILON {
            return 0.0;
        }
        let speed = first.y.mul_add(-1.0, last.y).mul_add(seconds.recip(), 0.0);
        if speed.is_finite() { speed } else { 0.0 }
    }
}
