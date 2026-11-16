//! The new-tab menu's launch roles keep Lucide 1.50.0's shapes at the
//! provider's 1.5 (TRACKER 776), one asset each.
use super::panel_role_tests::{strokes, tree};
use super::{Assets, Glyph, view_box};
use gpui::AssetSource as _;

/// Each role, its path and its stroked-vector count after usvg turns
/// circles and rects into paths.
const ROLES: [(Glyph, &str, usize); 5] = [
    (Glyph::SquarePen, "kit/square-pen.svg", 2),
    (Glyph::SquareTerminal, "kit/square-terminal.svg", 3),
    (Glyph::FileDiff, "kit/file-diff.svg", 4),
    (Glyph::Files, "kit/files.svg", 3),
    (Glyph::Globe, "kit/globe.svg", 3),
];

#[test]
fn launch_roles_have_one_asset_each_on_the_source_grid_and_stroke() {
    for (glyph, path, count) in ROLES {
        assert_eq!(glyph.path(), path);
        assert_eq!(view_box(glyph.svg()), Some([0.0, 0.0, 24.0, 24.0]));
        assert_eq!(
            Assets
                .list("kit/")
                .unwrap()
                .iter()
                .filter(|entry| entry.as_ref() == path)
                .count(),
            1
        );
        // The shared parser asserts unfilled vector paths at stroke 1.5.
        assert_eq!(strokes(tree(glyph).root()), count, "{glyph:?}");
        // A command list draws them at its 2.2 too.
        assert!(glyph.command_weight_path().starts_with("kit/command-weight/"));
    }
}
