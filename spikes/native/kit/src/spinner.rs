//! A spinner: work turning (the page's `ui/kit/spinner.tsx`, beside words
//! that already say so). Three quarters of a ring, once round a second.

use std::f32::consts::TAU;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{Animation, AnimationExt as _, AnyElement, ElementId, Hsla, Transformation, radians};

use crate::assets::Glyph;
use crate::picker::glyph;

/// One turn.
const TURN: Duration = Duration::from_secs(1);

/// A spinner `size` square in `color`; `id` keeps its turn across frames.
#[must_use]
pub fn spinner(id: impl Into<ElementId>, size: f32, color: Hsla) -> AnyElement {
    glyph(Glyph::Spinner, size, color)
        .with_animation(id, Animation::new(TURN).repeat(), |ring, delta| {
            ring.with_transformation(Transformation::rotate(radians(delta.mul_add(TAU, 0.0))))
        })
        .into_any_element()
}
