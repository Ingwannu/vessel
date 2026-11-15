//! The motion tokens: every transition's length and curve, read from one
//! place (the owner, 2026-11-14, word 39: quick, fine transitions on every
//! change a person causes, with Animations off giving the end state at
//! once; nothing animating continuously).
//!
//! [Decision Log]
//! - Purpose: one set of values for every track (the kit's controls, the
//!   shell's surfaces, the composer, the conversation), so a change of
//!   pace is one edit.
//! - Constraint: the kit and `view` do not depend on each other; the app
//!   reads both. Transitions are drawn with GPUI (`with_animation`) in the
//!   kit and the app, so the tokens live here. The effort slider's model
//!   in `view` (`slider::motion`) restates the same length and curve and
//!   is pinned to these by the app's test `motion_tokens_tests`.
//! - Values: the Codex app's (`150 ms` for a control's own change,
//!   `300 ms` for a surface, `cubic-bezier(.23, 1, .32, 1)`), the start
//!   values the coordinator named.
//! - Held motion: the app keeps one flag for every reason motion is held
//!   (the system's reduced motion, the Animations setting off, a software
//!   renderer; `app::motion`), GPUI's `App::reduce_motion`. GPUI's
//!   `with_animation` draws a one-shot transition's end at once under it;
//!   motion driven by hand (a timer, `request_animation_frame`) asks
//!   [`held`] and jumps to its end.
//! - Rule: only one-shot transitions; nothing repeats while the screen is
//!   still (each frame is the whole window redrawn).

use std::time::Duration;

use gpui::{Animation, App};

/// A control's own change: a hover fill, a press, a toggle, a selection
/// moving, a row opening.
pub const QUICK: Duration = Duration::from_millis(150);

/// A surface arriving or leaving: a menu, a popover, a dialog, a panel, a
/// page, a toast, a message.
pub const STANDARD: Duration = Duration::from_millis(300);

/// The curve's control points: `cubic-bezier(.23, 1, .32, 1)`, fast then
/// settling.
pub const CURVE: [f32; 4] = [0.23, 1.0, 0.32, 1.0];

/// Progress along [`CURVE`] at `t` (0 to 1) of the time; for
/// `Animation::with_easing` and hand-driven motion.
#[must_use]
pub fn ease(t: f32) -> f32 {
    let [x1, y1, x2, y2] = CURVE;
    bezier(x1, y1, x2, y2, t)
}

/// A one-shot transition of `length` along [`CURVE`] (GPUI draws its end
/// at once while motion is held).
#[must_use]
pub fn transition(length: Duration) -> Animation {
    Animation::new(length).with_easing(ease)
}

/// Whether motion is held: hand-driven motion jumps to its end.
#[must_use]
pub fn held(cx: &App) -> bool {
    cx.reduce_motion()
}

/// A CSS cubic Bézier from (0, 0) to (1, 1): the curve's x solved for `t`
/// by Newton's steps (bisection where the slope is flat), its y returned.
#[expect(
    clippy::float_arithmetic,
    reason = "a curve's polynomial; the values stay within 0 to 1"
)]
fn bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    let along = |a: f32, b: f32, s: f32| {
        let u = 1.0 - s;
        3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
    };
    let slope = |a: f32, b: f32, s: f32| {
        let u = 1.0 - s;
        3.0 * u * u * a + 6.0 * u * s * (b - a) + 3.0 * s * s * (1.0 - b)
    };
    let t = t.clamp(0.0, 1.0);
    let mut s = t;
    for _ in 0..8 {
        let off = along(x1, x2, s) - t;
        if off.abs() < 1e-5 {
            return along(y1, y2, s);
        }
        let d = slope(x1, x2, s);
        if d.abs() < 1e-6 {
            break;
        }
        s = (s - off / d).clamp(0.0, 1.0);
    }
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    s = t;
    for _ in 0..24 {
        if along(x1, x2, s) < t {
            low = s;
        } else {
            high = s;
        }
        s = f32::midpoint(low, high);
    }
    along(y1, y2, s)
}

#[cfg(test)]
mod tests {
    use super::ease;

    /// The curve starts at 0, ends at 1, never goes back, and is far along
    /// early (fast then settling).
    #[test]
    fn the_curve_is_fast_then_settles() {
        assert!(ease(0.0).abs() < 1e-4);
        assert!((ease(1.0) - 1.0).abs() < 1e-4);
        let mut last = 0.0;
        for step in 1..=100_u8 {
            let value = ease(f32::from(step) / 100.0);
            assert!(value + 1e-5 >= last, "the curve never goes back");
            last = value;
        }
        assert!(ease(0.25) > 0.6, "most of the way in the first quarter");
    }
}
