//! The window controls' glyphs (minimise, maximise, restore, close) in the
//! mark's line language, drawn in whole device pixels at every scale.
//!
//! [Decision Log]
//! - Purpose: the owner asked for the title bar's controls "small, in the
//!   same feel" as the brand's multiline drawing (2026-11-14); the lead
//!   kept the cells (46 × 36, close to the window's corner, one click, the
//!   same hover and pressed fills) and asked for the glyphs only, sharp at
//!   1.25 and 1.5 as at 1 and 2.
//! - Rules (`docs/design/brand/README.md`, "Multiline rules"): one line
//!   with round ends; corners turn on circles of half a pitch; where one
//!   stroke passes in front of another, the back one breaks for one gap
//!   (the cross; the restored window's back square). The mark's loops
//!   (two lines a stroke) read only from 12 points at scale 1 and were
//!   measured unreadable at 9, so a glyph is one line.
//! - Pixels: a 9-point glyph is `round(9 × scale)` device pixels; the line
//!   is `max(1, ⌊scale + ¼⌋)` pixels (1, 1, 1, 2 at 1, 1.25, 1.5, 2: two
//!   pixels at 1.5 read heavier than the text beside it) and the gap at
//!   least two pixels (`1.33 ×` the line), so a break or a doubled edge
//!   never closes into a grey band. Straight strokes cover whole device
//!   rows and columns, drawn through `vector::snapped` from the glyph's
//!   own snapped origin; only the cross's diagonals and the corners are
//!   antialiased.
//! - Alternatives: `svg()` with one 9 × 9 vector (rasterised at
//!   `9 × scale`, so at 1.25 a one-unit stroke straddles two device rows
//!   and greys, the lead's finding); one asset per scale (GPUI picks no
//!   asset by scale).
//! - Cost: one small raster per glyph, colour and scale, cached.
#![expect(
    clippy::float_arithmetic,
    reason = "glyph geometry in device pixels is float arithmetic on whole-pixel values"
)]

use gpui::{
    Div, Hsla, InteractiveElement as _, ParentElement as _, SharedString, Styled as _, div, px,
    size,
};

use crate::vector::{self, Shape};

/// A window control's glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowGlyph {
    /// Minimise: one line.
    Minimize,
    /// Maximise: a square with corners on half a pitch.
    Maximize,
    /// Restore: a square in front of another; the back one breaks a gap
    /// short of it.
    Restore,
    /// Close: a cross whose back stroke breaks a gap either side of the
    /// front one.
    Close,
}

/// The glyph's size in points.
pub const SIZE: f32 = 9.0;

/// Line, gap and pitch in whole device pixels at `scale`.
#[must_use]
pub fn metrics(scale: f32) -> (f32, f32, f32) {
    let line = (scale + 0.25).floor().max(1.0);
    let gap = (1.33 * line).round().max(2.0);
    (line, gap, line + gap)
}

/// A rounded rectangle's centre line from `(x0, y0)` to `(x1, y1)`, its
/// corners on circles of radius `r`, closed.
fn rounded(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> Vec<(f32, f32)> {
    const STEPS: u8 = 6;
    let corners = [
        (x1 - r, y0 + r, -std::f32::consts::FRAC_PI_2),
        (x1 - r, y1 - r, 0.0),
        (x0 + r, y1 - r, std::f32::consts::FRAC_PI_2),
        (x0 + r, y0 + r, std::f32::consts::PI),
    ];
    let mut out = Vec::new();
    for (cx, cy, from) in corners {
        for i in 0..=STEPS {
            let a = std::f32::consts::FRAC_PI_2.mul_add(f32::from(i) / f32::from(STEPS), from);
            out.push((r.mul_add(a.cos(), cx), r.mul_add(a.sin(), cy)));
        }
    }
    out
}

fn line(points: Vec<(f32, f32)>, width: f32, closed: bool) -> Shape {
    Shape {
        points,
        closed,
        stroke: Some(width),
        alpha: 1.0,
    }
}

/// The glyph's shapes in device pixels from its snapped origin.
#[must_use]
pub fn shapes(glyph: WindowGlyph, scale: f32) -> Vec<Shape> {
    let (w, _gap, pitch) = metrics(scale);
    let side = (SIZE * scale).round();
    let half = w / 2.0;
    let radius = (pitch / 2.0).max(half);
    match glyph {
        WindowGlyph::Minimize => {
            let y = ((side - w) / 2.0).floor() + half;
            vec![line(vec![(half, y), (side - half, y)], w, false)]
        }
        WindowGlyph::Maximize => vec![line(
            rounded(half, half, side - half, side - half, radius),
            w,
            true,
        )],
        WindowGlyph::Restore => {
            let front = side - pitch;
            let back = vec![(pitch + half + radius, half), (side - half - radius, half)];
            let mut corner = Vec::new();
            for i in 0..=6_u8 {
                let a = std::f32::consts::FRAC_PI_2
                    .mul_add(f32::from(i) / 6.0, -std::f32::consts::FRAC_PI_2);
                corner.push((
                    radius.mul_add(a.cos(), side - half - radius),
                    radius.mul_add(a.sin(), half + radius),
                ));
            }
            let mut path = back;
            path.extend(corner);
            path.push((side - half, front - half - radius));
            vec![
                line(
                    rounded(half, pitch + half, front - half, side - half, radius),
                    w,
                    true,
                ),
                line(path, w, false),
            ]
        }
        WindowGlyph::Close => {
            let centre = side / 2.0;
            let reach = pitch / std::f32::consts::SQRT_2;
            vec![
                line(vec![(half, side - half), (side - half, half)], w, false),
                line(
                    vec![(half, half), (centre - reach, centre - reach)],
                    w,
                    false,
                ),
                line(
                    vec![(centre + reach, centre + reach), (side - half, side - half)],
                    w,
                    false,
                ),
            ]
        }
    }
}

/// The glyph in `rest`, turning `hover` while the pointer is over the
/// cell named `group` (the close glyph's white on red); `None` keeps it.
#[must_use]
pub fn window_glyph(
    glyph: WindowGlyph,
    rest: Hsla,
    hover: Option<Hsla>,
    group: impl Into<SharedString>,
) -> Div {
    let box_ = size(px(SIZE), px(SIZE));
    let layer = move |ink: Hsla| vector::snapped(box_, ink, move |scale| shapes(glyph, scale));
    let group: SharedString = group.into();
    let base = div().absolute().top_0().left_0().child(layer(rest));
    let mut cell = div().relative().flex_none().size(px(SIZE));
    match hover {
        Some(ink) => {
            let shown = group.clone();
            cell = cell
                .child(base.group_hover(group, |style| style.opacity(0.0)))
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .opacity(0.0)
                        .group_hover(shown, |style| style.opacity(1.0))
                        .child(layer(ink)),
                );
        }
        None => cell = cell.child(base),
    }
    cell
}

#[cfg(test)]
mod tests;
