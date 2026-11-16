//! The root Import role is a framed import arrow, with its explicit source ink.
use super::{Assets, Glyph, view_box};
use gpui::AssetSource as _;
use resvg::usvg;

fn tree() -> usvg::Tree {
    let data = Assets.load(Glyph::Import.path()).unwrap().unwrap();
    assert_eq!(data.as_ref(), Glyph::Import.svg().as_bytes());
    usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap()
}

fn check_strokes(group: &usvg::Group) -> usize {
    group.children().iter().fold(0_usize, |count, node| {
        count.saturating_add(match node {
            usvg::Node::Path(path) => {
                assert!(path.fill().is_none());
                assert_eq!(
                    path.stroke().unwrap().width().get().to_bits(),
                    2.2_f32.to_bits()
                );
                1
            }
            usvg::Node::Group(group) => check_strokes(group),
            usvg::Node::Image(_) | usvg::Node::Text(_) => panic!("vector line paths only"),
        })
    })
}

fn paint(logical: u16, quarter_scale: u16) -> image::RgbaImage {
    let side = u32::from(logical.checked_mul(quarter_scale).unwrap().div_ceil(4));
    let scale = f32::from(logical)
        .mul_add(f32::from(quarter_scale), 0.0)
        .mul_add(96.0_f32.recip(), 0.0);
    let mut pixels = resvg::tiny_skia::Pixmap::new(side, side).unwrap();
    resvg::render(
        &tree(),
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixels.as_mut(),
    );
    image::RgbaImage::from_raw(side, side, pixels.take()).unwrap()
}

#[test]
fn root_import_has_one_canonical_asset_and_its_source_specific_stroke() {
    assert_eq!(Glyph::Import.path(), "kit/import.svg");
    assert_eq!(view_box(Glyph::Import.svg()), Some([0.0, 0.0, 24.0, 24.0]));
    assert_eq!(
        Assets
            .list("kit/")
            .unwrap()
            .iter()
            .filter(|path| path.as_ref() == Glyph::Import.path())
            .count(),
        1
    );
    assert_eq!(check_strokes(tree().root()), 3);
}

#[test]
fn root_import_arrow_points_into_the_framed_body_at_fractional_scales() {
    let source = paint(24, 4);
    for (x, y) in [(12, 3), (12, 15), (8, 11), (2, 10), (22, 10), (12, 19)] {
        assert!(source.get_pixel(x, y).0[3] > 0, "source landmark {x},{y}");
    }
    assert_eq!(
        source.get_pixel(12, 21).0[3],
        0,
        "the source frame leaves its lower margin"
    );
    for logical in [14_u16, 16, 20] {
        for scale in [4_u16, 5, 6, 8] {
            let image = paint(logical, scale);
            assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
            assert!(
                image
                    .pixels()
                    .any(|pixel| pixel.0[3] > 0 && pixel.0[3] < u8::MAX)
            );
            let end = image.width().checked_sub(1).unwrap();
            for (x, y) in [(0, 0), (end, 0), (0, end), (end, end)] {
                assert_eq!(image.get_pixel(x, y).0[3], 0, "{logical}/{scale}");
            }
        }
    }
}
