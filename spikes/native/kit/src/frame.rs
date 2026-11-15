//! The content panel's edge in the mark's line language: two fine parallel
//! lines one gap apart, turning the corner on concentric circles, each on
//! whole device pixels at every scale.
//!
//! [Decision Log]
//! - Purpose: the owner wants the window's edges to carry the multiline
//!   feel, "parallel lines, a circuit simplified" (2026-11-14); the study
//!   drew one line (today), two, and three with pads; two read as the
//!   mark's lines and stay quieter than the content. The lead asked for a
//!   kit element: device-pixel lines, theme tokens, none at a phone's width.
//! - Rules: the line and the gap are the window glyphs' (`window_glyph::
//!   metrics`: one pixel and two at 1, 1.25 and 1.5, two and three at 2);
//!   the outer line's corner is the inner one's radius plus one pitch, so
//!   the two stay one gap apart round the corner.
//! - Alternatives: GPUI's `border_1` (one logical point: 1.25 device pixels
//!   at 1.25, greyed over two rows, and one line only); the drawing's
//!   raster (`vector`, a texture the panel's size for two thin lines).
//! - Choice: two quads whose bounds and border widths are whole device
//!   pixels converted back to points, painted by GPUI's quad shader: crisp
//!   straight edges, antialiased corners, no texture.
//! - Placement: the element fills the panel's own box and paints the inner
//!   line on the panel's edge and the outer one a pitch outside it, in the
//!   frame's margin, so it must sit in a parent that does not clip (the
//!   panel itself clips its content), and the panel drops its own border.
//! - Cost: two quads per frame the panel is drawn; nothing else.
#![expect(
    clippy::float_arithmetic,
    reason = "device-pixel rectangles converted to points"
)]

use gpui::{BorderStyle, Bounds, Corners, Edges, Styled as _, canvas, point, px, size};

use crate::Tokens;
use crate::window_glyph::metrics;

/// How many lines the edge has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lines {
    /// One line on the panel's edge (a phone's width, where the margin is
    /// the sheet's).
    One,
    /// The panel's edge and one more a pitch outside it.
    Two,
}

/// The panel's edge, `radius` points round, to place absolutely over the
/// panel's box (`top_0`, `left_0`, `size_full`) in a parent that does not
/// clip.
#[must_use]
pub fn panel_edges(radius: f32, lines: Lines, tokens: &Tokens) -> impl gpui::IntoElement {
    let colour = match lines {
        Lines::One => tokens.border_faint,
        Lines::Two => tokens.neutral.border,
    };
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _cx| {
            let scale = window.scale_factor();
            for edge in edges(bounds, radius, lines, scale) {
                window.paint_quad(gpui::quad(
                    edge.bounds,
                    Corners::all(edge.radius),
                    gpui::transparent_black(),
                    Edges::all(edge.width),
                    colour,
                    BorderStyle::Solid,
                ));
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// One line of the edge: its outer box, corner radius and width, in points
/// that land on whole device pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Edge {
    pub(crate) bounds: Bounds<gpui::Pixels>,
    pub(crate) radius: gpui::Pixels,
    pub(crate) width: gpui::Pixels,
}

/// The edge's lines for a panel at `bounds`, at `scale`.
pub(crate) fn edges(
    bounds: Bounds<gpui::Pixels>,
    radius: f32,
    lines: Lines,
    scale: f32,
) -> Vec<Edge> {
    let device = |logical: gpui::Pixels| (f32::from(logical) * scale).round();
    let (x0, y0) = (device(bounds.origin.x), device(bounds.origin.y));
    let (x1, y1) = (device(bounds.right()), device(bounds.bottom()));
    let (line, _gap, pitch) = metrics(scale);
    let corner = (radius * scale).round();
    let count: u8 = match lines {
        Lines::One => 1,
        Lines::Two => 2,
    };
    (0..count)
        .map(|k| {
            let out = pitch * f32::from(k);
            Edge {
                bounds: Bounds {
                    origin: point(px((x0 - out) / scale), px((y0 - out) / scale)),
                    size: size(
                        px((x1 - x0 + 2.0 * out) / scale),
                        px((y1 - y0 + 2.0 * out) / scale),
                    ),
                },
                radius: px((corner + out) / scale),
                width: px(line / scale),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
