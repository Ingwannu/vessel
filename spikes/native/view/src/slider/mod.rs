//! The effort slider's logic, as the local Codex app's model card has it
//! (the owner, 2026-11-14: "learn the local Codex app's slider"; the
//! values were read from the app's own stylesheet and script, recorded in
//! `docs/plans/backend-m12-effort-slider.md`). The kit draws it; what a
//! position means, how it moves and what decorates it are decided here,
//! without GPUI, under test.
//!
//! [Decision Log]
//! - Purpose: the owner found the slider low quality and jumpy and named
//!   the Codex app as the reference for its look and feel.
//! - What the reference does: a stepped slider. The pointer maps over the
//!   whole control, `round(x / width * (stops - 1))`, so the stop changes
//!   as the pointer crosses a midpoint; the thumb and the fill glide to
//!   each new stop (150 ms while dragging, 300 ms otherwise, one easing);
//!   the thumb grows on hover and press with a spring. There is no
//!   continuous follow and no pull near a stop.
//! - Since 0.0.0.0.15 a drag differs ([`drag`], the owner's words 41-42:
//!   the stepped glide read as not smooth): the thumb follows the pointer
//!   with a pull onto each stop and springs onto the stop on release; the
//!   stop under a pointer is the nearest one along the line between the
//!   end stops (where the thumb sits), not the share of the whole
//!   control. Clicks, keys and the wheel keep the reference's glide.
//! - Alternatives: GPUI's `Animation` (a fixed duration from a start, no
//!   way to retarget mid-flight from the value it has reached).
//! - Choice: the motion is state the kit keeps and steps on the frame
//!   clock ([`motion`]): a tween restarted from where it is, a spring
//!   integrated per frame. Geometry and the commit rule are here; the top
//!   stop's field, burst and the fast dots are in [`flair`].
//! - Cost: the kit must ask for frames while something moves and stop
//!   asking when it rests (measured on the card closed: no frames).

pub mod drag;
pub mod flair;
pub mod motion;
pub mod state;
#[cfg(test)]
mod tests;

/// The track's height.
pub const TRACK: f32 = 24.0;
/// The control's height (the pointer's target).
pub const CONTROL: f32 = 28.0;
/// The block the control sits in.
pub const BLOCK: f32 = 32.0;
/// The block's side padding.
pub const BLOCK_SIDE: f32 = 6.0;
/// The thumb at rest.
pub const THUMB: f32 = 28.0;
/// The thumb hovered or pressed.
pub const THUMB_GROWN: f32 = 32.0;
/// The first and last stop's distance from the track's ends (half the
/// thumb, less one).
pub const INSET: f32 = 13.0;
/// A stop's dot.
pub const DOT: f32 = 4.0;
/// A hovered stop's scale.
pub const DOT_HOVER: f32 = 2.0;

/// `count` as a float; the slider never has more stops than a `u16`.
#[must_use]
pub fn float(count: usize) -> f32 {
    f32::from(u16::try_from(count).unwrap_or(u16::MAX))
}

/// Stop `index` of `stops` as a fraction of the way, 0 to 1.
#[must_use]
pub fn fraction(index: usize, stops: usize) -> f32 {
    let last = stops.saturating_sub(1);
    if last == 0 {
        return 0.0;
    }
    float(index.min(last)).mul_add(float(last).recip(), 0.0)
}

/// Where along a track `width` wide a point at `fraction` of the way is
/// drawn: the stops sit between the two insets.
#[must_use]
pub fn along(fraction: f32, width: f32) -> f32 {
    let line = INSET.mul_add(-2.0, width).max(0.0);
    fraction.mul_add(line, INSET)
}

/// Whether a drag that began on `pressed` and ended on `released` sends a
/// choice: only a change does.
#[must_use]
pub fn commits(pressed: Option<usize>, released: usize) -> bool {
    pressed != Some(released)
}
