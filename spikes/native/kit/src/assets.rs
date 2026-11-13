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
    /// Three lines: opens the sidebar's sheet at a phone's width.
    Menu,
    /// A picker's caret.
    ChevronDown,
    /// A row that opens a list of its own (a provider's models).
    ChevronRight,
    /// Back, from such a list.
    ChevronLeft,
    /// The chosen row's mark.
    Check,
    /// A search row.
    Search,
    /// Send.
    ArrowUp,
    /// Stop.
    Stop,
    /// The permissions mode.
    Shield,
    /// The effort (thinking).
    Brain,
    /// An agent.
    Bot,
    /// A failure.
    Alert,
    /// A folder.
    Folder,
    /// A machine.
    Machine,
    /// Three dots: a menu of more actions (the Session actions).
    More,
    /// An agent's feature, and the features (Paseo's `Settings2`: two
    /// sliders).
    Sliders,
    /// Settings (Paseo's `Settings`: a gear).
    Gear,
}

/// Every glyph, for the asset source.
const ALL: [Glyph; 23] = [
    Glyph::Mark,
    Glyph::Minimize,
    Glyph::Maximize,
    Glyph::Restore,
    Glyph::Close,
    Glyph::Update,
    Glyph::Menu,
    Glyph::ChevronDown,
    Glyph::ChevronRight,
    Glyph::ChevronLeft,
    Glyph::Check,
    Glyph::Search,
    Glyph::ArrowUp,
    Glyph::Stop,
    Glyph::Shield,
    Glyph::Brain,
    Glyph::Bot,
    Glyph::Alert,
    Glyph::Folder,
    Glyph::Machine,
    Glyph::More,
    Glyph::Sliders,
    Glyph::Gear,
];

/// A line glyph on Lucide's grid (24 units, a stroke of 1.5, round ends),
/// as Paseo's icons are drawn; the paths are the kit's own.
macro_rules! line_glyph {
    ($body:literal) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">"#,
            $body,
            "</svg>"
        )
    };
}

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
            Self::Menu => "kit/menu.svg",
            Self::ChevronDown => "kit/chevron-down.svg",
            Self::ChevronRight => "kit/chevron-right.svg",
            Self::ChevronLeft => "kit/chevron-left.svg",
            Self::Check => "kit/check.svg",
            Self::Search => "kit/search.svg",
            Self::ArrowUp => "kit/arrow-up.svg",
            Self::Stop => "kit/stop.svg",
            Self::Shield => "kit/shield.svg",
            Self::Brain => "kit/brain.svg",
            Self::Bot => "kit/bot.svg",
            Self::Alert => "kit/alert.svg",
            Self::Folder => "kit/folder.svg",
            Self::Machine => "kit/machine.svg",
            Self::More => "kit/more.svg",
            Self::Sliders => "kit/sliders.svg",
            Self::Gear => "kit/gear.svg",
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
            Self::Menu => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M1.5 3h9M1.5 6h9M1.5 9h9" fill="none" stroke="black" stroke-width="1.2" stroke-linecap="round"/></svg>"#
            }
            Self::ChevronDown => line_glyph!(r#"<path d="M6 9l6 6 6-6"/>"#),
            Self::ChevronRight => line_glyph!(r#"<path d="M9 6l6 6-6 6"/>"#),
            Self::ChevronLeft => line_glyph!(r#"<path d="M15 6l-6 6 6 6"/>"#),
            Self::Check => line_glyph!(r#"<path d="M20 6L9 17l-5-5"/>"#),
            Self::Search => {
                line_glyph!(r#"<circle cx="11" cy="11" r="7"/><path d="M20 20l-3.6-3.6"/>"#)
            }
            Self::ArrowUp => line_glyph!(r#"<path d="M12 19V5M5 12l7-7 7 7"/>"#),
            Self::Stop => line_glyph!(r#"<rect x="6" y="6" width="12" height="12" rx="1.5"/>"#),
            Self::Shield => {
                line_glyph!(r#"<path d="M12 3l7 3v5.5c0 4.6-3 8.1-7 9.5-4-1.4-7-4.9-7-9.5V6z"/>"#)
            }
            Self::Brain => line_glyph!(
                r#"<path d="M12 5.5a3 3 0 0 0-5.6-1.2A3 3 0 0 0 4 8.5a3.2 3.2 0 0 0 .8 5.8A3 3 0 0 0 9 19a3 3 0 0 0 3-1.5z"/><path d="M12 5.5a3 3 0 0 1 5.6-1.2A3 3 0 0 1 20 8.5a3.2 3.2 0 0 1-.8 5.8A3 3 0 0 1 15 19a3 3 0 0 1-3-1.5"/>"#
            ),
            Self::Bot => line_glyph!(
                r#"<rect x="4.5" y="8" width="15" height="11" rx="2.5"/><path d="M12 8V4.5M9.5 12.5v1.5M14.5 12.5v1.5"/>"#
            ),
            Self::Alert => {
                line_glyph!(r#"<path d="M12 4l9 16H3z"/><path d="M12 10v4M12 17v.01"/>"#)
            }
            Self::Folder => line_glyph!(
                r#"<path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2h7.5A2.5 2.5 0 0 1 21 9.5v7a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 16.5z"/>"#
            ),
            Self::Machine => line_glyph!(
                r#"<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>"#
            ),
            Self::More => line_glyph!(
                r#"<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>"#
            ),
            Self::Sliders => line_glyph!(
                r#"<path d="M19.5 7H10M14 17H4.5"/><circle cx="7" cy="7" r="2.75"/><circle cx="17" cy="17" r="2.75"/>"#
            ),
            Self::Gear => line_glyph!(
                r#"<circle cx="12" cy="12" r="3"/><path d="M12 2.75v2.5M12 18.75v2.5M2.75 12h2.5M18.75 12h2.5M5.46 5.46l1.77 1.77M16.77 16.77l1.77 1.77M5.46 18.54l1.77-1.77M16.77 7.23l1.77-1.77"/><circle cx="12" cy="12" r="6.25"/>"#
            ),
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
fn view_box(svg: &str) -> Option<[f32; 4]> {
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

#[cfg(test)]
mod tests {
    use super::{MARK, view_box, window_icon};

    #[test]
    fn the_icon_is_drawn_from_the_marks_own_view_box() {
        assert_eq!(view_box(MARK), Some([170.0, 170.0, 1660.0, 1660.0]));
        let icon = window_icon(64).unwrap();
        // The tile's rounded corner is clear; its edge, inside the inset,
        // is the brand colour.
        assert_eq!(icon.get_pixel(0, 0).0[3], 0);
        let edge = icon.get_pixel(32, 2).0;
        assert_eq!((edge[0], edge[1], edge[2]), (0x1a, 0x58, 0x68));
    }
}
