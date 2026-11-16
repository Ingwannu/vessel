//! Reference motion math for a shared bottom sheet and a fading modal.
//!
//! [Decision Log]
//! - Purpose: geometry, pointer velocity and animation completion share one
//!   definition, independent of the renderer or which area owns the sheet.
//! - Reference: Gorhom snapPoint/default spring; RNW `ModalAnimation` fade.
//! - Choice: normalized vertical positions survive viewport resizing. The
//!   spring follows Reanimated's critically-damped branch for zeta >= 1,
//!   with its relative-energy termination. Gorhom's legacy pixel-rest
//!   properties are not consumed by the installed Reanimated version.
//! - Cost: the caller supplies elapsed time and height and owns pointer
//!   routing. This module cannot steal focus or run a screen's action.

use super::positions::Positions;
use std::time::Duration;

/// A sheet's available stops, in the reference's tie-breaking order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// Below the viewport, fully dismissed.
    Closed,
    /// Initial authored height, or the measured content-fit open position.
    Compact,
    /// Expanded authored height; aliases the same open position for content fit.
    Expanded,
}

impl Stop {
    pub(crate) const ALL: [Self; 3] = [Self::Closed, Self::Compact, Self::Expanded];

    pub(crate) fn position_in(self, points: impl Into<Positions>) -> f32 {
        points.into().position(self)
    }

    pub(crate) const fn position(self) -> f32 {
        match self {
            Self::Closed => 1.0,
            Self::Compact => 0.4,
            Self::Expanded => 0.1,
        }
    }
}

/// Closest reference stop to the projected point (200 ms of velocity).
/// Equal distances retain reference order, including Closed before 60%.
pub(crate) fn destination(position: f32, velocity: f32) -> Stop {
    nearest(position, velocity, Positions::default())
}

pub(crate) fn destination_in(position: f32, velocity: f32, points: impl Into<Positions>) -> Stop {
    let points = points.into();
    if points == Positions::default() {
        destination(position, velocity)
    } else {
        nearest(position, velocity, points)
    }
}

fn nearest(position: f32, velocity: f32, points: Positions) -> Stop {
    let projected = velocity.mul_add(0.2, position);
    let mut chosen = Stop::Closed;
    let mut distance = chosen.position_in(points).mul_add(-1.0, projected).abs();
    for stop in Stop::ALL {
        let next = stop.position_in(points).mul_add(-1.0, projected).abs();
        if next < distance {
            chosen = stop;
            distance = next;
        }
    }
    chosen
}

/// A spring's sampled state: completion has exactly zero velocity.
#[derive(Debug, Clone, Copy)]
pub(crate) enum SpringValue {
    Moving { position: f32, velocity: f32 },
    Resting(Stop),
}

impl SpringValue {
    pub(crate) fn position_in(self, points: impl Into<Positions>) -> f32 {
        let points = points.into();
        if points == Positions::default() {
            return self.position();
        }
        match self {
            Self::Moving { position, .. } => position,
            Self::Resting(stop) => stop.position_in(points),
        }
    }
    pub(crate) const fn position(self) -> f32 {
        match self {
            Self::Moving { position, .. } => position,
            Self::Resting(stop) => stop.position(),
        }
    }

    pub(crate) const fn velocity(self) -> f32 {
        match self {
            Self::Moving { velocity, .. } => velocity,
            Self::Resting(_) => 0.0,
        }
    }
}

/// Gorhom's damping500/stiffness1000/mass3 reaches Reanimated's zeta>=1
/// branch. Use that branch, its overshoot clamp and relative-energy limit.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Spring {
    from: f32,
    velocity: f32,
    to: Stop,
    points: Positions,
}

impl Spring {
    pub(crate) const fn new(from: f32, velocity: f32, to: Stop) -> Self {
        Self {
            from,
            velocity,
            to,
            points: Positions::DEFAULT,
        }
    }

    pub(crate) fn with_points(
        from: f32,
        velocity: f32,
        to: Stop,
        points: impl Into<Positions>,
    ) -> Self {
        Self {
            points: points.into(),
            ..Self::new(from, velocity, to)
        }
    }

    pub(super) const fn target(self) -> Stop {
        self.to
    }

    pub(crate) fn sample(self, elapsed: Duration, height: f32, reduced: bool) -> SpringValue {
        if reduced || !height.is_finite() || height <= 0.0 {
            return SpringValue::Resting(self.to);
        }
        let target = self.to.position_in(self.points);
        let displacement = target.mul_add(-1.0, self.from);
        // Reanimated clips inherited velocity that initially moves away
        // from the new target. Its initial energy still uses the supplied
        // velocity, as spring.ts's onStart does.
        let supplied = self.velocity;
        let initial_velocity = if displacement.mul_add(supplied, 0.0) > 0.0 {
            0.0
        } else {
            supplied
        };
        let frequency = 1000.0_f32.mul_add(3.0_f32.recip(), 0.0).sqrt();
        let seconds = elapsed.as_secs_f32();
        let envelope = frequency.mul_add(seconds.mul_add(-1.0, 0.0), 0.0).exp();
        let slope = frequency.mul_add(displacement, initial_velocity);
        let offset = envelope.mul_add(slope.mul_add(seconds, displacement), 0.0);
        let velocity = envelope.mul_add(
            frequency.mul_add(
                slope.mul_add(seconds, displacement).mul_add(-1.0, 0.0),
                slope,
            ),
            0.0,
        );
        let overshot = offset < displacement.min(0.0) || offset > displacement.max(0.0);
        let initial_energy = energy(displacement, supplied);
        let at_rest = initial_energy <= f32::MIN_POSITIVE
            || energy(offset, velocity).mul_add(initial_energy.recip(), 0.0) <= 6.0e-9;
        if overshot || at_rest {
            SpringValue::Resting(self.to)
        } else {
            SpringValue::Moving {
                position: offset.mul_add(1.0, target),
                velocity,
            }
        }
    }
}

fn energy(displacement: f32, velocity: f32) -> f32 {
    displacement
        .mul_add(displacement, 0.0)
        .mul_add(500.0, velocity.mul_add(velocity, 0.0).mul_add(1.5, 0.0))
}

/// The desktop palette's entering fade: the kit's surface length (word
/// 39's tokens; RNW's `ModalAnimation` was 250 ms).
pub(crate) const FADE: Duration = crate::motion::STANDARD;

/// Command Center mounts a visible Modal and unmounts it at close, so
/// only the entering curve runs there (command-center.tsx945–949).
pub(crate) fn fade_in(elapsed: Duration, reduced: bool) -> f32 {
    if reduced || elapsed >= FADE {
        return 1.0;
    }
    let progress = elapsed
        .as_secs_f32()
        .mul_add(FADE.as_secs_f32().recip(), 0.0);
    crate::motion::ease(progress)
}
