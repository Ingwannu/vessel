//! Actual shared physics, with literal content height rather than equal authored stops.
use super::*;
use gpui::point;
use std::time::Duration;
fn cap() -> Percentage {
    Percentage::new(80).unwrap()
}
fn visible(state: &mut State, height: f32, now: Instant) -> Geometry {
    match state.frame(px(height), now, true) {
        Frame::Visible(frame) => frame,
        frame @ (Frame::PendingLayout | Frame::Closed) => panic!("visible: {frame:?}"),
    }
}
#[test]
fn content_fit_uses_measured_pixels_below_the_viewport_cap_and_resizes_in_place() {
    let now = Instant::now();
    let mut state = State::default();
    assert_eq!(
        state.fit_content(px(173.5), px(600.0), cap(), now),
        Fit::Updated
    );
    let frame = visible(&mut state, 600.0, now);
    assert!(frame.height.as_f32().mul_add(-1.0, 173.5).abs() < 0.001);
    assert!(frame.top.as_f32().mul_add(-1.0, 426.5).abs() < 0.001);
    assert_eq!(frame.backdrop.to_bits(), 1.0_f32.to_bits());
    assert_eq!(
        state.fit_content(px(173.5), px(600.0), cap(), now),
        Fit::Unchanged
    );
    assert_eq!(
        state.fit_content(px(1200.0), px(600.0), cap(), now),
        Fit::Updated
    );
    let frame = visible(&mut state, 600.0, now);
    assert!(frame.height.as_f32().mul_add(-1.0, 480.0).abs() < 0.001);
    assert_eq!(
        state.fit_content(px(1200.0), px(1000.0), cap(), now),
        Fit::Unchanged
    );
    let frame = visible(&mut state, 1000.0, now);
    assert!(frame.height.as_f32().mul_add(-1.0, 800.0).abs() < 0.001);
    for (content, viewport) in [(0.0, 600.0), (100.0, 0.0), (f32::NAN, 600.0)] {
        assert_eq!(
            state.fit_content(px(content), px(viewport), cap(), now),
            Fit::PendingLayout
        );
    }
    assert!(
        visible(&mut state, 1000.0, now)
            .height
            .as_f32()
            .mul_add(-1.0, 800.0)
            .abs()
            < 0.001
    );
    assert!(SnapPoints::new(80, 80).is_none());
}
#[test]
fn fitted_content_scrolls_at_its_only_open_stop_and_pulls_to_close() {
    let now = Instant::now();
    let mut state = State::default();
    assert_eq!(
        state.fit_content(px(300.0), px(1000.0), cap(), now),
        Fit::Updated
    );
    let _frame = visible(&mut state, 1000.0, now);
    state.press(Origin::Content, point(px(50.0), px(800.0)), px(0.0), now);
    let later = now + Duration::from_secs(1);
    let offset = state
        .pan(point(px(50.0), px(700.0)), px(600.0), later)
        .unwrap();
    assert!(offset.as_f32().mul_add(-1.0, 100.0).abs() < 0.001);
    assert_eq!(state.release(later), Release::Pan(Stop::Expanded));
    let _frame = visible(&mut state, 1000.0, later);
    state.press(Origin::Content, point(px(50.0), px(700.0)), offset, later);
    let after = later + Duration::from_secs(1);
    let offset = state
        .pan(point(px(50.0), px(750.0)), px(600.0), after)
        .unwrap();
    assert!(offset.as_f32().mul_add(-1.0, 50.0).abs() < 0.001);
    assert!(
        visible(&mut state, 1000.0, after)
            .top
            .as_f32()
            .mul_add(-1.0, 700.0)
            .abs()
            < 0.001
    );
    let _released = state.release(after);
    let _frame = visible(&mut state, 1000.0, after);
    state.press(Origin::Handle, point(px(50.0), px(700.0)), px(0.0), after);
    let close = after + Duration::from_secs(1);
    assert!(
        state
            .pan(point(px(50.0), px(990.0)), px(600.0), close)
            .is_some()
    );
    assert_eq!(state.release(close), Release::Pan(Stop::Closed));
    assert!(matches!(
        state.frame(px(1000.0), close, true),
        Frame::Closed
    ));
    state.retain_compact();
    assert!(
        visible(&mut state, 1000.0, close)
            .height
            .as_f32()
            .mul_add(-1.0, 300.0)
            .abs()
            < 0.001
    );
}
