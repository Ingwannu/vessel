//! The drawing's raster box and its SVG.
#![expect(
    clippy::float_arithmetic,
    reason = "a colour channel written as a fraction of 255"
)]

use super::{Drawing, Shape, device_box, extent, svg};

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
