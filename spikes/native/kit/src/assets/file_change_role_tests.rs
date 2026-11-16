//! Actual file-change assets retain the aligned square family and inherited ink.
use super::panel_role_tests::{paint, strokes, tree};
use super::{Assets, Glyph, view_box};
use gpui::AssetSource as _;

const ROLES: [(Glyph, &str, usize); 3] = [
    (Glyph::SquarePlus, "kit/square-plus.svg", 3),
    (Glyph::SquareMinus, "kit/square-minus.svg", 2),
    (Glyph::SquareDot, "kit/square-dot.svg", 2),
];

#[test]
fn file_change_family_has_one_asset_per_role_on_the_inherited_source_grid() {
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
        // Existing shared parser asserts unfilled vector paths at stroke1.5.
        assert_eq!(strokes(tree(glyph).root()), count);
    }
}

#[test]
fn file_change_plus_minus_and_dot_cannot_be_swapped_or_filled() {
    let plus = paint(Glyph::SquarePlus, 24, 4);
    let minus = paint(Glyph::SquareMinus, 24, 4);
    let dot = paint(Glyph::SquareDot, 24, 4);
    for image in [&plus, &minus, &dot] {
        assert!(image.get_pixel(3, 12).0[3] > 0, "same square frame");
        assert_eq!(image.get_pixel(7, 7).0[3], 0, "unfilled square");
    }
    assert!(plus.get_pixel(12, 8).0[3] > 0, "vertical plus arm");
    assert_eq!(minus.get_pixel(12, 8).0[3], 0);
    assert_eq!(dot.get_pixel(12, 8).0[3], 0);
    assert!(plus.get_pixel(8, 12).0[3] > 0, "horizontal plus arm");
    assert!(minus.get_pixel(8, 12).0[3] > 0, "horizontal minus arm");
    assert_eq!(dot.get_pixel(8, 12).0[3], 0);
    assert!(dot.get_pixel(12, 12).0[3] > 0, "stroked center circle");
}

#[test]
fn fourteen_point_file_change_roles_keep_ink_and_a_clear_rim_at_fractional_scales() {
    for (glyph, _, _) in ROLES {
        for scale in [4_u16, 5, 6, 8] {
            let image = paint(glyph, 14, scale);
            assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
            assert!(
                image
                    .pixels()
                    .any(|pixel| pixel.0[3] > 0 && pixel.0[3] < u8::MAX)
            );
            let end = image.width().checked_sub(1).unwrap();
            for at in 0..image.width() {
                for (x, y) in [(at, 0), (at, end), (0, at), (end, at)] {
                    assert_eq!(image.get_pixel(x, y).0[3], 0, "{glyph:?}/{scale}");
                }
            }
        }
    }
}
