//! Geometry and routing tests complement the real pointer captures;
//! these do not claim to exercise hit testing or rendered font metrics.

use std::time::{Duration, Instant};

use gpui::{point, px};

use super::motion::{Spring, SpringValue, fade_in};
use super::{Frame, Geometry, Movement, Origin, Release, State, Stop};

fn visible(state: &mut State, now: Instant, reduced: bool) -> Geometry {
    match state.frame(px(1000.0), now, reduced) {
        Frame::Visible(frame) => frame,
        frame @ (Frame::PendingLayout | Frame::Closed) => {
            panic!("expected a visible sheet, got {frame:?}")
        }
    }
}

fn opened() -> (State, Instant) {
    let now = Instant::now();
    let mut state = State::default();
    state.present(now);
    let frame = visible(&mut state, now, true);
    assert_eq!(frame.top, px(400.0));
    assert_eq!(frame.height, px(600.0));
    (state, now)
}

fn settle(state: &mut State, now: Instant) -> Frame {
    let mut frame = Frame::PendingLayout;
    for tick in 1_u64..=120 {
        frame = state.frame(
            px(1000.0),
            now.checked_add(Duration::from_millis(tick.saturating_mul(16)))
                .unwrap(),
            false,
        );
    }
    frame
}

#[test]
fn invalid_viewport_is_not_a_dismissal() {
    let (mut state, now) = opened();
    assert!(matches!(
        state.frame(px(0.0), now, false),
        Frame::PendingLayout
    ));
    assert_eq!(visible(&mut state, now, false).top, px(400.0));
}

#[test]
fn handle_expands_collapses_closes_and_reopens_at_sixty_percent() {
    let (mut state, now) = opened();
    let later = now + Duration::from_secs(1);
    state.press(Origin::Handle, point(px(100.0), px(400.0)), px(0.0), now);
    assert!(
        state
            .pan(point(px(100.0), px(100.0)), px(1000.0), later)
            .is_some()
    );
    assert_eq!(
        state.release(later + Duration::from_millis(200)),
        Release::Pan(Stop::Expanded)
    );
    let after = later + Duration::from_secs(3);
    assert!(matches!(settle(&mut state, later), Frame::Visible(_)));
    assert_eq!(visible(&mut state, after, false).top, px(100.0));

    state.press(Origin::Handle, point(px(100.0), px(100.0)), px(0.0), after);
    assert!(
        state
            .pan(
                point(px(100.0), px(400.0)),
                px(1000.0),
                after + Duration::from_secs(1),
            )
            .is_some()
    );
    assert_eq!(
        state.release(after + Duration::from_millis(1200)),
        Release::Pan(Stop::Compact)
    );
    assert!(matches!(
        settle(&mut state, after + Duration::from_millis(1200)),
        Frame::Visible(_)
    ));
    let after = after + Duration::from_secs(4);
    assert_eq!(visible(&mut state, after, false).top, px(400.0));

    state.press(Origin::Handle, point(px(100.0), px(400.0)), px(0.0), after);
    assert!(
        state
            .pan(
                point(px(100.0), px(900.0)),
                px(1000.0),
                after + Duration::from_secs(1),
            )
            .is_some()
    );
    assert_eq!(
        state.release(after + Duration::from_millis(1200)),
        Release::Pan(Stop::Closed)
    );
    assert!(matches!(
        settle(&mut state, after + Duration::from_millis(1200)),
        Frame::Closed
    ));
    state.present(after + Duration::from_secs(4));
    assert_eq!(
        visible(&mut state, after + Duration::from_secs(4), true).top,
        px(400.0)
    );
}

#[test]
fn a_fast_upward_pan_projects_to_expanded_before_reaching_it() {
    let (mut state, now) = opened();
    state.press(Origin::Handle, point(px(100.0), px(400.0)), px(0.0), now);
    assert!(
        state
            .pan(
                point(px(100.0), px(300.0)),
                px(1000.0),
                now + Duration::from_millis(20),
            )
            .is_some()
    );
    assert_eq!(
        state.release(now + Duration::from_millis(25)),
        Release::Pan(Stop::Expanded)
    );
}

#[test]
fn scrollable_content_reaches_its_top_before_the_sheet_collapses() {
    let (mut state, now) = opened();
    state.press(Origin::Handle, point(px(100.0), px(400.0)), px(0.0), now);
    assert!(
        state
            .pan(
                point(px(100.0), px(100.0)),
                px(1000.0),
                now + Duration::from_millis(200),
            )
            .is_some()
    );
    assert_eq!(
        state.release(now + Duration::from_millis(400)),
        Release::Pan(Stop::Expanded)
    );
    assert_eq!(
        visible(&mut state, now + Duration::from_millis(400), true).top,
        px(100.0)
    );
    let now = now + Duration::from_secs(1);

    state.press(Origin::Content, point(px(100.0), px(400.0)), px(200.0), now);
    assert_eq!(
        state.pan(
            point(px(100.0), px(500.0)),
            px(1000.0),
            now + Duration::from_millis(200)
        ),
        Some(px(100.0))
    );
    assert_eq!(
        visible(&mut state, now + Duration::from_millis(200), false).top,
        px(100.0)
    );
    assert_eq!(
        state.release(now + Duration::from_millis(400)),
        Release::Pan(Stop::Expanded)
    );
}

#[test]
fn content_expands_first_then_scrolls_without_collapsing_on_release() {
    let (mut state, now) = opened();
    state.press(Origin::Content, point(px(100.0), px(600.0)), px(0.0), now);
    assert_eq!(
        state.pan(
            point(px(100.0), px(100.0)),
            px(1000.0),
            now + Duration::from_millis(200)
        ),
        Some(px(200.0))
    );
    assert_eq!(
        state.release(now + Duration::from_millis(400)),
        Release::Pan(Stop::Expanded)
    );
}

#[test]
fn stationary_presses_remain_clicks_but_a_pan_is_consumed() {
    let (mut state, now) = opened();
    state.press(Origin::Content, point(px(100.0), px(500.0)), px(0.0), now);
    assert_eq!(
        state.pan(
            point(px(101.0), px(501.0)),
            px(1000.0),
            now + Duration::from_millis(20)
        ),
        None
    );
    assert_eq!(state.release(now + Duration::from_millis(30)), Release::Tap);
    state.press(Origin::Content, point(px(100.0), px(500.0)), px(0.0), now);
    assert!(
        state
            .pan(
                point(px(110.0), px(500.0)),
                px(1000.0),
                now + Duration::from_millis(20),
            )
            .is_some()
    );
    assert_eq!(
        state.release(now + Duration::from_millis(200)),
        Release::Pan(Stop::Compact)
    );
}

#[test]
fn reduced_motion_keeps_direct_pointer_movement_but_settles_immediately() {
    let (mut state, now) = opened();
    state.press(Origin::Handle, point(px(100.0), px(400.0)), px(0.0), now);
    assert!(
        state
            .pan(
                point(px(100.0), px(300.0)),
                px(1000.0),
                now + Duration::from_secs(1),
            )
            .is_some()
    );
    let frame = visible(&mut state, now + Duration::from_secs(1), true);
    assert_eq!(frame.top, px(300.0));
    assert_eq!(frame.movement, Movement::Dragging);
    assert_eq!(
        state.release(now + Duration::from_millis(1200)),
        Release::Pan(Stop::Compact)
    );
    assert_eq!(
        visible(&mut state, now + Duration::from_millis(1200), true).top,
        px(400.0)
    );
    state.close(now + Duration::from_secs(2));
    assert!(matches!(
        state.frame(px(1000.0), now + Duration::from_secs(2), true),
        Frame::Closed
    ));
}

#[test]
fn a_delayed_frame_advances_at_most_the_references_sixty_four_ms() {
    let now = Instant::now();
    let mut state = State::default();
    state.present(now);
    let frame = visible(&mut state, now + Duration::from_secs(1), false);
    assert!((f32::from(frame.top) - 804.430_1).abs() < 0.001);
}

#[test]
fn spring_values_match_samples_from_the_actual_reanimated_helper() {
    // Independently sampled from springUtils's critical branch and
    // termination with Gorhom's actual configuration, not a second Rust
    // implementation of this module's formulas.
    for (from, to, supplied, ms, position, velocity, resting) in [
        (
            1.0,
            Stop::Compact,
            0.0,
            64,
            0.804_430_1,
            -3.978_760_7,
            false,
        ),
        (
            1.0,
            Stop::Compact,
            0.0,
            150,
            0.545_043_2,
            -1.939_799_4,
            false,
        ),
        (1.0, Stop::Compact, 0.0, 250, 0.434_777, -0.520_829_9, false),
        (1.0, Stop::Compact, 0.0, 750, 0.4, 0.0, true),
        (
            0.4,
            Stop::Expanded,
            -2.5,
            150,
            0.148_274_1,
            -0.688_853,
            false,
        ),
        (
            0.4,
            Stop::Expanded,
            2.5,
            150,
            0.172_521_6,
            -0.969_899_7,
            false,
        ),
        (0.9, Stop::Closed, 0.0, 150, 0.975_826_1, 0.323_299_9, false),
    ] {
        let value =
            Spring::new(from, supplied, to).sample(Duration::from_millis(ms), 1000.0, false);
        assert_eq!(matches!(value, SpringValue::Resting(_)), resting);
        assert!((value.position() - position).abs() < 0.000_001);
        assert!((value.velocity() - velocity).abs() < 0.000_01);
    }
}

#[test]
fn entering_fade_runs_the_motion_tokens_and_completes_when_held() {
    // Word 39's tokens: the surface length on the kit's curve.
    assert!(fade_in(Duration::ZERO, false).abs() < 1e-4);
    assert_eq!(
        fade_in(crate::motion::STANDARD, false).to_bits(),
        1.0_f32.to_bits()
    );
    assert_eq!(fade_in(Duration::ZERO, true).to_bits(), 1.0_f32.to_bits());
    assert!(fade_in(Duration::from_millis(125), false) > 0.5);
}
