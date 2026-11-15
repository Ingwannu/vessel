//! A figure as the shapes the kit paints: closed loops stroked at the
//! mark's line weight, the star filled, each with its share of the ink.
#![expect(
    clippy::float_arithmetic,
    reason = "line weights and ink shares are products of the mark's ratios"
)]

use super::{Figure, Loops, Pt, geometry, motion::Journey, ratios};

/// What one colour the drawing is painted in, and how strongly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ink {
    /// The colour, `0xRRGGBB`.
    pub rgb: u32,
    /// Its opacity, 0 to 1 (the study's quiet ink: 0.07 on dark, 0.10 of
    /// the brand colour on light).
    pub alpha: f32,
}

/// One shape: a polyline, stroked or filled.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    /// Its points, in logical points.
    pub points: Vec<Pt>,
    /// Whether the last point joins the first.
    pub closed: bool,
    /// The stroke's width in points, or `None` for a fill.
    pub stroke: Option<f32>,
    /// Its share of the ink's opacity, 0 to 1.
    pub alpha: f32,
}

fn loop_shapes(v: &Loops, out: &mut Vec<Shape>) {
    let centre_line = [v.arms.left, v.arms.apex, v.arms.right];
    let radius = f32::from(v.loops) * v.pitch;
    let width = ratios::LINE * v.pitch;
    for k in 0..v.loops {
        let alpha = if k.saturating_add(1) == v.loops {
            v.alpha * v.outer
        } else {
            v.alpha
        };
        if alpha <= 0.0 {
            continue;
        }
        let d = (f32::from(k) + 0.5) * v.pitch;
        out.push(Shape {
            points: geometry::loop_around(&centre_line, radius, d),
            closed: true,
            stroke: Some(width),
            alpha,
        });
    }
}

/// The figure's shapes, back to front.
#[must_use]
pub fn shapes(figure: &Figure) -> Vec<Shape> {
    let mut out = Vec::new();
    if let Some(v) = &figure.v {
        loop_shapes(v, &mut out);
    }
    if let Some(star) = figure.star {
        out.push(Shape {
            points: geometry::star(star.centre, star.span, 0.0),
            closed: true,
            stroke: None,
            alpha: 1.0,
        });
    }
    out
}

/// The avatar's ring stroke, in points (the study's 1.25 at 24).
pub const RING: f32 = 1.25;

/// The avatar of `size` points at `centre`, `journey` of the way: the star
/// relaxing into a disc (the morph), the disc giving way to a ring (the
/// mark phase; the agent's own mark is drawn inside by the kit at the same
/// share). Each shape's `alpha` is relative: the disc's to the drawing's
/// ink, the ring's to the avatar's ring colour.
#[must_use]
pub fn avatar(centre: Pt, size: f32, journey: Journey) -> Vec<Shape> {
    let mut out = Vec::new();
    let disc = 1.0 - journey.marked;
    if disc > 0.0 {
        out.push(Shape {
            points: geometry::star(centre, size, journey.morphed),
            closed: true,
            stroke: None,
            alpha: disc,
        });
    }
    if journey.marked > 0.0 {
        out.push(Shape {
            points: geometry::star(centre, size - RING, 1.0),
            closed: true,
            stroke: Some(RING),
            alpha: journey.marked,
        });
    }
    out
}
