//! Paseo's horizontal three-bar `VolumeMeter`, with finite transitions only.
//!
//! [Decision Log]
//! - Purpose: voice and dictation use one level renderer.
//! - Constraints: kit has no view dependency; app explicitly maps validated
//!   voice telemetry into these small drawing values. Mute must be static.
//! - Choice: reference 8-point bars/gaps, 12+30*level heights and three
//!   scale factors, foreground tokens, `QUICK` one-shot changes. No idle pulse.
//! - Difference: clamp native alpha to one and use shared motion tokens,
//!   rather than the reference's separate 70/140 ms animation timings.

use gpui::{
    AnimationExt as _, AnyElement, Div, ElementId, Hsla, IntoElement as _, div, prelude::*, px,
};

use crate::theme::spacing;
use crate::{Tokens, motion};

const BASE_HEIGHT: f32 = 12.0;
const LEVEL_HEIGHT: f32 = 30.0;
const CONTAINER_HEIGHT: f32 = 60.0;
const SCALES: [f32; 3] = [1.2, 1.05, 0.9];

/// A finite normalized drawing value, independent of any view/domain type.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Level(f32);

/// The drawing adapter refused unvalidated telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("meter level must be finite and between zero and one")]
pub struct InvalidLevel;

impl Level {
    /// Check the numeric drawing boundary; never pass raw PCM here.
    ///
    /// # Errors
    /// Non-finite and non-normalized input cannot affect geometry.
    pub fn new(raw: f32) -> Result<Self, InvalidLevel> {
        if !raw.is_finite() || !(0.0..=1.0).contains(&raw) {
            return Err(InvalidLevel);
        }
        Ok(Self(raw))
    }
}

/// Muted state has no level or activity that could animate it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    /// Three static, dim bars regardless of capture measurements.
    Muted,
    /// Sound level without server-confirmed speech.
    Listening(Level),
    /// Confirmed speech brightens the same geometry.
    Speaking(Level),
}

/// Caller-owned transition; its id changes only when the displayed state changes.
#[derive(Debug, Clone, Copy)]
pub struct Change {
    /// The previously drawn state.
    pub before: State,
    /// The new state derived from current telemetry.
    pub after: State,
}

#[derive(Debug, Clone, Copy)]
struct Geometry {
    heights: [f32; 3],
    opacity: f32,
}
impl Geometry {
    fn of(state: State) -> Self {
        let (level, opacity) = match state {
            State::Muted => (0.0, 0.3),
            State::Listening(level) => (level.0, 0.5),
            State::Speaking(level) => (level.0, level.0.mul_add(0.3, 0.9).min(1.0)),
        };
        Self {
            heights: SCALES
                .map(|scale| level.mul_add(LEVEL_HEIGHT.mul_add(scale, 0.0), BASE_HEIGHT)),
            opacity,
        }
    }
    fn between(self, other: Self, progress: f32) -> Self {
        Self {
            heights: std::array::from_fn(|index| {
                let from = self.heights.get(index).copied().unwrap_or(BASE_HEIGHT);
                let to = other.heights.get(index).copied().unwrap_or(BASE_HEIGHT);
                from.mul_add(-1.0, to).mul_add(progress, from)
            }),
            opacity: self
                .opacity
                .mul_add(-1.0, other.opacity)
                .mul_add(progress, self.opacity),
        }
    }
}

fn frame() -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(px(spacing::S2))
        .h(px(CONTAINER_HEIGHT))
}

fn lines(geometry: Geometry, foreground: Hsla) -> [Div; 3] {
    geometry.heights.map(|height| {
        div()
            .w(px(spacing::S2))
            .h(px(height))
            .flex_shrink_0()
            .rounded_full()
            .bg(foreground)
            .opacity(geometry.opacity)
    })
}

fn bars(geometry: Geometry, foreground: Hsla) -> Div {
    frame().children(lines(geometry, foreground))
}

struct Transition {
    before: Geometry,
    after: Geometry,
    progress: f32,
    foreground: Hsla,
}
impl IntoElement for Transition {
    type Element = Div;
    fn into_element(self) -> Div {
        bars(
            self.before.between(self.after, self.progress),
            self.foreground,
        )
    }
}

/// Draw a horizontal voice meter. Muted/unchanged states schedule no animation.
#[must_use]
pub fn voice_meter(id: impl Into<ElementId>, change: Change, tokens: &Tokens) -> AnyElement {
    let after = Geometry::of(change.after);
    let foreground = tokens.neutral.foreground;
    if change.after == State::Muted || change.after == change.before {
        return bars(after, foreground).into_any_element();
    }
    Transition {
        before: Geometry::of(change.before),
        after,
        progress: 0.0,
        foreground,
    }
    .with_animation(
        id,
        motion::transition(motion::QUICK),
        |mut state, progress| {
            state.progress = progress;
            state
        },
    )
    .into_any_element()
}

#[cfg(test)]
mod tests;
