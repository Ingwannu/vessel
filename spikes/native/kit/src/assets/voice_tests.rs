//! Exact voice stroke/fill source and shared asset rendering, never an area SVG.
use super::*;

fn raster(glyph: Glyph) -> image::RgbaImage {
    let bytes = Assets.load(glyph.path()).unwrap().unwrap();
    let tree = resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).unwrap();
    let mut pixels = resvg::tiny_skia::Pixmap::new(20, 20).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(20.0 / 24.0, 20.0 / 24.0),
        &mut pixels.as_mut(),
    );
    image::RgbaImage::from_raw(20, 20, pixels.take()).unwrap()
}

#[test]
fn voice_assets_are_registered_at_source_stroke_and_filled_stop_is_distinct() {
    let all = Assets.list("kit/").unwrap();
    for glyph in [Glyph::Mic, Glyph::MicOff, Glyph::SquareFilled] {
        assert_eq!(
            all.iter()
                .filter(|path| path.as_ref() == glyph.path())
                .count(),
            1
        );
        assert_eq!(view_box(glyph.svg()), Some([0.0, 0.0, 24.0, 24.0]));
        assert!(glyph.svg().contains("stroke-width=\"2.5\""));
        assert!(raster(glyph).pixels().any(|pixel| pixel.0[3] > 0));
    }
    assert_eq!(raster(Glyph::SquareFilled).get_pixel(10, 10).0[3], 255);
    assert_eq!(raster(Glyph::Stop).get_pixel(10, 10).0[3], 0);
    assert_ne!(
        raster(Glyph::Mic).into_raw(),
        raster(Glyph::MicOff).into_raw()
    );
}
