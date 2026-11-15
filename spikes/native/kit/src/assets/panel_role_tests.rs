//! Panel roles cannot silently become window controls or swapped divider axes.
use super::{Assets, Glyph, view_box};
use gpui::AssetSource as _;
use resvg::usvg;
const ROLES: [Glyph; 5] = [
    Glyph::Columns2,
    Glyph::Rows2,
    Glyph::PanelMaximize,
    Glyph::PanelMinimize,
    Glyph::X,
];
const QUARTERS: u16 = 4;
fn strokes(group: &usvg::Group) -> usize {
    let mut count = 0_usize;
    for node in group.children() {
        match node {
            usvg::Node::Path(path) => {
                assert!(path.fill().is_none());
                assert_eq!(
                    path.stroke().unwrap().width().get().to_bits(),
                    1.5_f32.to_bits()
                );
                count = count.saturating_add(1);
            }
            usvg::Node::Group(group) => count = count.saturating_add(strokes(group)),
            usvg::Node::Image(_) | usvg::Node::Text(_) => panic!("vector line paths only"),
        }
    }
    count
}
fn tree(glyph: Glyph) -> usvg::Tree {
    let data = Assets.load(glyph.path()).unwrap().unwrap();
    assert_eq!(data.as_ref(), glyph.svg().as_bytes());
    usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap()
}
fn paint(glyph: Glyph, logical: u16, quarter_scale: u16) -> image::RgbaImage {
    let side = u32::from(
        logical
            .checked_mul(quarter_scale)
            .unwrap()
            .div_ceil(QUARTERS),
    );
    let pixels_per_point = f32::from(quarter_scale).mul_add(f32::from(QUARTERS).recip(), 0.0);
    let scale = f32::from(logical)
        .mul_add(pixels_per_point, 0.0)
        .mul_add(24.0_f32.recip(), 0.0);
    let mut pixels = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
    resvg::render(
        &tree(glyph),
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixels.as_mut(),
    );
    image::RgbaImage::from_raw(side, side, pixels.take()).unwrap()
}
#[test]
fn canonical_panel_roles_are_unique_vector_paths_on_the_source_grid_and_stroke() {
    for (glyph, path) in ROLES.into_iter().zip([
        "kit/columns-2.svg",
        "kit/rows-2.svg",
        "kit/panel-maximize.svg",
        "kit/panel-minimize.svg",
        "kit/x.svg",
    ]) {
        assert_eq!(glyph.path(), path);
        assert_eq!(view_box(glyph.svg()), Some([0.0, 0.0, 24.0, 24.0]));
        assert_eq!(
            Assets
                .list("kit/")
                .unwrap()
                .iter()
                .filter(|name| name.as_ref() == path)
                .count(),
            1
        );
        assert!(strokes(tree(glyph).root()) > 0);
    }
    for glyph in [
        Glyph::Maximize,
        Glyph::Minimize,
        Glyph::Restore,
        Glyph::Close,
    ] {
        assert_eq!(view_box(glyph.svg()), Some([0.0, 0.0, 10.0, 10.0]));
    }
}
#[test]
fn panel_dividers_and_inward_restore_corners_have_their_own_roles() {
    let column = paint(Glyph::Columns2, 24, QUARTERS);
    let row = paint(Glyph::Rows2, 24, QUARTERS);
    assert!(column.get_pixel(12, 6).0[3] > 0);
    assert_eq!(column.get_pixel(6, 12).0[3], 0);
    assert!(row.get_pixel(6, 12).0[3] > 0);
    assert_eq!(row.get_pixel(12, 6).0[3], 0);
    let maximize = paint(Glyph::PanelMaximize, 24, QUARTERS);
    let restore = paint(Glyph::PanelMinimize, 24, QUARTERS);
    assert!(maximize.get_pixel(3, 5).0[3] > 0);
    assert_eq!(maximize.get_pixel(8, 5).0[3], 0);
    assert!(restore.get_pixel(8, 5).0[3] > 0);
    assert_eq!(restore.get_pixel(3, 5).0[3], 0);
    let close = paint(Glyph::X, 24, QUARTERS);
    assert!(close.get_pixel(12, 12).0[3] > 0);
    assert_eq!(close.get_pixel(6, 12).0[3], 0);
}
#[test]
fn panel_role_vectors_retain_ink_and_a_clear_rim_at_fractional_display_scales() {
    for glyph in ROLES {
        for logical in [14_u16, 16, 20] {
            for quarter_scale in [4_u16, 5, 6, 8] {
                let image = paint(glyph, logical, quarter_scale);
                assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
                assert!(
                    image
                        .pixels()
                        .any(|pixel| pixel.0[3] > 0 && pixel.0[3] < u8::MAX)
                );
                let edge = image.width().checked_sub(1).unwrap();
                for at in 0..image.width() {
                    for (x, y) in [(at, 0), (at, edge), (0, at), (edge, at)] {
                        assert_eq!(
                            image.get_pixel(x, y).0[3],
                            0,
                            "{glyph:?}/{logical}/{quarter_scale}"
                        );
                    }
                }
            }
        }
    }
}
