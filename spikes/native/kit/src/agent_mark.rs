//! Official artwork, isolated from node identities and the view model.
//!
//! [Decision Log]
//! - Purpose: identify agents with their owners' artwork, never redraws.
//! - Constraint: SVG masks lose official colours; remote assets are untrusted.
//! - Alternatives: runtime downloads or a generic bot for every provider.
//! - Choice: reviewed embedded paths, each rasterised at the display's own
//!   pixels by [`raster`] (why not GPUI's `img()`: there). Brand minimum
//!   sizes and clear space win over the requested slot size.
//! - Cost: compact `OpenCode` slots use the bot; see ADR 0048.
mod paths;
mod raster;
mod settings;
pub use settings::{SettingsRowEdge, settings_mark};

use crate::{Tokens, assets::Glyph, glyph, theme::Scheme};
use gpui::{Div, ParentElement as _, Styled as _, div, px};

/// Logical icon size. Display scaling is the platform's responsibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    /// Inline metadata.
    Twelve,
    /// Compact label.
    Fourteen,
    /// Picker row.
    Sixteen,
    /// Touch toolbar.
    Twenty,
}
impl Size {
    /// All sizes required by the visual comparison.
    pub const ALL: [Self; 4] = [Self::Twelve, Self::Fourteen, Self::Sixteen, Self::Twenty];
    /// Logical points.
    #[must_use]
    pub const fn points(self) -> f32 {
        match self {
            Self::Twelve => 12.0,
            Self::Fourteen => 14.0,
            Self::Sixteen => 16.0,
            Self::Twenty => 20.0,
        }
    }
}
/// Available room, without increasing the surface's existing row height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    /// Inline metadata or a 28-point toolbar.
    Compact,
    /// A 36-point picker row.
    Row,
    /// A wide attribution or comparison slot.
    Lockup,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Geometry {
    Spark,
    Blossom,
    Sparkle,
    OpenCode,
    Company,
}
/// Reviewed embedded bytes, never a URL supplied by the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Image {
    /// The name its rasters are cached by; not loaded through GPUI's
    /// asset source.
    path: &'static str,
    bytes: &'static [u8],
}
/// Reviewed artwork. Its private construction prevents a node from supplying
/// arbitrary SVG or a URL. Identities and their mapping belong to the view;
/// the app only adapts that result to these reviewed constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Artwork {
    light: Image,
    dark: Image,
    geometry: Geometry,
}
macro_rules! vector {
    ($(#[$meta:meta])* $name:ident, $path:literal, $light:ident, $dark:ident, $geometry:ident) => {
        $(#[$meta])*
        pub const $name: Artwork = Artwork {
            light: Image { path: concat!("kit/agent-marks/", $path, "-light.svg"), bytes: paths::$light.as_bytes() },
            dark: Image { path: concat!("kit/agent-marks/", $path, "-dark.svg"), bytes: paths::$dark.as_bytes() },
            geometry: Geometry::$geometry,
        };
    };
}
// The inventory is fixed at build time so asset lookup neither allocates nor
// parses user data; each monochrome alternative is an approved brand form.
vector!(/// Anthropic's official Clay Spark, with reasonable clear space.
CLAUDE, "claude", CLAUDE, CLAUDE, Spark);
vector!(/// `OpenAI`'s approved black/white Blossom including its original inset.
CODEX, "codex", CODEX_LIGHT, CODEX_DARK, Blossom);
vector!(/// `OpenCode`'s official two-tone mark with its required clear space.
OPENCODE, "opencode", OPENCODE_LIGHT, OPENCODE_DARK, OpenCode);
vector!(/// GitHub's current bare Invertocat with official clear space.
GITHUB, "github", GITHUB_LIGHT, GITHUB_DARK, Company);
/// Copilot uses its vendor's mark beside the explicit product name; the
/// forge control shares exactly these bytes, not a second icon copy.
pub const COPILOT: Artwork = GITHUB;
vector!(/// Google's Gemini spark in its own gradient, with Google's inset.
GEMINI, "gemini", GEMINI, GEMINI, Sparkle);
#[cfg(test)]
const ALL: [Artwork; 5] = [CLAUDE, CODEX, GEMINI, OPENCODE, COPILOT];
impl Artwork {
    const fn image(self, scheme: Scheme) -> Image {
        match scheme {
            Scheme::Light => self.light,
            Scheme::Dark => self.dark,
        }
    }
    fn image_points(self, size: Size) -> Option<f32> {
        // Match the reference's ink, not equal outer image dimensions.
        // Vendor insets/clear space remain inside these unaltered SVGs.
        // See the measured path bounds in the source/rights record.
        match self.geometry {
            Geometry::Spark => Some(size.points().mul_add(1.506_410_2, 0.0)),
            Geometry::Blossom => Some(size.points().mul_add(1.377_5, 0.0)),
            Geometry::Company => Some(size.points().mul_add(1.333_333_4, 0.0)),
            // 192-unit icon, 176-unit spark: the reference's spark fills
            // its 24-unit box edge to edge.
            Geometry::Sparkle => Some(size.points().mul_add(1.090_909_1, 0.0)),
            Geometry::OpenCode => None,
        }
    }
    /// The unchanged image viewport including its clear space. This lets
    /// audit fixtures project a vendor original at the exact same scale;
    /// application surfaces use [`agent_mark`] instead of drawing assets.
    #[must_use]
    pub fn image_size(self, size: Size) -> gpui::Size<gpui::Pixels> {
        self.image_points(size)
            .map_or(gpui::size(px(32.0), px(36.0)), |points| {
                gpui::size(px(points), px(points))
            })
    }
    fn dimensions(self, size: Size, space: Space) -> Option<(f32, f32)> {
        match self.geometry {
            Geometry::Spark | Geometry::Blossom | Geometry::Sparkle | Geometry::Company => {
                let image = self.image_points(size).unwrap_or(size.points());
                if space == Space::Compact && image > 28.0 {
                    return None;
                }
                let width = match space {
                    Space::Row => 32.0,
                    Space::Compact | Space::Lockup => image,
                };
                // The clear-space image can occupy the existing vertical
                // padding, as OpenCode does, without changing row pitch.
                Some((width, size.points()))
            }
            Geometry::OpenCode => match space {
                Space::Compact => None,
                Space::Row | Space::Lockup => Some((32.0, 36.0)),
            },
        }
    }
}
/// Shared mark control: unknown artwork or an insufficient slot draws the bot.
#[must_use]
pub fn agent_mark(artwork: Option<Artwork>, size: Size, space: Space, tokens: &Tokens) -> Div {
    render(
        artwork,
        size,
        space,
        tokens,
        tokens.neutral.foreground_muted,
        None,
    )
}
/// The mark in the muted ink, where a list shows many (Paseo's session
/// list draws its provider icons in `foregroundMuted`); the colours stay
/// in the pickers and Settings.
#[must_use]
pub fn agent_mark_muted(
    artwork: Option<Artwork>,
    size: Size,
    space: Space,
    tokens: &Tokens,
) -> Div {
    render(
        artwork,
        size,
        space,
        tokens,
        tokens.neutral.foreground_muted,
        Some(tokens.neutral.foreground_muted),
    )
}
fn render(
    artwork: Option<Artwork>,
    size: Size,
    space: Space,
    tokens: &Tokens,
    fallback: gpui::Hsla,
    tint: Option<gpui::Hsla>,
) -> Div {
    if let Some((art, (width, height))) = artwork.and_then(|art| {
        art.dimensions(size, space)
            .map(|dimensions| (art, dimensions))
    }) {
        let image = art.image_size(size);
        div()
            .w(px(width))
            .h(px(height))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(raster::exact(art.image(tokens.scheme), image, tint))
    } else {
        let fallback_size = match space {
            Space::Row => 16.0,
            Space::Compact | Space::Lockup => size.points(),
        };
        let width = match space {
            Space::Row => 32.0,
            Space::Compact | Space::Lockup => fallback_size,
        };
        div()
            .w(px(width))
            .h(px(fallback_size))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(glyph(Glyph::Bot, fallback_size, fallback))
    }
}
/// A forge with an official embedded mark. No node identities enter the kit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forge {
    /// GitHub, sharing the same vendor artwork as the Copilot identifier.
    GitHub,
}
/// A forge's typed use of the shared official vendor artwork.
#[must_use]
pub fn forge_mark(forge: Forge, size: Size, tokens: &Tokens) -> Div {
    match forge {
        Forge::GitHub => agent_mark(Some(GITHUB), size, Space::Compact, tokens),
    }
}

/// Every embedded image, once per name (a mark whose two schemes share
/// bytes keeps two names), for the tests.
#[cfg(test)]
fn images() -> impl Iterator<Item = Image> {
    ALL.into_iter().flat_map(|art| [art.light, art.dark])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn brand_geometry_wins_over_small_slots() {
        for size in Size::ALL {
            assert_eq!(OPENCODE.dimensions(size, Space::Compact), None);
            assert_eq!(OPENCODE.dimensions(size, Space::Row), Some((32.0, 36.0)));
            assert_eq!(
                COPILOT.dimensions(size, Space::Row),
                Some((32.0, size.points()))
            );
            assert_eq!(
                COPILOT.dimensions(size, Space::Lockup),
                Some((COPILOT.image_points(size).unwrap(), size.points()))
            );
        }
    }
    /// The outline of the first clipped group (Gemini's spark).
    fn clipped_outline(tree: &resvg::usvg::Tree) -> resvg::usvg::Rect {
        tree.root()
            .children()
            .iter()
            .find_map(|node| {
                if let resvg::usvg::Node::Group(group) = node {
                    group.clip_path()
                } else {
                    None
                }
            })
            .map(|clip| clip.root().abs_bounding_box())
            .expect("a clipped group")
    }

    #[test]
    fn approved_viewports_yield_the_measured_reference_ink() {
        // Width cannot also match when official aspect ratios differ from
        // the reference's redraws; preserve geometry and record that delta.
        // Use true cubic extrema: a renderer's approximate ink rectangle
        // can omit the Invertocat foot's interior vertical maximum.
        for (art, expected) in [
            (CLAUDE, (16.0_f32, 16.0_f32)),
            (CODEX, (14.80, 14.67)),
            (GITHUB, (16.0, 15.50)),
            (GEMINI, (16.0, 16.0)),
        ] {
            let svg = std::str::from_utf8(art.image(Scheme::Light).bytes).unwrap();
            let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).unwrap();
            // Gemini's colour field is an image wider than the spark, cut
            // by the spark's outline: its ink is the clip's outline.
            let bounds = if svg == paths::GEMINI {
                clipped_outline(&tree)
            } else {
                tree.root().abs_bounding_box()
            };
            let factor = f32::from(art.image_size(Size::Sixteen).width)
                .mul_add(tree.size().width().recip(), 0.0);
            assert!(
                bounds
                    .width()
                    .mul_add(factor, expected.0.mul_add(-1.0, 0.0))
                    .abs()
                    < 0.02
            );
            assert!(
                bounds
                    .height()
                    .mul_add(factor, expected.1.mul_add(-1.0, 0.0))
                    .abs()
                    < 0.02
            );
        }
        assert_eq!(CLAUDE.dimensions(Size::Twenty, Space::Compact), None);
    }
    #[test]
    fn opencode_mask_is_not_ink_and_its_quiet_zone_stays_transparent() {
        for scheme in [Scheme::Light, Scheme::Dark] {
            let data = OPENCODE.image(scheme).bytes;
            let svg = std::str::from_utf8(data).unwrap();
            let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).unwrap();
            let mut pixels = resvg::tiny_skia::Pixmap::new(32, 36).unwrap();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::from_scale(0.066_666_67, 0.066_666_67),
                &mut pixels.as_mut(),
            );
            assert_eq!(pixels.pixel(0, 0).unwrap().alpha(), 0);
            assert_eq!(pixels.pixel(7, 18).unwrap().alpha(), 0);
            assert_eq!(
                pixels.pixel(16, 14).unwrap().alpha(),
                0,
                "a mask path must not fill the open counter"
            );
            assert_eq!(pixels.pixel(9, 9).unwrap().alpha(), 255);
        }
    }

    /// A name is the raster cache's key: one name, one drawing.
    #[test]
    fn each_image_name_stands_for_one_drawing() {
        let names: std::collections::BTreeMap<&str, &[u8]> =
            images().map(|image| (image.path, image.bytes)).collect();
        for image in images() {
            assert_eq!(names.get(image.path), Some(&image.bytes));
        }
        assert_eq!(names.len(), 10);
        assert_eq!(GITHUB, COPILOT);
    }

    /// Every embedded drawing is a vector that parses, with nothing in it
    /// that reaches outside: no script and no link. The one raster is
    /// Gemini's colour field, inside Google's file as a `data:` JPEG under
    /// a vector outline (`paths::GEMINI`); no other drawing has one.
    #[test]
    fn embedded_drawings_are_inert_vectors() {
        for image in images() {
            let svg = std::str::from_utf8(image.bytes).unwrap();
            assert!(svg.starts_with("<svg "), "{}", image.path);
            assert!(!svg.contains("<script"));
            let gemini = svg == paths::GEMINI;
            let links = svg.matches("href=").count();
            assert_eq!(links, usize::from(gemini), "{}", image.path);
            assert_eq!(svg.matches("<image").count(), usize::from(gemini));
            if gemini {
                assert_eq!(svg.matches("href=\"data:image/jpeg;base64,").count(), 1);
            }
            let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).unwrap();
            assert!(!tree.root().children().is_empty());
        }
    }

    /// Gemini is drawn in Google's four colours, not as a one-colour mask:
    /// red at the top arm, blue at the right, green at the bottom, yellow
    /// at the left (the embedded colour field is rendered).
    #[test]
    fn the_gemini_spark_keeps_its_four_colours() {
        let svg = std::str::from_utf8(GEMINI.image(Scheme::Dark).bytes).unwrap();
        let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).unwrap();
        let mut pixels = resvg::tiny_skia::Pixmap::new(192, 192).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixels.as_mut(),
        );
        let at = |x, y| pixels.pixel(x, y).unwrap().demultiply();
        let (top, right, bottom, left) = (at(96, 30), at(160, 96), at(96, 160), at(30, 96));
        for arm in [top, right, bottom, left] {
            assert_eq!(arm.alpha(), 255);
        }
        assert!(top.red() > top.green() && top.red() > top.blue(), "red top");
        assert!(
            right.blue() > right.red() && right.blue() > right.green(),
            "blue right"
        );
        assert!(
            bottom.green() > bottom.red() && bottom.green() > bottom.blue(),
            "green bottom"
        );
        assert!(
            left.red() > left.blue() && left.green() > left.blue(),
            "yellow left"
        );
        assert_eq!(pixels.pixel(4, 4).unwrap().alpha(), 0, "Google's inset");
    }
}
