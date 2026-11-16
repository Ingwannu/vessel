//! Zoomable-viewport roles keep the canonical lens and corner shapes, distinct
//! from the window's Maximize/Close and the panel's corner arrows.
use super::panel_role_tests::{paint, strokes, tree};
use super::{Assets, Glyph, view_box};
use gpui::AssetSource as _;

const ROLES: [(Glyph, &str, usize); 3] = [
    (Glyph::ZoomIn, "kit/zoom-in.svg", 4),
    (Glyph::ZoomOut, "kit/zoom-out.svg", 3),
    (Glyph::Scan, "kit/scan.svg", 4),
];

#[test]
fn viewport_roles_have_one_asset_each_on_the_inherited_source_grid_and_stroke() {
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
        assert_eq!(strokes(tree(glyph).root()), count);
    }
}

#[test]
fn zoom_in_zoom_out_and_scan_cannot_be_swapped_or_filled() {
    let zoom_in = paint(Glyph::ZoomIn, 24, 4);
    let zoom_out = paint(Glyph::ZoomOut, 24, 4);
    let scan = paint(Glyph::Scan, 24, 4);
    for lens in [&zoom_in, &zoom_out] {
        assert!(lens.get_pixel(11, 3).0[3] > 0, "the lens ring");
        assert!(lens.get_pixel(19, 19).0[3] > 0, "the lens handle");
        assert!(lens.get_pixel(9, 11).0[3] > 0, "the horizontal arm");
        assert_eq!(lens.get_pixel(6, 6).0[3], 0, "an unfilled lens");
    }
    assert!(zoom_in.get_pixel(11, 9).0[3] > 0, "the plus's vertical arm");
    assert_eq!(zoom_out.get_pixel(11, 9).0[3], 0, "no vertical arm");
    assert!(scan.get_pixel(3, 6).0[3] > 0, "a corner's leg");
    assert!(scan.get_pixel(20, 20).0[3] > 0, "the opposite corner");
    assert_eq!(scan.get_pixel(12, 3).0[3], 0, "open between corners");
    assert_eq!(scan.get_pixel(3, 12).0[3], 0, "open between corners");
    assert_eq!(scan.get_pixel(12, 12).0[3], 0, "an empty centre");
}

#[test]
fn small_toolbar_viewport_roles_keep_ink_and_a_clear_rim_at_fractional_scales() {
    for (glyph, _, _) in ROLES {
        for logical in [14_u16, 18] {
            for scale in [4_u16, 5, 6, 8] {
                let image = paint(glyph, logical, scale);
                assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
                assert!(
                    image
                        .pixels()
                        .any(|pixel| pixel.0[3] > 0 && pixel.0[3] < u8::MAX)
                );
                let end = image.width().checked_sub(1).unwrap();
                for at in 0..image.width() {
                    for (x, y) in [(at, 0), (at, end), (0, at), (end, at)] {
                        assert_eq!(image.get_pixel(x, y).0[3], 0, "{glyph:?}/{logical}/{scale}");
                    }
                }
            }
        }
    }
}
