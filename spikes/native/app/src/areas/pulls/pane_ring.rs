//! The checks' ring: Paseo's `pull-request-panel/checks-ring.tsx` (a 20
//! box, a stroke of 3, the track in `surface3`, one segment per group from
//! twelve o'clock clockwise), painted on a canvas as the context meter's
//! ring is (GPUI draws no SVG built at run time); split from
//! `pane_checks` for the file budget.

use gpui::{Bounds, PathBuilder, Pixels, Point, canvas, point};
use vessel_app_kit::Tokens;
use vessel_app_kit::ui::{AnyElement, Hsla, IntoElement as _, Styled as _, px};
use vessel_app_view::workspace::pull::checks::{
    RING_RADIUS, RING_SIZE, RING_STROKE, Segment, circumference,
};

use super::pane_checks::colour;

/// Points along a whole turn of an arc.
const STEPS: u16 = 64;

/// Paseo's ring: the track, then each segment from twelve o'clock.
pub(super) fn ring(arcs: &[Segment], tokens: &Tokens) -> AnyElement {
    let track = tokens.neutral.surface3;
    let painted: Vec<(Segment, Hsla)> = arcs
        .iter()
        .map(|arc| (*arc, colour(arc.presentation, tokens)))
        .collect();
    // One part is the whole ring (Paseo's single dash): a closed circle.
    let whole_ring = painted.len() == 1;
    canvas(
        |_, _, _| (),
        move |bounds, (), window, _cx| {
            if let Some(path) = arc_path(bounds, 0.0, circumference(), true) {
                window.paint_path(path, track);
            }
            for (arc, ink) in &painted {
                if let Some(path) = arc_path(bounds, arc.start, arc.length, whole_ring) {
                    window.paint_path(path, *ink);
                }
            }
        },
    )
    .size(px(RING_SIZE))
    .flex_none()
    .into_any_element()
}

/// The stroke of an arc `length` long from `start` along the ring.
fn arc_path(
    bounds: Bounds<Pixels>,
    start: f32,
    length: f32,
    closed: bool,
) -> Option<gpui::Path<Pixels>> {
    let mut points = arc_points(bounds, start, length);
    if closed {
        let _end = points.pop();
    }
    let (first, rest) = points.split_first()?;
    let mut path = PathBuilder::stroke(px(RING_STROKE));
    path.move_to(*first);
    for next in rest {
        path.line_to(*next);
    }
    if closed {
        path.close();
    }
    path.build().ok()
}

#[expect(
    clippy::float_arithmetic,
    clippy::arithmetic_side_effects,
    reason = "an arc's points are trigonometry on the canvas's floats"
)]
fn arc_points(bounds: Bounds<Pixels>, start: f32, length: f32) -> Vec<Point<Pixels>> {
    let center = bounds.center();
    let radius = RING_RADIUS;
    let whole = circumference();
    let from = std::f32::consts::TAU * start / whole;
    let turn = std::f32::consts::TAU * length / whole;
    (0..=STEPS)
        .map(|step| {
            let angle = from + turn * f32::from(step) / f32::from(STEPS);
            point(
                center.x + px(radius * angle.sin()),
                center.y - px(radius * angle.cos()),
            )
        })
        .collect()
}
