//! A QR matrix, dark on light in either theme, as Paseo's pairing tile.
//!
//! [Decision Log]
//! - Purpose: one scannable QR presentation for every pairing screen.
//! - Constraints: the encoder has no GPUI dependency; the kit never sees
//!   the source link. QR modules are credentials too: never log or save.
//! - Alternatives: an SVG file (leaves a credential on disk), a themed
//!   matrix (inverted codes scan inconsistently), per-module divs (many
//!   layout nodes for one picture).
//! - Choice: one canvas given the side and dark module positions, with
//!   the standard four-module quiet zone and integer-pixel modules sized
//!   to its bounds. Black and white here are QR protocol colours, not UI
//!   theme colours. No interpolation or rounded module corners.
//! - Cost: unused pixels centre the integer-pixel matrix within its box.

use gpui::{Bounds, Div, canvas, div, fill, point, prelude::*, px, size};
use std::num::NonZeroU16;

/// Paseo's desktop pairing tile; the page may cap it to its available room.
pub const PAIRING_SIZE: f32 = 304.0;

/// Draw a matrix supplied by a view model. `side` excludes the quiet
/// zone; positions outside it are ignored. Nothing retains the source
/// text, and the next masked frame drops the matrix with its canvas.
#[must_use]
pub fn qr(
    side: NonZeroU16,
    dark: impl IntoIterator<Item = (u16, u16)>,
    width: gpui::Pixels,
) -> Div {
    let dark: Vec<_> = dark
        .into_iter()
        .filter(|(x, y)| *x < side.get() && *y < side.get())
        .collect();
    div().w(width).h(width).max_w_full().child(
        canvas(
            |bounds, _window, _cx| bounds,
            move |_, bounds, window, _cx| {
                window.paint_quad(fill(bounds, gpui::white()));
                let total = f32::from(side.get()).mul_add(1.0, 8.0);
                let step = total
                    .recip()
                    .mul_add(f32::from(bounds.size.width.min(bounds.size.height)), 0.0)
                    .floor();
                if step < 1.0 {
                    return;
                }
                let edge = step.mul_add(total, 0.0);
                let offset_x = edge
                    .mul_add(-1.0, f32::from(bounds.size.width))
                    .mul_add(0.5, 0.0)
                    .floor();
                let offset_y = edge
                    .mul_add(-1.0, f32::from(bounds.size.height))
                    .mul_add(0.5, 0.0)
                    .floor();
                for (x, y) in &dark {
                    let origin = point(
                        px(f32::from(*x)
                            .mul_add(1.0, 4.0)
                            .mul_add(step, offset_x)
                            .mul_add(1.0, f32::from(bounds.origin.x))),
                        px(f32::from(*y)
                            .mul_add(1.0, 4.0)
                            .mul_add(step, offset_y)
                            .mul_add(1.0, f32::from(bounds.origin.y))),
                    );
                    window.paint_quad(fill(
                        Bounds::new(origin, size(px(step), px(step))),
                        gpui::black(),
                    ));
                }
            },
        )
        .size_full(),
    )
}
