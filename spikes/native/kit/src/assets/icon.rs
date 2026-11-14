//! The window's icon, drawn from the brand mark (split from `assets.rs`,
//! which holds the glyph inventory: the file budget, docs/QUALITY.md M3).

use super::{BRAND, MARK};

/// The window's icon (X11 reads it; Windows and macOS take theirs from the
/// executable and the bundle, the release track's): the mark in white on
/// a rounded tile of the brand colour, `size` pixels square.
///
/// # Errors
/// The mark could not be drawn (only a broken build could cause it).
pub fn window_icon(size: u32) -> Result<image::RgbaImage, IconError> {
    // The tile is drawn in the mark's own units, read from its view box
    // (the owner redraws the mark; nothing here assumes its numbers), the
    // mark inset by a tenth of its box on each side.
    let view = view_box(MARK).ok_or(IconError::Mark)?;
    let paths = MARK
        .split_once("<path")
        .map(|(_, rest)| format!("<path{rest}"))
        .ok_or(IconError::Mark)?
        .replace("currentColor", "#ffffff")
        .replace("</svg>", "");
    let tile = tile_svg(view, &paths);
    let tree = resvg::usvg::Tree::from_str(&tile, &resvg::usvg::Options::default())
        .map_err(|_bad| IconError::Mark)?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).ok_or(IconError::Size)?;
    let scale = size_scale(size, tree.size().width());
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    image::RgbaImage::from_raw(size, size, pixmap.take()).ok_or(IconError::Size)
}

/// A drawing's view box: x, y, width, height.
pub(super) fn view_box(svg: &str) -> Option<[f32; 4]> {
    let (_, rest) = svg.split_once("viewBox=\"")?;
    let (text, _) = rest.split_once('"')?;
    let mut numbers = text.split_whitespace().map(str::parse::<f32>);
    let mut next = || numbers.next()?.ok();
    Some([next()?, next()?, next()?, next()?])
}

/// The window icon's drawing: a rounded tile of the brand colour a fifth
/// larger than the mark's view box, the mark's `paths` on it.
#[expect(
    clippy::float_arithmetic,
    reason = "a view box's units are fractional; this is a drawing's geometry"
)]
fn tile_svg(view: [f32; 4], paths: &str) -> String {
    let [x, y, width, height] = view;
    let (inset_x, inset_y) = (width / 10.0, height / 10.0);
    let (left, top) = (x - inset_x, y - inset_y);
    let (wide, high) = (width + 2.0 * inset_x, height + 2.0 * inset_y);
    // The tile's corner, as the first icon's: 180 of 840.
    let corner = wide * 180.0 / 840.0;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{left} {top} {wide} {high}"><rect x="{left}" y="{top}" width="{wide}" height="{high}" rx="{corner}" fill="#{BRAND:06x}"/>{paths}</svg>"##
    )
}

/// How much to scale a drawing `width` wide to `size` pixels.
#[expect(
    clippy::cast_precision_loss,
    clippy::as_conversions,
    clippy::float_arithmetic,
    reason = "an icon's size is a few hundred pixels: exact in f32, and the ratio is what resvg takes"
)]
fn size_scale(size: u32, width: f32) -> f32 {
    size as f32 / width
}

/// The icon could not be made.
#[derive(Debug, thiserror::Error)]
pub enum IconError {
    /// The mark's file is not as expected.
    #[error("the mark could not be read")]
    Mark,
    /// The size is not a drawable one.
    #[error("not a drawable icon size")]
    Size,
}
