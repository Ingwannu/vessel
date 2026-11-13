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

use std::time::Duration;

/// A sheet's available stops, in the reference's tie-breaking order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// Below the viewport, fully dismissed.
    Closed,
    /// The initial 60% height.
    Compact,
    /// The expanded 90% height.
    Expanded,
}

impl Stop {
    pub(crate) const ALL: [Self; 3] = [Self::Closed, Self::Compact, Self::Expanded];

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
    let projected = velocity.mul_add(0.2, position);
    let mut chosen = Stop::Closed;
    let mut distance = chosen.position().mul_add(-1.0, projected).abs();
    for stop in Stop::ALL {
        let next = stop.position().mul_add(-1.0, projected).abs();
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
}

impl Spring {
    pub(crate) const fn new(from: f32, velocity: f32, to: Stop) -> Self {
        Self { from, velocity, to }
    }

    pub(crate) fn sample(self, elapsed: Duration, height: f32, reduced: bool) -> SpringValue {
        if reduced || !height.is_finite() || height <= 0.0 {
            return SpringValue::Resting(self.to);
        }
        let target = self.to.position();
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

/// RNW's `ModalAnimation` duration, shared by both fade directions.
pub(crate) const FADE: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fade {
    In,
    Out,
}

impl Fade {
    /// Interpolation, including the reference's distinct enter/exit curves.
    pub(crate) fn progress(self, elapsed: Duration, reduced: bool) -> f32 {
        if reduced || elapsed >= FADE {
            return 1.0;
        }
        let progress = elapsed
            .as_secs_f32()
            .mul_add(FADE.as_secs_f32().recip(), 0.0);
        match self {
            Self::In => bezier(progress, (0.215, 0.61), (0.355, 1.0)),
            Self::Out => bezier(progress, (0.47, 0.0), (0.745, 0.715)),
        }
    }
}

/// Invert the curve's time axis before evaluating opacity. Using a
/// polynomial's parameter as time would not match a CSS cubic-bezier.
fn bezier(time: f32, first: (f32, f32), second: (f32, f32)) -> f32 {
    if time <= 0.0 {
        return 0.0;
    }
    if time >= 1.0 {
        return 1.0;
    }
    // De Casteljau interpolation avoids an expanded polynomial and
    // keeps every intermediate within the control points' unit interval.
    let lerp =
        |from: f32, to: f32, at: f32| to.mul_add(at, from.mul_add(at.mul_add(-1.0, 0.0), from));
    let coordinate = |at: f32, one: f32, two: f32| {
        let a = one.mul_add(at, 0.0);
        let b = lerp(one, two, at);
        let c = lerp(two, 1.0, at);
        lerp(lerp(a, b, at), lerp(b, c, at), at)
    };
    let mut lower = 0.0;
    let mut upper = 1.0;
    for _ in 0..20 {
        let at = f32::midpoint(lower, upper);
        if coordinate(at, first.0, second.0) < time {
            lower = at;
        } else {
            upper = at;
        }
    }
    coordinate(f32::midpoint(lower, upper), first.1, second.1)
}
