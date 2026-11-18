//! A content-sized sheet (TRACKER 799, the menu's and the list's): its
//! positions measured against its own height, one open stop and closed.

use std::time::{Duration, Instant};

use gpui::{point, px};

use super::{Frame, Geometry, Origin, Release, SnapPoints, State, Stop};

const HEIGHT: f32 = 300.0;

fn visible(state: &mut State, now: Instant) -> Geometry {
    match state.frame(px(HEIGHT), now, false) {
        Frame::Visible(frame) => frame,
        frame @ (Frame::PendingLayout | Frame::Closed) => {
            panic!("expected a visible sheet, got {frame:?}")
        }
    }
}

fn after(now: Instant, millis: u64) -> Instant {
    now.checked_add(Duration::from_millis(millis)).unwrap()
}

/// At rest at its open stop, as a menu sheet appears.
fn opened() -> (State, Instant) {
    let now = Instant::now();
    let mut state = State::with_points(SnapPoints::CONTENT);
    state.retain_compact();
    let frame = visible(&mut state, now);
    assert_eq!(frame.top, px(0.0));
    assert_eq!(frame.backdrop.to_bits(), 1.0_f32.to_bits());
    (state, now)
}

/// Drag the handle down by `distance`, slowly, then hold before letting go.
fn slow_pan(state: &mut State, now: Instant, distance: f32) -> Release {
    state.press(Origin::Handle, point(px(100.0), px(10.0)), px(0.0), now);
    for step in 1_u8..=10 {
        let at = after(now, u64::from(step).saturating_mul(40));
        let y = distance.mul_add(f32::from(step).mul_add(0.1, 0.0), 10.0);
        let _scroll = state.pan(point(px(100.0), px(y)), px(0.0), at);
    }
    state.release(after(now, 900))
}

#[test]
fn a_short_pan_springs_back_to_its_open_stop() {
    let (mut state, now) = opened();
    assert_eq!(slow_pan(&mut state, now, 40.0), Release::Pan(Stop::Compact));
}

#[test]
fn a_pan_past_half_its_height_closes_it() {
    let (mut state, now) = opened();
    assert_eq!(slow_pan(&mut state, now, 200.0), Release::Pan(Stop::Closed));
}

#[test]
fn a_flick_down_closes_it_and_the_dim_follows_the_drag() {
    let (mut state, now) = opened();
    state.press(Origin::Handle, point(px(100.0), px(10.0)), px(0.0), now);
    let half = after(now, 20);
    let _scroll = state.pan(point(px(100.0), px(160.0)), px(0.0), half);
    let held = visible(&mut state, half);
    assert!(held.top > px(140.0) && held.top < px(160.0));
    assert!(held.backdrop > 0.45 && held.backdrop < 0.55);
    let _scroll = state.pan(point(px(100.0), px(170.0)), px(0.0), after(now, 30));
    assert_eq!(state.release(after(now, 32)), Release::Pan(Stop::Closed));
}
