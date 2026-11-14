//! Shapes computed elsewhere (the brand's drawing behind a screen, the
//! agent's avatar), drawn from their vectors at the display's own pixels.
//!
//! [Decision Log]
//! - Purpose: the brand's multiline drawing must be as sharp as the mark at
//!   scale 1, 1.25, 1.5 and 2 (the owner's checks are zoomed crops), cost
//!   nothing while nothing moves, and follow one bounded transition.
//! - Constraints: the kit knows no brand and no view; the shapes come in
//!   as points in the window's logical coordinates with a stroke or a
//!   fill and a share of one ink.
//! - Alternatives: GPUI paths (`PathBuilder`, tessellated by lyon on every
//!   paint and drawn with at most 4× MSAA: five coverage levels on an edge,
//!   visible as steps on the 60° lines in an 800 % crop); `svg()` (one
//!   mask colour, rasterised at the element's whole bounds); `img()`
//!   (resampled, the cause of the agents' broken marks).
//! - Choice: the agent marks' method (`agent_mark/raster.rs`): the shapes
//!   are written as one SVG over their own bounding box, rasterised by
//!   GPUI's renderer (resvg) at exactly the device pixels that box covers,
//!   and painted 1:1. Rasters are kept by content and size in a small
//!   cache; one pushed out is dropped from GPUI's atlas, so a transition's
//!   frames do not pile up textures. Of the large ones (a backdrop the
//!   panel's size, about 3 MB at 1280 × 800 and scale 1, held once in the
//!   cache and once in the atlas) only the two newest stay, so a journey's
//!   frames cannot keep twelve; small ones (glyphs, avatars) are drawn every
//!   frame and must not be pushed out by a transition, or they would be
//!   rasterised again each frame. A backdrop rasterises only the area it
//!   fills: the whole extent of the open session's frame (its apex far
//!   below the screen) made one raster of about 70 MB (measured: the app's
//!   memory 75 MB above a build without the drawing).
//! - Cost: at rest one texture per drawing and nothing per frame; a new
//!   layout or a transition frame rasterises once (measured in the brand's
//!   notes: the 1920 drawing at scale 2 in a few milliseconds).
//! - Keep: no resampling between the vector and the screen.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::hash::{DefaultHasher, Hash as _, Hasher as _};
use std::sync::Arc;

use gpui::{
    App, Bounds, Corners, DevicePixels, Pixels, RenderImage, Styled as _, SvgSize, Window, canvas,
    point, px, size,
};

/// One shape: a polyline in the window's logical points.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    /// Its points, `(x, y)`.
    pub points: Vec<(f32, f32)>,
    /// Whether the last point joins the first.
    pub closed: bool,
    /// The stroke's width in points, or `None` for a fill.
    pub stroke: Option<f32>,
    /// Its share of the ink's opacity, 0 to 1.
    pub alpha: f32,
}

/// Shapes in one ink.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawing {
    /// Back to front.
    pub shapes: Vec<Shape>,
    /// The ink, a theme colour with its opacity.
    pub ink: gpui::Hsla,
}

/// How many rasters are kept: a backdrop at rest, the frame before it in a
/// transition, and the avatars of one conversation (one per agent kind
/// and size; every message of an agent shares its raster).
const KEPT: usize = 12;

/// A raster above this many bytes (four a device pixel, 512 × 512) is
/// large: a backdrop, not a glyph or an avatar.
const LARGE: u64 = 1 << 20;

/// How many large rasters are kept: the one drawn and the one before it.
const LARGE_KEPT: usize = 2;

/// A raster's identity: its SVG's hash, its device width and height.
type Key = (u64, i32, i32);

/// The kept rasters, the newest last.
type Kept = Vec<(Key, Arc<RenderImage>)>;

thread_local! {
    /// Rasters by [`Key`]. GPUI draws on one thread.
    static RASTERS: RefCell<Kept> = const { RefCell::new(Vec::new()) };
}

/// A drawing made at paint time from the area it fills (its shapes in
/// the window's coordinates), painted where its shapes are. Place it to
/// fill the area it may show in, absolutely, before (behind) the content;
/// the area clips it. Making it at paint lets it use boxes the content
/// measured while it was laid out in the same frame.
pub fn drawing(
    make: impl FnOnce(Bounds<Pixels>, &mut Window, &mut App) -> Option<Drawing> + 'static,
) -> impl gpui::IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, cx| {
            if let Some(drawing) = make(bounds, window, cx) {
                paint(&drawing, Some(bounds), window, cx);
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// The drawing in a box of `logical` points that its shapes are laid out
/// in (an avatar beside a message): the shapes' coordinates are relative to
/// the box's origin.
#[must_use]
pub fn inline(drawing: Arc<Drawing>, logical: gpui::Size<Pixels>) -> impl gpui::IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, cx| {
            let origin = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
            paint(&moved(&drawing, origin), None, window, cx);
        },
    )
    .flex_none()
    .w(logical.width)
    .h(logical.height)
}

/// Shapes laid out in whole device pixels from the box's own origin,
/// snapped to the device grid, and made for the scale they are drawn at
/// (`make` gets the scale and returns points and stroke widths in device
/// pixels): a glyph whose straight strokes must cover whole device rows at
/// 1.25 and 1.5 as at 1 and 2 (the window controls).
pub fn snapped(
    logical: gpui::Size<Pixels>,
    ink: gpui::Hsla,
    make: impl Fn(f32) -> Vec<Shape> + 'static,
) -> impl gpui::IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, cx| {
            let scale = window.scale_factor();
            let origin = (
                device_float(f32::from(bounds.origin.x), scale),
                device_float(f32::from(bounds.origin.y), scale),
            );
            let drawing = Drawing {
                shapes: make(scale)
                    .into_iter()
                    .map(|shape| to_logical(shape, origin, scale))
                    .collect(),
                ink,
            };
            paint(&drawing, None, window, cx);
        },
    )
    .flex_none()
    .w(logical.width)
    .h(logical.height)
}

/// A logical coordinate's nearest whole device pixel, as a float.
#[expect(clippy::float_arithmetic, reason = "points to device pixels")]
fn device_float(logical: f32, scale: f32) -> f32 {
    (logical * scale).round()
}

/// A shape in device pixels from a device origin, in logical points.
#[expect(clippy::float_arithmetic, reason = "device pixels back to points")]
fn to_logical(shape: Shape, (ox, oy): (f32, f32), scale: f32) -> Shape {
    Shape {
        points: shape
            .points
            .into_iter()
            .map(|(x, y)| ((ox + x) / scale, (oy + y) / scale))
            .collect(),
        stroke: shape.stroke.map(|width| width / scale),
        ..shape
    }
}

#[expect(clippy::float_arithmetic, reason = "a translation")]
fn moved(drawing: &Drawing, (dx, dy): (f32, f32)) -> Drawing {
    Drawing {
        shapes: drawing
            .shapes
            .iter()
            .map(|shape| Shape {
                points: shape
                    .points
                    .iter()
                    .map(|&(x, y)| (x + dx, y + dy))
                    .collect(),
                ..shape.clone()
            })
            .collect(),
        ink: drawing.ink,
    }
}

/// `extent` within `clip` (left, top, right, bottom), or `None` when
/// nothing of it is inside.
fn clipped(
    (x0, y0, x1, y1): (f32, f32, f32, f32),
    clip: Option<Bounds<Pixels>>,
) -> Option<(f32, f32, f32, f32)> {
    let Some(clip) = clip else {
        return Some((x0, y0, x1, y1));
    };
    let (left, top) = (f32::from(clip.origin.x), f32::from(clip.origin.y));
    let (right, bottom) = (f32::from(clip.right()), f32::from(clip.bottom()));
    let inside = (x0.max(left), y0.max(top), x1.min(right), y1.min(bottom));
    (inside.0 < inside.2 && inside.1 < inside.3).then_some(inside)
}

/// The shapes' bounds, grown by half the widest stroke and one point of
/// air for the antialiased edge.
#[expect(clippy::float_arithmetic, reason = "bounds grown by a stroke's half")]
fn extent(drawing: &Drawing) -> Option<(f32, f32, f32, f32)> {
    let grow = drawing
        .shapes
        .iter()
        .filter_map(|shape| shape.stroke)
        .fold(0.0_f32, f32::max)
        / 2.0
        + 1.0;
    let mut points = drawing
        .shapes
        .iter()
        .filter(|shape| shape.alpha > 0.0)
        .flat_map(|shape| shape.points.iter().copied());
    let first = points.next()?;
    let (x0, y0, x1, y1) = points.fold(
        (first.0, first.1, first.0, first.1),
        |(left, top, right, bottom), (x, y)| (left.min(x), top.min(y), right.max(x), bottom.max(y)),
    );
    Some((x0 - grow, y0 - grow, x1 + grow, y1 + grow))
}

/// A colour channel, 0 to 1, as a byte.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to 0..=255 first"
)]
fn byte(channel: f32) -> u32 {
    channel.mul_add(255.0, 0.0).clamp(0.0, 255.0).round() as u32
}

/// The ink as `0xRRGGBB` and its opacity.
fn ink(colour: gpui::Hsla) -> (u32, f32) {
    let rgba = gpui::Rgba::from(colour);
    let rgb = (byte(rgba.r) << 16) | (byte(rgba.g) << 8) | byte(rgba.b);
    (rgb, rgba.a)
}

/// The SVG of the shapes over the box `(x0, y0)`, `w × h` points.
fn svg(drawing: &Drawing, x0: f32, y0: f32, w: f32, h: f32, device: (i32, i32)) -> String {
    let (rgb, alpha) = ink(drawing.ink);
    let mut out = String::with_capacity(4096);
    let _infallible = write!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{x0} {y0} {w} {h}" width="{}" height="{}">"#,
        device.0, device.1
    );
    for shape in &drawing.shapes {
        if shape.alpha <= 0.0 {
            continue;
        }
        out.push_str("<path d=\"");
        for (i, (x, y)) in shape.points.iter().enumerate() {
            let _infallible = write!(out, "{}{x:.2},{y:.2}", if i == 0 { 'M' } else { 'L' });
        }
        if shape.closed {
            out.push('Z');
        }
        #[expect(clippy::float_arithmetic, reason = "the shape's share of the ink")]
        let opacity = (alpha * shape.alpha).clamp(0.0, 1.0);
        match shape.stroke {
            Some(width) => {
                let _infallible = write!(
                    out,
                    "\" fill=\"none\" stroke=\"#{rgb:06x}\" stroke-width=\"{width:.3}\" stroke-opacity=\"{opacity:.4}\" stroke-linejoin=\"round\" stroke-linecap=\"round\"/>"
                );
            }
            None => {
                let _infallible = write!(
                    out,
                    "\" fill=\"#{rgb:06x}\" fill-opacity=\"{opacity:.4}\"/>"
                );
            }
        }
    }
    out.push_str("</svg>");
    out
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

/// The device box the logical box lands on: origin rounded, far corner
/// rounded, so the raster and the box agree to the pixel.
pub(crate) fn device_box(x0: f32, y0: f32, x1: f32, y1: f32, scale: f32) -> (i32, i32, i32, i32) {
    let (left, top) = (whole(x0.mul_add(scale, 0.0)), whole(y0.mul_add(scale, 0.0)));
    let (right, bottom) = (whole(x1.mul_add(scale, 0.0)), whole(y1.mul_add(scale, 0.0)));
    (
        left,
        top,
        right.saturating_sub(left).max(1),
        bottom.saturating_sub(top).max(1),
    )
}

/// Paints `drawing`, rasterising only what lies inside `clip`: a backdrop's
/// lines may run far past the area it fills (the open session's frame
/// meets its apex thousands of points below the screen), and a raster of
/// the whole extent was measured at about 70 MB for one frame.
fn paint(drawing: &Drawing, clip: Option<Bounds<Pixels>>, window: &mut Window, cx: &mut App) {
    let Some((x0, y0, x1, y1)) = extent(drawing).and_then(|box_| clipped(box_, clip)) else {
        return;
    };
    let scale = window.scale_factor();
    let (left, top, width, height) = device_box(x0, y0, x1, y1, scale);
    let (lx, ly) = (
        f32::from(points(left, scale)),
        f32::from(points(top, scale)),
    );
    let (lw, lh) = (
        f32::from(points(width, scale)),
        f32::from(points(height, scale)),
    );
    let text = svg(drawing, lx, ly, lw, lh, (width, height));
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    let key = (hasher.finish(), width, height);
    let Some(raster) = raster(key, &text, window, cx) else {
        return;
    };
    let snapped = Bounds {
        origin: point(points(left, scale), points(top, scale)),
        size: size(points(width, scale), points(height, scale)),
    };
    if let Err(error) = window.paint_image(snapped, snapped, Corners::default(), raster, 0, false) {
        tracing::warn!(%error, "a drawing could not be painted");
    }
}

/// The bytes a raster holds: four a device pixel.
fn bytes((_, width, height): Key) -> u64 {
    let side = |n: i32| u64::try_from(n).unwrap_or(0);
    side(width).saturating_mul(side(height)).saturating_mul(4)
}

/// Which rasters stay (`keys` oldest first, the one just made last): the
/// newest [`KEPT`], of which at most [`LARGE_KEPT`] are [`LARGE`].
fn kept(keys: &[Key]) -> Vec<bool> {
    let (mut all, mut large) = (0_usize, 0_usize);
    let mut flags: Vec<bool> = keys
        .iter()
        .rev()
        .map(|&key| {
            let big = bytes(key) > LARGE;
            let keep = all < KEPT && (!big || large < LARGE_KEPT);
            if keep {
                all = all.saturating_add(1);
                large = large.saturating_add(usize::from(big));
            }
            keep
        })
        .collect();
    flags.reverse();
    flags
}

fn raster(key: Key, text: &str, window: &mut Window, cx: &mut App) -> Option<Arc<RenderImage>> {
    let found = RASTERS.with_borrow_mut(|rasters| {
        let at = rasters.iter().position(|(k, _)| *k == key)?;
        let hit = rasters.remove(at);
        let image = Arc::clone(&hit.1);
        rasters.push(hit);
        Some(image)
    });
    if found.is_some() {
        return found;
    }
    let renderer = cx.svg_renderer();
    let made = renderer.parse_svg(text.as_bytes()).and_then(|parsed| {
        renderer.render_parsed(
            &parsed,
            SvgSize::ExactSize(size(DevicePixels(key.1), DevicePixels(key.2))),
        )
    });
    match made {
        Ok(made) => {
            let evicted = RASTERS.with_borrow_mut(|rasters| {
                rasters.push((key, Arc::clone(&made)));
                let flags = kept(&rasters.iter().map(|(k, _)| *k).collect::<Vec<_>>());
                let (stay, gone): (Vec<_>, Vec<_>) = std::mem::take(rasters)
                    .into_iter()
                    .zip(flags)
                    .partition(|(_, keep)| *keep);
                *rasters = stay.into_iter().map(|(entry, _)| entry).collect();
                gone.into_iter()
                    .map(|((_, image), _)| image)
                    .collect::<Vec<_>>()
            });
            for image in evicted {
                cx.drop_image(image, Some(window));
            }
            Some(made)
        }
        Err(error) => {
            tracing::warn!(%error, "a drawing could not be rasterised");
            None
        }
    }
}

#[cfg(test)]
mod tests;
