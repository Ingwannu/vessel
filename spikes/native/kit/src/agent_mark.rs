//! Official artwork, isolated from node identities and the view model.
//!
//! [Decision Log]
//! - Purpose: identify agents with their owners' artwork, never redraws.
//! - Constraint: SVG masks lose official colours; remote assets are untrusted.
//! - Alternatives: runtime downloads or a generic bot for every provider.
//! - Choice: reviewed embedded paths through the existing image renderer.
//!   Brand minimum sizes and clear space win over the requested slot size.
//! - Cost: compact `OpenCode` slots use the bot; see ADR 0048.
mod paths;
mod settings;
pub use settings::{SettingsRowEdge, settings_mark};

use crate::{Tokens, assets::Glyph, glyph, theme::Scheme};
use gpui::{Div, ParentElement as _, Styled as _, div, img, px};

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
    Plate,
    OpenCode,
    Company,
}
/// Reviewed embedded bytes, never a URL supplied by the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Image {
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
/// Google Gemini CLI's exact official PNG; never a traced redraw.
pub const GEMINI: Artwork = Artwork {
    light: GEMINI_IMAGE,
    dark: GEMINI_IMAGE,
    geometry: Geometry::Plate,
};
const GEMINI_IMAGE: Image = Image {
    path: "kit/agent-marks/gemini.png",
    bytes: include_bytes!("agent_mark/gemini-cli.png"),
};
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
            Geometry::Plate => Some(size.points()),
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
            Geometry::Spark | Geometry::Blossom | Geometry::Plate | Geometry::Company => {
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
    )
}
fn render(
    artwork: Option<Artwork>,
    size: Size,
    space: Space,
    tokens: &Tokens,
    fallback: gpui::Hsla,
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
            .child(
                img(art.image(tokens.scheme).path)
                    .flex_none()
                    .w(image.width)
                    .h(image.height),
            )
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

/// `AssetSource` accepts only reviewed embedded names.
pub(crate) fn load(path: &str) -> Option<&'static [u8]> {
    ALL.into_iter()
        .flat_map(|art| [art.light, art.dark])
        .find(|image| image.path == path)
        .map(|image| image.bytes)
}
pub(crate) fn asset_paths() -> impl Iterator<Item = &'static str> {
    ALL.into_iter()
        .flat_map(|art| {
            [
                Some(art.light.path),
                (art.light.path != art.dark.path).then_some(art.dark.path),
            ]
        })
        .flatten()
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
        ] {
            let svg = std::str::from_utf8(art.image(Scheme::Light).bytes).unwrap();
            let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).unwrap();
            let bounds = tree.root().abs_bounding_box();
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
        assert_eq!(
            GEMINI.image_size(Size::Sixteen),
            gpui::size(px(16.0), px(16.0))
        );
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

    #[test]
    fn one_asset_name_is_listed_once_even_when_both_schemes_share_a_png() {
        let paths: Vec<&str> = asset_paths().collect();
        let unique: std::collections::BTreeSet<&str> = paths.iter().copied().collect();
        assert_eq!(paths.len(), unique.len());
        assert_eq!(paths.len(), 9);
        assert_eq!(GITHUB, COPILOT);
    }

    #[test]
    fn embedded_vectors_are_parseable_and_remote_paths_are_rejected() {
        for path in asset_paths() {
            let data = load(path).unwrap();
            if data.starts_with(b"\x89PNG\r\n\x1a\n") {
                let icon = image::load_from_memory(data).unwrap();
                assert_eq!((icon.width(), icon.height()), (1645, 1645));
                continue;
            }
            let svg = std::str::from_utf8(data).unwrap();
            assert!(!svg.contains("<script"));
            assert!(!svg.contains("href="));
            assert!(!svg.contains("<image"));
            let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).unwrap();
            assert!(!tree.root().children().is_empty());
        }
        assert_eq!(load("https://example.invalid/logo.svg"), None);
        assert_eq!(load("kit/agent-marks/unknown-light.svg"), None);
    }
}
