//! A remeasure is layout, not a new owner open or a second motion engine.
use super::*;
use std::time::Duration;

fn fit(state: &mut State, viewport: f32, now: Instant) {
    assert_eq!(
        state.fit_content(px(240.0), px(viewport), Percentage::new(80).unwrap(), now),
        Fit::Updated
    );
}
fn frame(state: &mut State, viewport: f32, now: Instant, reduced: bool) -> Geometry {
    match state.frame(px(viewport), now, reduced) {
        Frame::Visible(frame) => frame,
        other @ (Frame::PendingLayout | Frame::Closed) => panic!("visible: {other:?}"),
    }
}
fn near(actual: Pixels, expected: f32) {
    assert!(actual.as_f32().mul_add(-1.0, expected).abs() < 0.005);
}

#[test]
fn measured_first_frame_animates_from_closed_with_a_valid_viewport() {
    let now = Instant::now();
    let mut state = State::default();
    fit(&mut state, 600.0, now);
    let initial = frame(&mut state, 600.0, now, false);
    near(initial.height, 0.0);
    assert_eq!(initial.movement, Movement::Animating);
    let later = frame(&mut state, 600.0, now + Duration::from_millis(32), false);
    assert!(later.height > initial.height && later.height < px(240.0));
    near(frame(&mut state, 600.0, now, true).height, 240.0);
}

#[test]
fn measured_resize_keeps_a_resting_panel_bottom_anchored_without_an_animation() {
    let now = Instant::now();
    let mut state = State::default();
    fit(&mut state, 600.0, now);
    let _opened = frame(&mut state, 600.0, now, true);
    fit(&mut state, 800.0, now);
    let resized = frame(&mut state, 800.0, now, false);
    near(resized.top, 560.0);
    near(resized.height, 240.0);
    assert_eq!(resized.movement, Movement::Resting);
}

#[test]
fn measured_resize_preserves_partial_openness_and_an_existing_close_target() {
    let now = Instant::now();
    let mut state = State::default();
    fit(&mut state, 600.0, now);
    let midway = now + Duration::from_millis(32);
    let before = frame(&mut state, 600.0, midway, false);
    fit(&mut state, 800.0, midway);
    near(
        frame(&mut state, 800.0, midway, false).height,
        before.height.as_f32(),
    );
    let _opened = frame(&mut state, 800.0, midway, true);
    state.close(midway);
    fit(&mut state, 600.0, midway);
    let closing = frame(&mut state, 600.0, midway + Duration::from_millis(32), false);
    assert!(closing.height > px(0.0) && closing.height < px(240.0));
    assert!(matches!(
        state.frame(px(600.0), midway, true),
        Frame::Closed
    ));
    fit(&mut state, 800.0, midway);
    assert!(matches!(
        state.frame(px(800.0), midway, false),
        Frame::Closed
    ));
}
