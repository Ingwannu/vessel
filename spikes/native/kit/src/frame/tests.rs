//! The panel's edge on whole device pixels at every scale.
#![expect(clippy::float_arithmetic, reason = "points back to device pixels")]

use gpui::{Bounds, point, px, size};

use super::{Lines, edges};

fn whole(value: f32) -> bool {
    (value - value.round()).abs() < 1e-3
}

/// Both lines' boxes, widths and radii land on device pixels; the outer
/// one is one pitch outside the inner, its corner one pitch rounder.
#[test]
fn the_edges_land_on_device_pixels_one_pitch_apart() {
    let panel = Bounds {
        origin: point(px(264.3), px(36.6)),
        size: size(px(1007.1), px(755.2)),
    };
    for (scale, pitch) in [(1.0, 3.0), (1.25, 3.0), (1.5, 3.0), (2.0, 5.0)] {
        let two = edges(panel, 10.0, Lines::Two, scale);
        let inner = two.first().expect("the panel's own line");
        let outer = two.get(1).expect("the line a pitch outside");
        for edge in [inner, outer] {
            for value in [
                f32::from(edge.bounds.origin.x),
                f32::from(edge.bounds.origin.y),
                f32::from(edge.bounds.size.width),
                f32::from(edge.bounds.size.height),
                f32::from(edge.width),
            ] {
                assert!(whole(value * scale), "{value} at {scale}");
            }
        }
        let apart = (f32::from(inner.bounds.origin.x) - f32::from(outer.bounds.origin.x)) * scale;
        assert!((apart - pitch).abs() < 1e-3, "{apart} at {scale}");
        let rounder = (f32::from(outer.radius) - f32::from(inner.radius)) * scale;
        assert!((rounder - pitch).abs() < 1e-3);
    }
    assert_eq!(edges(panel, 10.0, Lines::One, 1.0).len(), 1);
}
