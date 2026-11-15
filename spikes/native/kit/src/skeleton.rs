//! Neutral loading shapes, after Paseo's `components/ui/skeleton.tsx`.
//!
//! [Decision Log]
//! - Purpose: provider, model and other loading rows share one pulse/skin.
//! - Constraint: only shapes fade; a card's surface and border stay solid.
//! - Alternatives: animate each shape independently (phases can diverge),
//!   or fade their parent (also fades its furniture).
//! - Choice: one mounted GPUI animation builds all shapes with one typed
//!   opacity. The caller composes its existing row/card geometry.
//! - Why: the shared phase starts at mount, runs 1 s each way from 0.4 to
//!   0.8, and schedules frames only while drawn. No detached timer lives on.
//! - Cost: loading contents are rebuilt for an animation frame, like other
//!   frame-driven kit controls. Reduced motion follows GPUI's shared policy.
use crate::Tokens;
use gpui::{
    Animation, AnimationExt as _, AnyElement, Div, ElementId, IntoElement as _, Pixels, Size, div,
    prelude::*, px,
};
use std::time::Duration;

/// Paseo's small and medium corners used by neutral loading shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rounding {
    /// `borderRadius.sm`: 2 points.
    Small,
    /// `borderRadius.md`: 6 points.
    Medium,
}
impl Rounding {
    const fn radius(self) -> Pixels {
        match self {
            Self::Small => px(2.0),
            Self::Medium => px(6.0),
        }
    }
}
/// A shape's caller-owned size; its fill and opacity stay shared.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    /// Width and height in logical points.
    pub size: Size<Pixels>,
    /// The reference's corner step.
    pub rounding: Rounding,
}
/// One frame's shared alpha. Only the pulse constructs it, within 0.4–0.8.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Opacity(f32);
impl Opacity {
    fn from_eased(progress: f32) -> Self {
        Self(progress.mul_add(0.4, 0.4))
    }
}
/// A neutral shape; every shape in one skeleton receives the same opacity.
#[must_use]
pub fn shape(geometry: Shape, opacity: Opacity, tokens: &Tokens) -> Div {
    div()
        .w(geometry.size.width)
        .h(geometry.size.height)
        .flex_shrink_0()
        .rounded(geometry.rounding.radius())
        .bg(tokens.neutral.surface2)
        .opacity(opacity.0)
}

struct Pulse<F> {
    draw: F,
    opacity: Opacity,
}
impl<F: Fn(Opacity) -> Div + 'static> IntoElement for Pulse<F> {
    type Element = Div;
    fn into_element(self) -> Self::Element {
        (self.draw)(self.opacity)
    }
}
/// One phase per mounted skeleton, passed to all of its shapes. `draw` must
/// only compose loading contents; interaction belongs to the actual rows.
#[must_use]
pub fn pulse(id: impl Into<ElementId>, draw: impl Fn(Opacity) -> Div + 'static) -> AnyElement {
    animated(id, draw, motion())
}
fn motion() -> Animation {
    Animation::new(Duration::from_secs(2))
        .repeat()
        .with_easing(gpui::bounce(timing))
}
// The motion boundary lets the renderer be checked with a controlled
// phase: GPUI's unsynced animation uses a wall Instant, not its test clock.
fn animated(
    id: impl Into<ElementId>,
    draw: impl Fn(Opacity) -> Div + 'static,
    motion: Animation,
) -> AnyElement {
    Pulse {
        draw,
        opacity: Opacity::from_eased(0.0),
    }
    .with_animation(id, motion, |mut pulse, progress| {
        pulse.opacity = Opacity::from_eased(progress);
        pulse
    })
    .into_any_element()
}

/// `Animated.timing`'s default is `inOut(ease)`, where ease is the cubic
/// Bézier (0.42,0,1,1), not GPUI's quadratic `ease_in_out`. Source:
/// react-native's `TimingAnimation`/`Easing`, used by Paseo's skeleton hook.
fn timing(progress: f32) -> f32 {
    if progress < 0.5 {
        bezier(progress.mul_add(2.0, 0.0)).mul_add(0.5, 0.0)
    } else {
        bezier(progress.mul_add(-2.0, 2.0)).mul_add(-0.5, 1.0)
    }
}
fn bezier(progress: f32) -> f32 {
    if progress <= 0.0 {
        return 0.0;
    }
    if progress >= 1.0 {
        return 1.0;
    }
    let mut low = 0.0_f32;
    let mut high = 1.0_f32;
    // The x curve is monotonic. Bisection avoids an unstable derivative
    // near its flat end; 24 steps reach f32 resolution without a new dependency.
    for _ in 0..24 {
        let mid = low.mul_add(0.5, high.mul_add(0.5, 0.0));
        let x = (-0.74_f32)
            .mul_add(mid, 0.48)
            .mul_add(mid, 1.26)
            .mul_add(mid, 0.0);
        if x < progress {
            low = mid;
        } else {
            high = mid;
        }
    }
    let parameter = low.mul_add(0.5, high.mul_add(0.5, 0.0));
    (-2.0_f32)
        .mul_add(parameter, 3.0)
        .mul_add(parameter, 0.0)
        .mul_add(parameter, 0.0)
}
#[cfg(test)]
mod tests;
