//! The kit's drawings, served to GPUI as assets: window controls on their
//! 10 by 10 grid and shared UI line glyphs on a 24 by 24 grid. Canonical
//! panel vectors retain their Lucide/Feather notices in the brand docs.
//!
//! Embedded as text, not files: the kit needs no separate asset bundle.
//! The inventory (the enum, its list and its paths) is declared once in
//! assets/inventory.rs and generated here by one macro. The
//! drawings are in `drawing.rs`, one arm per glyph: the two stayed in one
//! file until the git row's glyphs took it past its listed size
//! (docs/QUALITY.md M3). A registration still cannot miss its vector: the
//! drawing's match has no catch-all.

use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

mod drawing;
mod icon;

#[cfg(test)]
use icon::view_box;
pub use icon::{IconError, window_icon};

/// Vessel's mark, one colour (`fill="currentColor"`), embedded from the
/// design system's own file by path (`docs/design/brand/`, the owner's
/// paths, never copied): GPUI draws an SVG as a mask in the element's
/// text colour, so the one file serves every colour the brand README
/// asks for.
const MARK: &str = include_str!("../../../../docs/design/brand/vessel-mark-mono.svg");

/// The brand colour (`docs/design/brand/README.md`): the mark's colour on
/// light surfaces and its icon's tile; never the interface's.
pub const BRAND: u32 = 0x001a_5868;

// One declaration owns the public enum, stable iteration order and paths.
// Keeping these together makes a new glyph compiler-checked without duplicating
// three inventories or permitting a missing-path fallback.
macro_rules! glyph_inventory {
    ($( $(#[$doc:meta])* $name:ident => $path:literal, )+) => {
        /// A glyph the kit draws.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Glyph {
            $( $(#[$doc])* $name, )+
        }

        const ALL: [Glyph; [$(stringify!($name)),+].len()] = [$(Glyph::$name),+];

        impl Glyph {
            /// Its asset path.
            #[must_use]
            pub const fn path(self) -> &'static str {
                match self { $(Self::$name => $path,)+ }
            }
        }
    };
}

include!("assets/inventory.rs");

/// Where a line glyph is served at the command centre's row stroke.
const COMMAND_WEIGHT: &str = "kit/command-weight/";
/// The root every `line_glyph!` drawing starts with: Lucide's grid at the
/// provider's 1.5.
const LINE_ROOT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">"#;
const PROVIDER_STROKE: &str = r#"stroke-width="1.5""#;
const COMMAND_STROKE: &str = r#"stroke-width="2.2""#;

impl Glyph {
    /// This glyph as a command-centre row draws it.
    ///
    /// [Decision Log]
    /// - Purpose: every glyph in Paseo's command centre is drawn at stroke
    ///   2.2 (its Plus at 2.4; `root-registration.tsx` 65-107); Vessel's
    ///   command centre lists more rows than Paseo's (machines, the board,
    ///   each settings section, sessions, themes), and they must match
    ///   (coordinator, TRACKER 778, 2026-11-16).
    /// - Alternatives: a `Command*` role per glyph (two dozen copies of the
    ///   same paths that would drift from their shared roles); restroking the
    ///   shared roles (every other surface would turn heavy).
    /// - Choice: a line glyph (drawn by `line_glyph!` at 1.5) is also served
    ///   under `kit/command-weight/` with its root stroke at 2.2; the kit's
    ///   command row asks for this path. The `Command*` roles (Paseo's own root
    ///   shapes, already at their stroke) and glyphs that are not 1.5 line
    ///   drawings keep their own path.
    #[must_use]
    pub fn command_weight_path(self) -> SharedString {
        match self.path().strip_prefix("kit/") {
            Some(file) if self.svg().starts_with(LINE_ROOT) => {
                SharedString::from(format!("{COMMAND_WEIGHT}{file}"))
            }
            Some(_) | None => SharedString::new_static(self.path()),
        }
    }
}

/// The kit's assets, for `Application::with_assets`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        // The agents' marks are not here: `agent_mark` rasterises its own
        // embedded drawings at the display's pixels.
        if let Some(file) = path.strip_prefix(COMMAND_WEIGHT) {
            let line = ALL.into_iter().find(|glyph| {
                glyph.path().strip_prefix("kit/") == Some(file)
                    && glyph.svg().starts_with(LINE_ROOT)
            });
            return Ok(line.map(|glyph| {
                Cow::Owned(
                    glyph
                        .svg()
                        .replacen(PROVIDER_STROKE, COMMAND_STROKE, 1)
                        .into_bytes(),
                )
            }));
        }
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

#[cfg(test)]
mod tests {
    use super::{Assets, Glyph, MARK, view_box, window_icon};
    use gpui::AssetSource as _;

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

    #[test]
    fn calendar_clock_is_registered_and_renders_unclipped_at_24() {
        let glyph = Glyph::CalendarClock;
        assert_eq!(view_box(glyph.svg()), Some([0.0, 0.0, 24.0, 24.0]));
        assert_eq!(
            Assets.list(glyph.path()).unwrap(),
            [gpui::SharedString::from("kit/calendar-clock.svg")]
        );
        let bytes = Assets.load(glyph.path()).unwrap().unwrap();
        let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
        let mut pixels = resvg::tiny_skia::Pixmap::new(24, 24).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixels.as_mut(),
        );
        let image = image::RgbaImage::from_raw(24, 24, pixels.take()).unwrap();
        assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
        for at in 0..24 {
            for (x, y) in [(at, 0), (at, 23), (0, at), (23, at)] {
                assert_eq!(image.get_pixel(x, y).0[3], 0);
            }
        }
    }

    #[test]
    fn archive_and_git_glyphs_are_registered_and_unclipped() {
        for glyph in [
            Glyph::Trash2,
            Glyph::ArchiveRestore,
            Glyph::Minus,
            Glyph::GitBranchPlus,
            Glyph::Commit,
            Glyph::Undo2,
            Glyph::Split,
        ] {
            assert_eq!(view_box(glyph.svg()), Some([0.0, 0.0, 24.0, 24.0]));
            let bytes = Assets.load(glyph.path()).unwrap().unwrap();
            let tree =
                resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
            let mut pixels = resvg::tiny_skia::Pixmap::new(24, 24).unwrap();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::identity(),
                &mut pixels.as_mut(),
            );
            let image = image::RgbaImage::from_raw(24, 24, pixels.take()).unwrap();
            assert!(image.pixels().any(|pixel| pixel.0[3] > 0), "{glyph:?}");
            for at in 0..24 {
                for (x, y) in [(at, 0), (at, 23), (0, at), (23, at)] {
                    assert_eq!(image.get_pixel(x, y).0[3], 0, "{glyph:?}");
                }
            }
        }
        assert_eq!(Glyph::Commit.path(), "kit/commit.svg");
    }
}

#[cfg(test)]
mod header_glyph_tests {
    use super::{Assets, Glyph, view_box};
    use gpui::AssetSource as _;

    #[test]
    fn compact_menu_has_the_reference_painted_extents_and_gaps_at_16_by_12() {
        assert_eq!(
            view_box(Glyph::MenuCompact.svg()),
            Some([0.0, 0.0, 16.0, 12.0])
        );
        let bytes = Assets.load(Glyph::MenuCompact.path()).unwrap().unwrap();
        let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
        let boxes: Vec<_> = tree
            .root()
            .children()
            .iter()
            .map(|node| {
                let rect = node.abs_bounding_box();
                // usvg's rounded-rect cubic extremum at x=0 can be
                // -2.75e-32. Normalize only sub-EPSILON zero-origin noise;
                // all sizes/y positions and native raster gaps stay exact.
                let x = if rect.x().abs() < f32::EPSILON {
                    0.0
                } else {
                    rect.x()
                };
                [x, rect.y(), rect.width(), rect.height()]
            })
            .collect();
        assert_eq!(
            boxes,
            [
                [0.0, 0.0, 16.0, 1.5],
                [0.0, 5.25, 16.0, 1.5],
                [0.0, 10.5, 8.0, 1.5]
            ]
        );
        let mut pixels = resvg::tiny_skia::Pixmap::new(16, 12).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixels.as_mut(),
        );
        let image = image::RgbaImage::from_raw(16, 12, pixels.take()).unwrap();
        for y in [0, 1, 5, 6, 10, 11] {
            assert!(image.get_pixel(4, y).0[3] > 0);
        }
        for y in [2, 3, 4, 7, 8, 9] {
            for x in 0..16 {
                assert_eq!(image.get_pixel(x, y).0[3], 0);
            }
        }
        for y in [0, 1, 5, 6] {
            assert!(image.get_pixel(15, y).0[3] > 0);
        }
        for y in [10, 11] {
            for x in 8..16 {
                assert_eq!(image.get_pixel(x, y).0[3], 0);
            }
        }
        assert_eq!(Glyph::Menu.path(), "kit/menu.svg");
        assert_eq!(Glyph::PanelRight.path(), "kit/panel-right.svg");
    }
    #[test]
    fn left_panel_is_registered_on_the_same_line_grid() {
        assert_eq!(
            view_box(Glyph::PanelLeft.svg()),
            Some([0.0, 0.0, 24.0, 24.0])
        );
        let bytes = Assets.load(Glyph::PanelLeft.path()).unwrap().unwrap();
        let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
        let mut pixels = resvg::tiny_skia::Pixmap::new(24, 24).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixels.as_mut(),
        );
        let image = image::RgbaImage::from_raw(24, 24, pixels.take()).unwrap();
        assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
        for at in 0..24 {
            for (x, y) in [(at, 0), (at, 23), (0, at), (23, at)] {
                assert_eq!(image.get_pixel(x, y).0[3], 0);
            }
        }
    }
}

#[cfg(test)]
mod transfer_glyph_tests {
    use super::{Assets, Glyph, view_box};
    use gpui::AssetSource as _;

    #[test]
    fn transfer_glyphs_load_and_paint_unclipped_at_menu_and_action_sizes() {
        for glyph in [Glyph::Upload, Glyph::Download] {
            assert_eq!(view_box(glyph.svg()), Some([0.0, 0.0, 24.0, 24.0]));
            let bytes = Assets.load(glyph.path()).unwrap().unwrap();
            let tree =
                resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
            for size in [14_u16, 16] {
                let side = u32::from(size);
                let scale = f32::from(size).mul_add(24.0_f32.recip(), 0.0);
                let mut pixels = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
                resvg::render(
                    &tree,
                    resvg::tiny_skia::Transform::from_scale(scale, scale),
                    &mut pixels.as_mut(),
                );
                let image = image::RgbaImage::from_raw(side, side, pixels.take()).unwrap();
                assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
                let edge = side.checked_sub(1).unwrap();
                for at in 0..side {
                    for (x, y) in [(at, 0), (at, edge), (0, at), (edge, at)] {
                        assert_eq!(image.get_pixel(x, y).0[3], 0);
                    }
                }
            }
        }
        assert_eq!(Glyph::ArrowUp.path(), "kit/arrow-up.svg");
        assert_eq!(Glyph::Update.path(), "kit/update.svg");
    }
}

#[cfg(test)]
mod diagnostics_glyph_tests {
    use super::{Assets, Glyph, view_box};
    use gpui::AssetSource as _;
    #[test]
    fn stethoscope_loads_and_clears_edges_at_both_sidebar_sizes() {
        let glyph = Glyph::Stethoscope;
        assert_eq!(glyph.path(), "kit/stethoscope.svg");
        assert_eq!(view_box(glyph.svg()), Some([0.0, 0.0, 24.0, 24.0]));
        assert_eq!(
            Assets.list(glyph.path()).unwrap(),
            vec![gpui::SharedString::from(glyph.path())]
        );
        let bytes = Assets.load(glyph.path()).unwrap().unwrap();
        let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
        for size in [16_u16, 20, 24] {
            let side = u32::from(size);
            let mut pixels = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::from_scale(
                    f32::from(size) / 24.0,
                    f32::from(size) / 24.0,
                ),
                &mut pixels.as_mut(),
            );
            let image = image::RgbaImage::from_raw(side, side, pixels.take()).unwrap();
            assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
            let edge = side.checked_sub(1).unwrap();
            for at in 0..side {
                for (x, y) in [(at, 0), (at, edge), (0, at), (edge, at)] {
                    assert_eq!(image.get_pixel(x, y).0[3], 0);
                }
            }
        }
    }
}

#[cfg(test)]
mod settings_nav_glyph_tests {
    use super::{Assets, Glyph, view_box};
    use gpui::AssetSource as _;

    /// Settings' list draws these at 16 (pointer) and 20 (compact), as
    /// Paseo's `densityIconSize("md")`; each paints and keeps its rim clear.
    #[test]
    fn project_and_provider_glyphs_paint_unclipped_at_both_list_sizes() {
        for (glyph, path) in [
            (Glyph::FolderGit, "kit/folder-git.svg"),
            (Glyph::Boxes, "kit/boxes.svg"),
        ] {
            assert_eq!(glyph.path(), path);
            assert_eq!(view_box(glyph.svg()), Some([0.0, 0.0, 24.0, 24.0]));
            let bytes = Assets.load(glyph.path()).unwrap().unwrap();
            let tree =
                resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
            for size in [16_u16, 20, 24] {
                let side = u32::from(size);
                let scale = f32::from(size).mul_add(24.0_f32.recip(), 0.0);
                let mut pixels = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
                resvg::render(
                    &tree,
                    resvg::tiny_skia::Transform::from_scale(scale, scale),
                    &mut pixels.as_mut(),
                );
                let image = image::RgbaImage::from_raw(side, side, pixels.take()).unwrap();
                assert!(
                    image.pixels().any(|pixel| pixel.0[3] > 0),
                    "{path} at {size}"
                );
                let edge = side.checked_sub(1).unwrap();
                for at in 0..side {
                    for (x, y) in [(at, 0), (at, edge), (0, at), (edge, at)] {
                        assert_eq!(image.get_pixel(x, y).0[3], 0, "{path} at {size}");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod activity_indicator_asset_tests {
    use super::{Assets, Glyph, view_box};
    use gpui::AssetSource as _;
    fn paint(dimension: u16, degrees: f32) -> image::RgbaImage {
        let bytes = Assets
            .load(Glyph::ActivityIndicator.path())
            .unwrap()
            .unwrap();
        let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
        let side = u32::from(dimension);
        let mut pixels = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
        let scale = f32::from(dimension).mul_add(32.0_f32.recip(), 0.0);
        let transform = resvg::tiny_skia::Transform::from_scale(scale, scale).pre_concat(
            resvg::tiny_skia::Transform::from_rotate_at(degrees, 16.0, 16.0),
        );
        resvg::render(&tree, transform, &mut pixels.as_mut());
        image::RgbaImage::from_raw(side, side, pixels.take()).unwrap()
    }
    #[test]
    fn activity_profile_keeps_alpha_and_foreground_arc_at_each_quarter_phase() {
        assert_eq!(
            Glyph::ActivityIndicator.path(),
            "kit/activity-indicator.svg"
        );
        assert_eq!(
            view_box(Glyph::ActivityIndicator.svg()),
            Some([0.0, 0.0, 32.0, 32.0])
        );
        assert_eq!(view_box(Glyph::Spinner.svg()), Some([0.0, 0.0, 24.0, 24.0]));
        for (phase, foreground, background) in [
            (0.0, (25, 25), (16, 2)),
            (90.0, (6, 25), (29, 16)),
            (180.0, (6, 6), (16, 29)),
            (270.0, (25, 6), (2, 16)),
        ] {
            let image = paint(32, phase);
            assert_eq!(image.get_pixel(foreground.0, foreground.1).0[3], 255);
            assert_eq!(image.get_pixel(background.0, background.1).0[3], 51);
            assert_eq!(image.get_pixel(16, 16).0[3], 0);
            assert_eq!(image.get_pixel(0, 0).0[3], 0);
        }
        for side in [10_u16, 16, 20, 36] {
            let image = paint(side, 0.0);
            assert_eq!(image.dimensions(), (u32::from(side), u32::from(side)));
            assert!(image.pixels().any(|pixel| pixel.0[3] == 255));
            assert!(image.pixels().any(|pixel| pixel.0[3] == 51));
        }
    }
}

#[cfg(test)]
mod added_navigation_tests;

#[cfg(test)]
mod inventory_tests;

#[cfg(test)]
mod panel_role_tests;

#[cfg(test)]
mod import_role_tests;

#[cfg(test)]
mod command_role_tests;
#[cfg(test)]
mod command_weight_tests;
#[cfg(test)]
mod launch_role_tests;
#[cfg(test)]
mod sheet_role_tests;
#[cfg(test)]
mod tab_menu_role_tests;
#[cfg(test)]
mod viewport_role_tests;
