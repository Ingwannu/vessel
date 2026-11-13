//! The kit's own drawings, served to GPUI as assets: the window controls'
//! four glyphs, as SVG paths on a 10 by 10 grid (Windows 11's shapes:
//! a line, a square, two offset squares, a cross).
//!
//! Embedded as text, not files: four short paths need no bundle, and an
//! app built from the kit needs no asset directory to draw its chrome.

use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

/// Vessel's mark, one colour (`fill="currentColor"`), embedded from the
/// design system's own file by path (`docs/design/brand/`, the owner's
/// paths, never copied): GPUI draws an SVG as a mask in the element's
/// text colour, so the one file serves every colour the brand README
/// asks for.
const MARK: &str = include_str!("../../../../docs/design/brand/vessel-mark-mono.svg");

/// The brand colour (`docs/design/brand/README.md`): the mark's colour on
/// light surfaces and its icon's tile; never the interface's.
pub const BRAND: u32 = 0x001a_5868;

/// A glyph the kit draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// Vessel's mark.
    Mark,
    /// The window's minimise button.
    Minimize,
    /// Maximise.
    Maximize,
    /// Restore, when maximised.
    Restore,
    /// Close.
    Close,
    /// An update: an arrow down into a tray (the title bar's update
    /// status at a phone's width).
    Update,
}

/// Every glyph, for the asset source.
const ALL: [Glyph; 6] = [
    Glyph::Mark,
    Glyph::Minimize,
    Glyph::Maximize,
    Glyph::Restore,
    Glyph::Close,
    Glyph::Update,
];

impl Glyph {
    /// Its asset path.
    #[must_use]
    pub const fn path(self) -> &'static str {
        match self {
            Self::Mark => "kit/mark.svg",
            Self::Minimize => "kit/minimize.svg",
            Self::Maximize => "kit/maximize.svg",
            Self::Restore => "kit/restore.svg",
            Self::Close => "kit/close.svg",
            Self::Update => "kit/update.svg",
        }
    }

    const fn svg(self) -> &'static str {
        match self {
            Self::Mark => MARK,
            Self::Minimize => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0 5.5h10" stroke="black" stroke-width="1"/></svg>"#
            }
            Self::Maximize => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="black" stroke-width="1"/></svg>"#
            }
            Self::Restore => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect x="0.5" y="2.5" width="7" height="7" fill="none" stroke="black" stroke-width="1"/><path d="M2.5 2.5V0.5h7v7h-2" fill="none" stroke="black" stroke-width="1"/></svg>"#
            }
            Self::Close => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0.5 0.5l9 9M9.5 0.5l-9 9" stroke="black" stroke-width="1"/></svg>"#
            }
            Self::Update => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M6 1.5v6.5M3.2 5.4L6 8.2l2.8-2.8M1.5 10.5h9" fill="none" stroke="black" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>"#
            }
        }
    }
}

/// The kit's assets, for `Application::with_assets`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let glyph = ALL.into_iter().find(|glyph| glyph.path() == path);
        Ok(glyph.map(|glyph| Cow::Borrowed(glyph.svg().as_bytes())))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(ALL
            .into_iter()
            .map(Glyph::path)
            .filter(|asset| asset.starts_with(path))
            .map(SharedString::from)
            .collect())
    }
}

/// The window's icon (X11 reads it; Windows and macOS take theirs from the
/// executable and the bundle, the release track's): the mark in white on
/// a rounded tile of the brand colour, `size` pixels square.
///
/// # Errors
/// The mark could not be drawn (only a broken build could cause it).
pub fn window_icon(size: u32) -> Result<image::RgbaImage, IconError> {
    // The mark's own view box is 700 wide from (649, 650): the tile is
    // drawn in the same units, the mark inset by a tenth on each side.
    let paths = MARK
        .split_once("<path")
        .map(|(_, rest)| format!("<path{rest}"))
        .ok_or(IconError::Mark)?
        .replace("currentColor", "#ffffff")
        .replace("</svg>", "");
    let tile = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="579 580 840 840"><rect x="579" y="580" width="840" height="840" rx="180" fill="#{BRAND:06x}"/>{paths}</svg>"##
    );
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
