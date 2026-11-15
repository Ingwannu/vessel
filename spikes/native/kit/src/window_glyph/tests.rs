//! The window glyphs on the device grid at every scale the owner's screens use.
#![expect(clippy::float_arithmetic, reason = "measuring device-pixel geometry")]

use super::{WindowGlyph, metrics, shapes};

const SCALES: [f32; 4] = [1.0, 1.25, 1.5, 2.0];

fn whole(value: f32) -> bool {
    (value - value.round()).abs() < 1e-4
}

/// Line, gap: one pixel and two at 1, 1.25 and 1.5; two and three at 2.
#[test]
fn the_line_and_the_gap_are_whole_pixels() {
    assert_eq!(metrics(1.0), (1.0, 2.0, 3.0));
    assert_eq!(metrics(1.25), (1.0, 2.0, 3.0));
    assert_eq!(metrics(1.5), (1.0, 2.0, 3.0));
    assert_eq!(metrics(2.0), (2.0, 3.0, 5.0));
}

/// Every horizontal or vertical run's edges fall on device pixel edges,
/// so the stroke covers whole rows or columns and never greys over two.
#[test]
fn straight_strokes_cover_whole_device_rows() {
    for scale in SCALES {
        let (w, _, _) = metrics(scale);
        for glyph in [
            WindowGlyph::Minimize,
            WindowGlyph::Maximize,
            WindowGlyph::Restore,
        ] {
            for shape in shapes(glyph, scale) {
                for pair in shape.points.windows(2) {
                    let [(x0, y0), (x1, y1)] = pair else { continue };
                    if (y0 - y1).abs() < 1e-4 && (x0 - x1).abs() > 1.0 {
                        assert!(whole(y0 - w / 2.0), "{glyph:?} at {scale}: row {y0}");
                    }
                    if (x0 - x1).abs() < 1e-4 && (y0 - y1).abs() > 1.0 {
                        assert!(whole(x0 - w / 2.0), "{glyph:?} at {scale}: column {x0}");
                    }
                }
            }
        }
    }
}

/// The cross's back stroke stops one gap short of the front stroke's edge.
#[test]
fn the_cross_breaks_for_one_gap() {
    for scale in SCALES {
        let (w, g, _) = metrics(scale);
        let side = (9.0 * scale).round();
        let all = shapes(WindowGlyph::Close, scale);
        let stub = all.get(1).expect("the back stroke's first half");
        let &(x, y) = stub.points.last().expect("its end");
        // distance from the stub's end to the front diagonal x + y = side
        let distance = (side - x - y).abs() / std::f32::consts::SQRT_2;
        let air = distance - w / 2.0 - w / 2.0;
        assert!((air - g).abs() < 1e-3, "at {scale}: {air}");
    }
}

/// The restored window's back square keeps one gap from the front one.
#[test]
fn the_back_square_keeps_one_gap() {
    for scale in SCALES {
        let (w, g, pitch) = metrics(scale);
        let side = (9.0 * scale).round();
        let all = shapes(WindowGlyph::Restore, scale);
        let front = all.first().expect("the front square");
        let back = all.get(1).expect("the back square's visible part");
        let front_top = front
            .points
            .iter()
            .map(|p| p.1)
            .fold(f32::INFINITY, f32::min);
        let back_top = back
            .points
            .iter()
            .map(|p| p.1)
            .fold(f32::INFINITY, f32::min);
        assert!(((front_top - back_top) - pitch).abs() < 1e-3);
        let front_right = front
            .points
            .iter()
            .map(|p| p.0)
            .fold(f32::NEG_INFINITY, f32::max);
        let back_right = back
            .points
            .iter()
            .map(|p| p.0)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((back_right - front_right - w - g).abs() < 1e-3);
        assert!(back_right <= side - w / 2.0 + 1e-3);
    }
}
