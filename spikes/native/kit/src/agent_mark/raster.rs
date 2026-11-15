//! A mark drawn from its vector at the display's own pixels.
//!
//! [Decision Log]
//! - Purpose: the owner saw the agents' marks broken on their screen while
//!   Paseo, GOAT and agent-orchestrator draw the same marks cleanly
//!   (2026-11-14). Every mark must be sharp at any scale factor (1, 1.25,
//!   1.5, 2) and at any position, in its owner's colours.
//! - Cause, read in the pinned GPUI (`gpui-pre` 0.3.10): `img()` hands an
//!   SVG to `SvgRenderer::render_single_frame(bytes, 1.0)`, which rasterises
//!   it once at twice the SVG's own size (Codex's 721-unit view box becomes
//!   a 1442-pixel texture, `OpenCode`'s 960 × 1080), and `paint_image`
//!   stretches that texture onto the element's bounds. Shrunk 12:1
//!   (Claude) to 65:1 (Codex) by a bilinear sampler without mipmaps, each
//!   device pixel reads two by two texels of a much larger block: thin
//!   strokes break up and edges stair-step, worse at 1.25 and 1.5 where
//!   the ratio is not even. `svg()` is sharp because `paint_svg`
//!   rasterises at the bounds' device size (twice it, box-filtered 2:1),
//!   but it draws a one-colour mask: Gemini's gradient and `OpenCode`'s two
//!   tones cannot go through it.
//! - Alternatives: `svg()` for the one-colour marks and layered masks for
//!   `OpenCode` (two drawing paths, and no way to draw Gemini's gradient);
//!   `img()` with a pre-shrunk asset per size (still resampled at 1.25 and
//!   1.5, and one asset per size and scale); a patch to GPUI's image loader
//!   (a fork of a vendored crate for one control).
//! - Choice: one path for every mark. At paint, the element's bounds are
//!   snapped to the device grid (origin rounded, size the rounded device
//!   size), the reviewed SVG is rasterised by GPUI's own renderer
//!   (`render_parsed` with `SvgSize::ExactSize`, resvg underneath) at
//!   exactly that many device pixels, and `paint_image` draws it 1:1, so
//!   every texel lands on one device pixel and nothing is resampled. The
//!   raster is cached per (asset, device size) for the life of the app.
//! - Cost: a new size or scale factor rasterises once, on the UI thread;
//!   the cache holds one small texture per asset, size and scale in use
//!   and is never trimmed (five marks, two schemes, a few sizes). A mark's origin can move by up to half a device
//!   pixel to land on the grid, as text and `svg()` glyphs do.
//! - Keep: no resampling anywhere between the vector and the screen. If
//!   this changes, compare the crops with a reference rasteriser at the
//!   same device size (`kit/examples/mark_sharpness.rs`).
//! - A tint (the session list's muted marks, as Paseo's `agent-list.tsx`
//!   draws its provider icons in `foregroundMuted`): the same raster with
//!   every pixel's colour replaced by the tint and its coverage kept, so
//!   a muted mark is exactly as sharp as the coloured one. A mark whose
//!   parts differ only in colour (`OpenCode`'s inner square) reads as one
//!   silhouette then.
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use gpui::{
    App, Bounds, Corners, DevicePixels, Hsla, Pixels, RenderImage, Rgba, Styled as _, SvgSize,
    Window, canvas, point, px, size,
};

use super::Image;

/// An asset's name, and its device width and height.
type Key = (&'static str, i32, i32, Option<[u8; 4]>);

thread_local! {
    /// Rasters by [`Key`]. GPUI draws on one thread; an `Arc` keeps one
    /// image id, so the atlas uploads it once.
    static RASTERS: RefCell<HashMap<Key, Arc<RenderImage>>> = RefCell::new(HashMap::new());
}

/// `image` at `logical` points, drawn at the display's pixels, in its own
/// colours or in `tint`.
pub(super) fn exact(
    image: Image,
    logical: gpui::Size<Pixels>,
    tint: Option<Hsla>,
) -> impl gpui::IntoElement {
    let tint = tint.map(bgra);
    canvas(
        |_, _, _| {},
        move |bounds, (), window, cx| {
            paint(image, tint, bounds, window, cx);
        },
    )
    .flex_none()
    .w(logical.width)
    .h(logical.height)
}

/// The device-pixel box `bounds` lands on at `scale`: the origin rounded
/// to the nearest device pixel, the size its rounded device size (never
/// zero), so the raster and the box agree to the pixel.
pub(super) fn device_box(bounds: Bounds<Pixels>, scale: f32) -> (i32, i32, i32, i32) {
    let device = |logical: Pixels| whole(f32::from(logical).mul_add(scale, 0.0));
    (
        device(bounds.origin.x),
        device(bounds.origin.y),
        device(bounds.size.width).max(1),
        device(bounds.size.height).max(1),
    )
}

/// The nearest whole device pixel.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    reason = "a window's coordinates in device pixels are far inside i32; `as` saturates"
)]
fn whole(device: f32) -> i32 {
    device.round() as i32
}

/// `device` pixels in logical points at `scale`.
#[expect(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    reason = "a window's coordinates in device pixels are exact in f32"
)]
fn points(device: i32, scale: f32) -> Pixels {
    px((device as f32).mul_add(scale.recip(), 0.0))
}

fn paint(
    image: Image,
    tint: Option<[u8; 4]>,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let scale = window.scale_factor();
    let (x, y, width, height) = device_box(bounds, scale);
    let Some(raster) = raster(image, (width, height), tint, cx) else {
        return;
    };
    let snapped = Bounds {
        origin: point(points(x, scale), points(y, scale)),
        size: size(points(width, scale), points(height, scale)),
    };
    if let Err(error) = window.paint_image(snapped, snapped, Corners::default(), raster, 0, false) {
        tracing::warn!(%error, asset = image.path, "an agent's mark could not be drawn");
    }
}

fn raster(
    image: Image,
    (width, height): (i32, i32),
    tint: Option<[u8; 4]>,
    cx: &App,
) -> Option<Arc<RenderImage>> {
    let key = (image.path, width, height, tint);
    if let Some(found) = RASTERS.with_borrow(|rasters| rasters.get(&key).map(Arc::clone)) {
        return Some(found);
    }
    let renderer = cx.svg_renderer();
    let made = renderer
        .parse_svg(image.bytes)
        .and_then(|parsed| {
            renderer.render_parsed(
                &parsed,
                SvgSize::ExactSize(size(DevicePixels(width), DevicePixels(height))),
            )
        })
        .map(|made| tint.and_then(|tint| tinted(&made, tint)).unwrap_or(made));
    match made {
        Ok(made) => {
            let _first = RASTERS.with_borrow_mut(|rasters| rasters.insert(key, Arc::clone(&made)));
            Some(made)
        }
        Err(error) => {
            tracing::warn!(%error, asset = image.path, "an agent's mark could not be rasterised");
            None
        }
    }
}

/// One over 255 squared (a byte times a byte, as a share).
const INVERSE_SQUARE: f32 = 1.0 / 65_025.0;

/// `colour` as GPUI's straight BGRA bytes.
fn bgra(colour: Hsla) -> [u8; 4] {
    let Rgba { r, g, b, a } = colour.to_rgb();
    [byte(b), byte(g), byte(r), byte(a)]
}

/// A channel 0 to 1 as a byte.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0..=255 and rounded before the cast"
)]
fn byte(channel: f32) -> u8 {
    channel.mul_add(255.0, 0.0).round().clamp(0.0, 255.0) as u8
}

/// `made` with every pixel in `tint` (straight BGRA), its coverage kept and
/// scaled by the tint's own alpha.
fn tinted(made: &RenderImage, tint: [u8; 4]) -> Option<Arc<RenderImage>> {
    let bytes = made.as_bytes(0)?;
    let device = made.size(0);
    let mut out = Vec::with_capacity(bytes.len());
    let (pixels, _rest) = bytes.as_chunks::<4>();
    // Coverage times the tint's alpha, both 0 to 255: the product over
    // 255 squared is the share of full opacity.
    let per = f32::from(tint[3]).mul_add(INVERSE_SQUARE, 0.0);
    for pixel in pixels {
        let alpha = byte(f32::from(pixel[3]).mul_add(per, 0.0));
        out.extend_from_slice(&[tint[0], tint[1], tint[2], alpha]);
    }
    let width = u32::try_from(device.width.0).ok()?;
    let height = u32::try_from(device.height.0).ok()?;
    let buffer = image::RgbaImage::from_raw(width, height, out)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, RenderImage, point, px, size};

    use super::{device_box, tinted};

    /// A tint replaces the colour and keeps each pixel's coverage, scaled
    /// by the tint's own alpha.
    #[test]
    fn a_tint_keeps_the_coverage() {
        let bytes = vec![10, 20, 30, 255, 200, 100, 50, 128, 0, 0, 0, 0];
        let buffer = image::RgbaImage::from_raw(3, 1, bytes).unwrap();
        let made = RenderImage::new(vec![image::Frame::new(buffer)]);
        let full = tinted(&made, [1, 2, 3, 255]).unwrap();
        assert_eq!(
            full.as_bytes(0).unwrap(),
            &[1, 2, 3, 255, 1, 2, 3, 128, 1, 2, 3, 0]
        );
        let half = tinted(&made, [1, 2, 3, 128]).unwrap();
        assert_eq!(half.as_bytes(0).unwrap()[3], 128);
        assert_eq!(half.as_bytes(0).unwrap()[7], 64);
    }

    /// Off-grid origins and fractional sizes land on whole device pixels
    /// at every scale the owner's screens use.
    #[test]
    fn a_mark_lands_on_whole_device_pixels_at_every_scale() {
        let bounds = Bounds {
            origin: point(px(22.25), px(24.6)),
            size: size(px(24.102_564), px(24.102_564)),
        };
        assert_eq!(device_box(bounds, 1.0), (22, 25, 24, 24));
        assert_eq!(device_box(bounds, 1.25), (28, 31, 30, 30));
        assert_eq!(device_box(bounds, 1.5), (33, 37, 36, 36));
        assert_eq!(device_box(bounds, 2.0), (45, 49, 48, 48));
        let opencode = Bounds {
            origin: point(px(0.3), px(0.6)),
            size: size(px(32.0), px(36.0)),
        };
        assert_eq!(device_box(opencode, 1.25), (0, 1, 40, 45));
        let tiny = Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(0.1), px(0.1)),
        };
        assert_eq!(device_box(tiny, 1.0), (0, 0, 1, 1));
    }
}
