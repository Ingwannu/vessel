//! Configured stops share the default spring, drag and content-scroll policy.
use super::*;
use gpui::point;
use std::time::Duration;

#[test]
fn configured_heights_change_both_layout_and_the_scroll_handoff() {
    let now = Instant::now();
    let mut state = State::with_points(SnapPoints::new(85, 95).unwrap());
    state.present(now);
    let Frame::Visible(initial) = state.frame(px(1000.0), now, true) else {
        panic!("visible sheet");
    };
    assert!(f32::from(initial.height).mul_add(-1.0, 850.0).abs() < 0.01);
    state.press(Origin::Content, point(px(100.0), px(300.0)), px(0.0), now);
    let later = now + Duration::from_millis(100);
    let offset = state
        .pan(point(px(100.0), px(100.0)), px(1000.0), later)
        .unwrap();
    assert!(
        f32::from(offset).mul_add(-1.0, 100.0).abs() < 0.01,
        "travel past the 95% stop scrolls content"
    );
    assert_eq!(state.release(later), Release::Pan(Stop::Expanded));
    let Frame::Visible(expanded) = state.frame(px(1000.0), later, true) else {
        panic!("expanded sheet");
    };
    assert!(f32::from(expanded.height).mul_add(-1.0, 950.0).abs() < 0.01);
    state.retain_compact();
    let Frame::Visible(retained) = state.frame(px(1000.0), later, true) else {
        panic!("refused close remains visible");
    };
    assert!(f32::from(retained.height).mul_add(-1.0, 850.0).abs() < 0.01);
}

#[test]
fn point_order_and_percentage_bounds_are_validated_once() {
    for (compact, expanded) in [(0, 95), (85, 101), (95, 85), (85, 85)] {
        assert!(SnapPoints::new(compact, expanded).is_none());
    }
    assert_eq!(SnapPoints::default(), SnapPoints::new(60, 90).unwrap());
    assert_eq!(
        Stop::Compact.position_in(SnapPoints::default()).to_bits(),
        Stop::Compact.position().to_bits()
    );
    assert_eq!(
        Stop::Expanded.position_in(SnapPoints::default()).to_bits(),
        Stop::Expanded.position().to_bits()
    );
}
