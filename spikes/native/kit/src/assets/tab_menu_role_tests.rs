//! The tab menu's close roles keep Lucide's shapes at the provider's 1.5
//! (TRACKER 789), one asset each.
use super::panel_role_tests::{strokes, tree};
use super::{Assets, Glyph, view_box};
use gpui::AssetSource as _;

/// Each role, its path and its stroked-vector count after usvg turns
/// polylines into paths.
const ROLES: [(Glyph, &str, usize); 3] = [
    (Glyph::ArrowLeftToLine, "kit/arrow-left-to-line.svg", 3),
    (Glyph::ArrowRightToLine, "kit/arrow-right-to-line.svg", 3),
    (Glyph::CopyX, "kit/copy-x.svg", 4),
];

#[test]
fn tab_menu_roles_have_one_asset_each_on_the_source_grid_and_stroke() {
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
    }
}
