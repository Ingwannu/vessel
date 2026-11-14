//! The drawing's raster box and its SVG.
#![expect(
    clippy::float_arithmetic,
    reason = "a colour channel written as a fraction of 255"
)]

use super::{Drawing, KEPT, Shape, clipped, device_box, extent, kept, svg};

fn drawing() -> Drawing {
    Drawing {
        shapes: vec![
            Shape {
                points: vec![(10.0, 10.0), (110.5, 10.0), (110.5, 60.25)],
                closed: true,
                stroke: Some(3.0),
                alpha: 1.0,
            },
            Shape {
                points: vec![(40.0, 20.0), (50.0, 30.0), (40.0, 40.0)],
                closed: true,
                stroke: None,
                alpha: 0.5,
            },
            // an invisible shape does not widen the raster
            Shape {
                points: vec![(-500.0, -500.0), (900.0, 900.0)],
                closed: false,
                stroke: Some(3.0),
                alpha: 0.0,
            },
        ],
        ink: gpui::Rgba {
            r: 250.0 / 255.0,
            g: 250.0 / 255.0,
            b: 250.0 / 255.0,
            a: 0.07,
        }
        .into(),
    }
}

/// The raster covers every visible shape with its stroke's half and one
/// point of air, and nothing invisible.
#[test]
fn the_raster_covers_the_visible_shapes_and_their_strokes() {
    assert_eq!(extent(&drawing()), Some((7.5, 7.5, 113.0, 62.75)));
    let empty = Drawing {
        shapes: Vec::new(),
        ..drawing()
    };
    assert_eq!(extent(&empty), None);
}

/// The box lands on whole device pixels at every scale the owner's screens
/// use, its far corner rounded on its own so the size never drifts.
#[test]
fn the_box_lands_on_whole_device_pixels_at_every_scale() {
    let (x0, y0, x1, y1) = (7.5, 7.5, 113.0, 62.75);
    assert_eq!(device_box(x0, y0, x1, y1, 1.0), (8, 8, 105, 55));
    assert_eq!(device_box(x0, y0, x1, y1, 1.25), (9, 9, 132, 69));
    assert_eq!(device_box(x0, y0, x1, y1, 1.5), (11, 11, 159, 83));
    assert_eq!(device_box(x0, y0, x1, y1, 2.0), (15, 15, 211, 111));
}

/// The SVG states the device size and the logical box, strokes and fills
/// carry the ink at their share, and it parses.
#[test]
fn the_svg_is_the_shapes_in_one_ink() {
    let text = svg(&drawing(), 8.0, 8.0, 105.0, 55.0, (105, 55));
    assert!(text.starts_with(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="8 8 105 55" width="105" height="55">"#
    ));
    assert!(text.contains(r##"stroke="#fafafa" stroke-width="3.000" stroke-opacity="0.0700""##));
    assert!(text.contains(r##"fill="#fafafa" fill-opacity="0.0350""##));
    assert_eq!(
        text.matches("<path").count(),
        2,
        "the invisible shape is left out"
    );
    let tree = resvg::usvg::Tree::from_str(&text, &resvg::usvg::Options::default());
    assert!(tree.is_ok());
}

#[test]
fn small_rasters_stay_twelve() {
    let keys: Vec<_> = (0_u64..20).map(|hash| (hash, 11, 11)).collect();
    let flags = kept(&keys);
    assert_eq!(flags.iter().filter(|&&keep| keep).count(), KEPT);
    assert!(flags.iter().skip(8).all(|&keep| keep), "the newest stay");
}

#[test]
fn only_the_two_newest_large_rasters_stay() {
    // a glyph, three backdrop frames of a transition, an avatar
    let keys = [
        (1, 11, 11),
        (2, 1000, 760),
        (3, 1000, 760),
        (4, 1000, 760),
        (5, 48, 48),
    ];
    assert_eq!(kept(&keys), vec![true, false, true, true, true]);
}

#[test]
fn a_backdrop_at_scale_two_keeps_the_glyphs() {
    // one raster far above the large size never pushes the small ones out
    let mut keys: Vec<_> = (0_u64..6).map(|hash| (hash, 18, 18)).collect();
    keys.push((9, 3400, 2100));
    assert!(kept(&keys).iter().all(|&keep| keep));
}

#[test]
fn a_backdrop_rasterises_only_the_area_it_fills() {
    let panel = gpui::Bounds {
        origin: gpui::point(gpui::px(264.0), gpui::px(37.0)),
        size: gpui::size(gpui::px(1007.0), gpui::px(755.0)),
    };
    // the open session's frame: arms down to an apex far below the screen
    let far = (270.0, 60.0, 1265.0, 40_000.0);
    assert_eq!(
        clipped(far, Some(panel)),
        Some((270.0, 60.0, 1265.0, 792.0))
    );
    assert_eq!(clipped((0.0, 0.0, 100.0, 20.0), Some(panel)), None);
    assert_eq!(clipped(far, None), Some(far));
}
