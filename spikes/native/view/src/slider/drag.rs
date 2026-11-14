//! The thumb under a dragging pointer (the coordinator, 2026-11-14, from
//! the owner's words 23 and 41-42: "move smoothly and stick to an effort";
//! on the owner's screen 0.0.0.0.14's stepped glide still read as not
//! smooth).
//!
//! [Decision Log]
//! - Purpose: while the button is down the thumb follows the pointer
//!   frame by frame; near a stop it is pulled onto the stop and stays there
//!   until the pointer has gone a little way off (a detent); the effort
//!   chosen is the stop whose reach the thumb is in; on release the thumb
//!   springs onto that stop.
//! - Difference from the Codex app, named: its thumb only glides from stop
//!   to stop (150 ms) while dragged. A click on the track, the keys and the
//!   wheel keep its 300 ms glide.
//! - The pull ([`magnet`]): within [`STICK`] points of a stop the thumb
//!   stays on it; past that its distance from the stop rises along a power
//!   curve and meets the pointer at the midpoint between two stops. So the
//!   thumb lags the pointer leaving a stop and leads it arriving at the
//!   next, and the curve is continuous at the midpoint, where the nearest
//!   stop changes: the thumb never jumps.
//! - Catching up: a drag that begins away from the thumb (a press on the
//!   track, then a move) starts from where the thumb is and closes the gap
//!   with a [`CATCH`] time constant, not with a jump.
//! - The release: a critically damped spring ([`RELEASE`]: the Codex app's
//!   press stiffness, 420, with damping 2√420 instead of its 38), so it
//!   comes to rest on the stop without overshoot.
//! - Alternatives: the stepped glide kept and only shortened (the owner
//!   saw it as steps); a follow without the pull (no "sticking" to an
//!   effort); a spring chasing the pointer (lags the hand by the spring's
//!   settle time on every move).
//! - Under reduced motion there is no follow: the thumb is on the chosen
//!   stop.
#![expect(
    clippy::float_arithmetic,
    reason = "positions along the track are float formulas"
)]

use super::motion::SpringParams;
use super::{INSET, float, fraction};

/// The sticky zone around a stop, points: the thumb stays on the stop
/// until the pointer is this far from it.
pub const STICK: f32 = 8.0;
/// The share of a stop's reach the sticky zone may take on a short track.
const STICK_SHARE: f32 = 0.4;
/// The pull's curve past the sticky zone (1: no pull).
const PULL: f32 = 1.6;
/// How far the pointer moves, points, before a press becomes a drag.
pub const DRAG_START: f32 = 3.0;
/// The time constant the thumb closes the gap to the pointer with when a
/// drag begins away from it, seconds.
pub const CATCH: f32 = 0.06;
/// The release onto the stop: critically damped (2√420 ≈ 40.988).
pub const RELEASE: SpringParams = SpringParams {
    stiffness: 420.0,
    damping: 40.987_8,
    mass: 1.0,
};

/// A pointer over the track: points from the track's start, and the
/// track's width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pointer {
    /// Points from the track's start.
    pub x: f32,
    /// The track's width.
    pub width: f32,
}

impl Pointer {
    /// The line between the first and last stop, points.
    #[must_use]
    pub fn line(self) -> f32 {
        INSET.mul_add(-2.0, self.width).max(0.0)
    }

    /// The pointer as a share of the way from the first stop to the last,
    /// 0 to 1.
    #[must_use]
    pub fn fraction(self) -> f32 {
        let line = self.line();
        if line <= 0.0 {
            return 0.0;
        }
        ((self.x - INSET) / line).clamp(0.0, 1.0)
    }
}

/// The stop nearest `fraction` of the way, of `stops`.
#[must_use]
pub fn nearest(fraction: f32, stops: usize) -> usize {
    let last = stops.saturating_sub(1);
    let at = (fraction.clamp(0.0, 1.0) * float(last)).round();
    (0..=last).find(|index| float(*index) >= at).unwrap_or(last)
}

/// Where the thumb is drawn, as a share of the way, for a pointer at
/// `pointer` of the way, with `stops` stops on a line `line` points long.
#[must_use]
pub fn magnet(pointer: f32, stops: usize, line: f32) -> f32 {
    let pointer = pointer.clamp(0.0, 1.0);
    let last = stops.saturating_sub(1);
    if last == 0 || line <= 0.0 {
        return pointer;
    }
    let reach = line / float(last) / 2.0;
    let stick = STICK.min(reach * STICK_SHARE);
    let stop = fraction(nearest(pointer, stops), stops);
    let off = (pointer - stop) * line;
    let distance = off.abs();
    let drawn = if distance <= stick {
        0.0
    } else {
        reach
            * ((distance - stick) / (reach - stick))
                .clamp(0.0, 1.0)
                .powf(PULL)
    };
    stop + drawn.copysign(off) / line
}
