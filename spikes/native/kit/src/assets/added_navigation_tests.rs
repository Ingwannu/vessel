//! Registered original usage/catalog vectors keep the source grid and line width.
use super::{Assets, Glyph, view_box};
use gpui::AssetSource as _;
use resvg::usvg;

fn check_strokes(group: &usvg::Group) -> usize {
    let mut paths = 0_usize;
    for node in group.children() {
        match node {
            usvg::Node::Path(path) => {
                assert_eq!(
                    path.stroke().unwrap().width().get().to_bits(),
                    1.5_f32.to_bits()
                );
                paths = paths.saturating_add(1);
            }
            usvg::Node::Group(group) => paths = paths.saturating_add(check_strokes(group)),
            usvg::Node::Image(_) | usvg::Node::Text(_) => panic!("a line glyph has paths only"),
        }
    }
    paths
}
#[test]
fn usage_and_catalog_assets_are_registered_once_with_source_grid_and_stroke() {
    for (glyph, path) in [
        (Glyph::Gauge, "kit/gauge.svg"),
        (Glyph::PackagePlus, "kit/package-plus.svg"),
    ] {
        assert_eq!(glyph.path(), path);
        let data = Assets.load(path).unwrap().unwrap();
        assert_eq!(data.as_ref(), glyph.svg().as_bytes());
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
        let tree = usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap();
        assert!(check_strokes(tree.root()) > 0);
        let ink = tree.root().abs_stroke_bounding_box();
        assert!(ink.left() > 0.0 && ink.top() > 0.0);
        assert!(ink.right() < 24.0 && ink.bottom() < 24.0);
        for dimension in [16_u16, 20, 24] {
            let side = u32::from(dimension);
            let mut image = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
            let scale = f32::from(dimension).mul_add(24.0_f32.recip(), 0.0);
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::from_scale(scale, scale),
                &mut image.as_mut(),
            );
            assert_eq!((image.width(), image.height()), (side, side));
            assert!(image.pixels().iter().any(|pixel| pixel.alpha() > 0));
            assert!(image.pixels().iter().any(|pixel| pixel.alpha() == 0));
        }
    }
}
