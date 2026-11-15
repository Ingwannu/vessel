//! Bottom-sheet stops, pointer/scroll routing and reference motion.
//! Existing edge sheets keep their API; a host opts into this state.
//!
//! [Decision Log]
//! - Purpose: authored and natural-fit sheets use the same default60/90/closed
//!   engine or an explicit measured open position,
//!   projected-velocity release and spring, rather than area gestures.
//! - Constraint: GPUI exposes pointer positions but no gesture velocity;
//!   pointer samples supply it. The host owns hit boxes and focus.
//! - Choice: state sampled at frame time, with explicit scroll offsets
//!   returned from content pans. Reduced motion settles springs at once
//!   while direct manipulation still follows the pointer.
//! - Cost: native pointer slop and velocity sampling replace the
//!   reference's platform gesture recognizer; the snap rule is shared.

mod drag;
pub mod listener;
pub(crate) mod motion;
mod points;
mod positions;

#[cfg(test)]
mod configuration_tests;
#[cfg(test)]
mod fit_tests;
#[cfg(test)]
mod tests;

pub use motion::Stop;
pub use points::{Percentage, SnapPoints};
use positions::Positions;

use std::time::Instant;

use gpui::{Pixels, Point, px};

use drag::Drag;
use motion::{Spring, SpringValue, destination_in};

/// Result of measuring a natural-content sheet, not an owner close decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// No finite positive child/viewport height yet; retain current state.
    PendingLayout,
    /// The existing spring now targets this newly measured single open height.
    Updated,
    /// Same target; an active pan or spring is left untouched.
    Unchanged,
}

/// Where a pointer gesture began.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// The handle; it always moves the sheet.
    Handle,
    /// Scrollable contents; their offset participates in routing.
    Content,
}

/// Whether the visible sheet needs another animation frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Movement {
    /// At a stop.
    Resting,
    /// Settling toward a stop.
    Animating,
    /// Directly following a pointer.
    Dragging,
}

/// The visible frame, in the host window's coordinates.
#[derive(Debug, Clone, Copy)]
pub struct Geometry {
    /// The panel's top edge.
    pub top: Pixels,
    /// The panel's height, including its handle.
    pub height: Pixels,
    /// Backdrop coverage: zero when closed, one at the initial open height and above.
    pub backdrop: f32,
    /// Its movement state.
    pub movement: Movement,
}

/// A layout result distinguishes a dismissed sheet from a zero-size window.
#[derive(Debug, Clone, Copy)]
pub enum Frame {
    /// No valid viewport yet; retain the owner's open state.
    PendingLayout,
    /// Dismissal has completed.
    Closed,
    /// Draw these coordinates.
    Visible(Geometry),
}

/// A pointer release either remains a click or consumes the gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    /// No vertical pan began, so the ordinary click may proceed.
    Tap,
    /// The pan was consumed and now settles at this stop.
    Pan(Stop),
}

#[derive(Debug, Clone, Copy)]
enum Pose {
    At(Stop),
    Settling {
        last: Instant,
        elapsed: std::time::Duration,
        spring: Spring,
    },
    Held(f32),
}

/// Snap-sheet state, independent of the owning area and its actions.
#[derive(Debug)]
pub struct State {
    pose: Pose,
    drag: Option<Drag>,
    height: f32,
    points: Positions,
}

impl Default for State {
    fn default() -> Self {
        Self {
            pose: Pose::At(Stop::Closed),
            drag: None,
            height: 0.0,
            points: Positions::default(),
        }
    }
}

impl State {
    /// The same gesture/spring engine with two validated authored open heights.
    #[must_use]
    pub fn with_points(points: SnapPoints) -> Self {
        Self {
            points: points.into(),
            ..Self::default()
        }
    }

    /// Retarget this same spring/gesture engine to one measured open height.
    /// The full child height includes its handle/header. Invalid measurements
    /// retain the previous frame; authored two-stop configuration is unchanged.
    #[must_use]
    pub fn fit_content(
        &mut self,
        content: Pixels,
        viewport: Pixels,
        cap: Percentage,
        now: Instant,
    ) -> Fit {
        let Some(points) = Positions::fit(content, viewport, cap) else {
            return Fit::PendingLayout;
        };
        if self.points == points {
            return Fit::Unchanged;
        }
        let current = self.position(now, false);
        let position = current.position_in(self.points);
        self.points = points;
        self.drag = None;
        self.settle(position, current.velocity(), Stop::Compact, now);
        Fit::Updated
    }

    /// A refused close keeps its existing control at the initial open stop.
    pub(crate) fn retain_compact(&mut self) {
        self.drag = None;
        self.pose = Pose::At(Stop::Compact);
    }
    /// Present afresh at the configured initial stop (60% by default), from below.
    pub fn present(&mut self, now: Instant) {
        self.drag = None;
        self.pose = Pose::Settling {
            last: now,
            elapsed: std::time::Duration::ZERO,
            spring: Spring::with_points(1.0, 0.0, Stop::Compact, self.points),
        };
    }

    /// Start closing from the current visual position, cancelling a pan.
    pub fn close(&mut self, now: Instant) {
        let current = self.position(now, false);
        self.drag = None;
        self.settle(
            current.position_in(self.points),
            current.velocity(),
            Stop::Closed,
            now,
        );
    }

    /// Sample a frame. A host requests another frame only for Animating.
    #[must_use]
    pub fn frame(&mut self, height: Pixels, now: Instant, reduced: bool) -> Frame {
        let height = f32::from(height);
        if !height.is_finite() || height <= 0.0 {
            return Frame::PendingLayout;
        }
        self.height = height;
        let sampled = self.position(now, reduced);
        let movement = match sampled {
            SpringValue::Resting(stop) => {
                self.pose = Pose::At(stop);
                if stop == Stop::Closed {
                    return Frame::Closed;
                }
                Movement::Resting
            }
            SpringValue::Moving { .. } if self.drag.as_ref().is_some_and(Drag::moved) => {
                Movement::Dragging
            }
            SpringValue::Moving { .. } => Movement::Animating,
        };
        let position = sampled.position_in(self.points);
        Frame::Visible(Geometry {
            top: px(position.mul_add(height, 0.0)),
            height: px(position.mul_add(-1.0, 1.0).mul_add(height, 0.0).max(0.0)),
            backdrop: position
                .mul_add(-1.0, 1.0)
                .mul_add(self.points.compact_fraction().recip(), 0.0)
                .clamp(0.0, 1.0),
            movement,
        })
    }

    /// Record a potential pan. A stationary press does not stop motion
    /// or take the eventual click from the row under it.
    pub fn press(&mut self, origin: Origin, point: Point<Pixels>, scroll: Pixels, now: Instant) {
        if self.height <= 0.0 {
            return;
        }
        let position = self.position(now, false).position_in(self.points);
        self.drag = Some(Drag::with_points(
            origin,
            point,
            position,
            f32::from(scroll).max(0.0),
            now,
            self.points,
        ));
    }

    /// Move a pointer. Some(offset) consumes the pan and gives the
    /// scrollable's positive distance from its top; None leaves the click.
    pub fn pan(
        &mut self,
        point: Point<Pixels>,
        maximum_scroll: Pixels,
        now: Instant,
    ) -> Option<Pixels> {
        let current_position = self.position(now, false).position_in(self.points);
        let drag = self.drag.as_mut()?;
        let (position, scroll) = drag.pan(
            point,
            self.height,
            f32::from(maximum_scroll).max(0.0),
            current_position,
            now,
        )?;
        self.pose = Pose::Held(position);
        Some(px(scroll))
    }

    /// Finish a pan using the reference's 200 ms velocity projection.
    #[must_use]
    pub fn release(&mut self, now: Instant) -> Release {
        let Some(drag) = self.drag.take() else {
            return Release::Tap;
        };
        if !drag.moved() {
            return Release::Tap;
        }
        let position = self.position(now, false).position_in(self.points);
        // Content that has scrolled at the expanded stop must not snap
        // back down on release (Gorhom's interrupted-scroll guard).
        let velocity = drag.velocity(now).mul_add(self.height.recip(), 0.0);
        let stop = if drag.scrolled()
            && position
                <= Stop::Expanded
                    .position_in(self.points)
                    .mul_add(1.0, f32::EPSILON)
        {
            Stop::Expanded
        } else {
            destination_in(position, velocity, self.points)
        };
        self.settle(position, velocity.mul_add(0.5, 0.0), stop, now);
        Release::Pan(stop)
    }

    fn settle(&mut self, from: f32, velocity: f32, to: Stop, now: Instant) {
        self.pose = Pose::Settling {
            last: now,
            elapsed: std::time::Duration::ZERO,
            spring: Spring::with_points(from, velocity, to, self.points),
        };
    }

    fn position(&mut self, now: Instant, reduced: bool) -> SpringValue {
        match self.pose {
            Pose::At(stop) => SpringValue::Resting(stop),
            Pose::Settling {
                last,
                elapsed,
                spring,
            } => {
                // Reanimated caps an animation step at64ms after a delayed
                // frame rather than jumping an entire suspended interval.
                let step = now
                    .saturating_duration_since(last)
                    .min(std::time::Duration::from_millis(64));
                let elapsed = elapsed.saturating_add(step);
                self.pose = Pose::Settling {
                    last: now,
                    elapsed,
                    spring,
                };
                spring.sample(elapsed, self.height, reduced)
            }
            Pose::Held(position) => SpringValue::Moving {
                position,
                velocity: 0.0,
            },
        }
    }
}
