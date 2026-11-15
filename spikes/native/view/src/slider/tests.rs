//! The slider's logic against the reference's numbers.
#![expect(
    clippy::float_arithmetic,
    reason = "checking float formulas against their expected values"
)]

use super::drag::{Pointer, RELEASE, STICK, magnet, nearest};
use super::flair::{self, FastStops};
use super::motion::{EASE, GLIDE, GROW, SETTLE, SLIDE, Spring, Tween, Wheel};
use super::state::{Input, Motion};
use super::{INSET, along, commits, fraction};

fn near(a: f32, b: f32, within: f32) -> bool {
    (a - b).abs() <= within
}

/// The track the drag tests use: 260 wide, so 234 between the end stops;
/// six stops 46.8 apart.
const WIDTH: f32 = 260.0;

/// A pointer `x` points from the track's start.
const fn at(x: f32) -> Pointer {
    Pointer { x, width: WIDTH }
}

/// Where stop `index` of six is, points from the track's start.
fn stop_x(index: usize) -> f32 {
    along(fraction(index, 6), WIDTH)
}

/// The stop under a pointer is the nearest along the line between the
/// end stops; the ends hold past the track.
#[test]
fn the_pointer_picks_the_nearest_stop() {
    let line = WIDTH - 2.0 * INSET;
    assert!((at(INSET).fraction()).abs() < 1e-6);
    assert!((at(WIDTH - INSET).fraction() - 1.0).abs() < 1e-6);
    assert!((at(-40.0).fraction()).abs() < 1e-6);
    let midpoint = INSET + line * 0.1;
    assert_eq!(nearest(at(midpoint - 0.5).fraction(), 6), 0);
    assert_eq!(nearest(at(midpoint + 0.5).fraction(), 6), 1);
    assert_eq!(nearest(1.0, 6), 5);
    assert_eq!(nearest(0.7, 1), 0);
}

/// The pull: on the stop within the sticky zone, behind the pointer
/// leaving it, with the pointer at the midpoint, ahead of it arriving at
/// the next stop, and never moving back as the pointer goes on.
#[test]
fn the_magnet_holds_then_pulls_to_the_next_stop() {
    let line = WIDTH - 2.0 * INSET;
    let drawn = |x: f32| magnet(at(x).fraction(), 6, line);
    let stop = fraction(1, 6);
    assert!(near(drawn(stop_x(1) + STICK - 0.5), stop, 1e-6), "held");
    assert!(near(drawn(stop_x(1) - STICK + 0.5), stop, 1e-6), "held");
    let leaving = stop_x(1) + 14.0;
    assert!(
        drawn(leaving) < at(leaving).fraction(),
        "behind the pointer"
    );
    let midpoint = f32::midpoint(stop_x(1), stop_x(2));
    assert!(near(drawn(midpoint), at(midpoint).fraction(), 1e-4));
    let arriving = stop_x(2) - 14.0;
    assert!(drawn(arriving) > at(arriving).fraction(), "ahead of it");
    let mut last = 0.0_f32;
    for step in 0..=520_u16 {
        let place = drawn(f32::from(step) / 2.0);
        assert!(place + 1e-6 >= last, "never back at {step}");
        assert!(place - last < 0.03, "no jump at {step}: {last} -> {place}");
        last = place;
    }
}

/// Stops sit 13 in from either end; a drag sends only a change.
#[test]
fn stops_sit_between_the_insets_and_a_drag_commits_only_a_change() {
    assert!(near(along(fraction(0, 6), 230.0), 13.0, 1e-4));
    assert!(near(along(fraction(5, 6), 230.0), 217.0, 1e-4));
    assert!(near(along(fraction(2, 5), 230.0), 115.0, 1e-4));
    assert!(near(fraction(3, 1), 0.0, 1e-6));
    assert!(commits(Some(2), 3));
    assert!(!commits(Some(3), 3));
    assert!(commits(None, 0));
}

/// The reference's curves: the ends fixed, CSS `ease` at half time near
/// its known 0.802, the slide fast out of the gate.
#[test]
fn the_curves_match_css() {
    for curve in [SLIDE, EASE] {
        assert!(near(curve.at(0.0), 0.0, 1e-4));
        assert!(near(curve.at(1.0), 1.0, 1e-4));
        let mut last = 0.0;
        for step in 1..=20_u8 {
            let value = curve.at(f32::from(step) / 20.0);
            assert!(value >= last - 1e-4, "monotonic");
            last = value;
        }
    }
    assert!(near(EASE.at(0.5), 0.802, 0.003));
    assert!(SLIDE.at(0.25) > 0.75, "{}", SLIDE.at(0.25));
}

/// A glide retargeted mid-way starts from where the thumb is.
#[test]
fn a_glide_starts_from_where_it_is() {
    let mut tween = Tween::rest(0.0);
    tween.glide(1.0, 10.0, GLIDE);
    assert!(near(tween.at(10.0), 0.0, 1e-4));
    assert!(tween.moving(10.1));
    let halfway = tween.at(10.05);
    assert!(halfway > 0.0 && halfway < 1.0);
    tween.glide(0.5, 10.05, GLIDE);
    assert!(near(tween.at(10.05), halfway, 1e-4));
    assert!(near(tween.at(11.0), 0.5, 1e-4));
    assert!(!tween.moving(11.0));
    tween.glide(0.2, 12.0, 0.0);
    assert!(near(tween.at(12.0), 0.2, 1e-4), "reduced motion jumps");
}

/// The thumb's springs settle quickly with a small overshoot at most.
#[test]
fn the_thumb_springs_settle() {
    let grown = 32.0 / 28.0;
    let mut spring = Spring::rest(1.0);
    let mut peak = 1.0_f32;
    for _ in 0..30 {
        spring.step(grown, GROW, 1.0 / 60.0);
        peak = peak.max(spring.value);
    }
    assert!(spring.settled(grown), "{spring:?}");
    assert!(peak < grown + 0.01, "overshoot {peak}");
    for _ in 0..60 {
        spring.step(1.0, SETTLE, 1.0 / 60.0);
    }
    assert!(spring.settled(1.0), "{spring:?}");
}

/// A stop per 30 units in one direction; a pause or a turn starts over.
#[test]
fn the_wheel_counts_thirty_units_a_stop() {
    let mut wheel = Wheel::default();
    assert_eq!(wheel.add(10.0, 0.0), 0);
    assert_eq!(wheel.add(10.0, 0.05), 0);
    assert_eq!(wheel.add(10.0, 0.10), 1);
    assert_eq!(wheel.add(20.0, 0.5), 0, "after a pause it starts over");
    assert_eq!(wheel.add(-20.0, 0.55), 0, "a turn starts over");
    assert_eq!(wheel.add(-90.0, 0.6), -3);
}

/// The burst: invisible at a quarter size, fully seen at 22 %, gone at
/// the end, each particle ending at its own place.
#[test]
fn the_burst_flies_out_and_ends() {
    let start = flair::burst(0, 0.0).expect("on");
    assert!(near(start.opacity, 0.0, 1e-4) && near(start.scale, 0.25, 1e-4));
    let peak = flair::burst(0, 0.22 * flair::BURST_TIME).expect("on");
    assert!(near(peak.opacity, 1.0, 1e-3) && near(peak.scale, 1.28, 1e-3));
    let late = flair::burst(0, 0.6).expect("on");
    assert!(late.y < -30.0, "{late:?}");
    assert!(flair::burst(0, 0.7).is_none());
    assert!(flair::burst(16, 0.1).is_none());
    assert!(near(flair::burst_blue(2), 0.18, 1e-6));
    assert!(near(flair::burst_blue(4), 0.32, 1e-6));
    assert!(near(flair::burst_blue(0), 0.0, 1e-6));
}

/// Fast's dots stream from the thumb toward the start, fading in and out.
#[test]
fn fast_dots_stream_back_from_the_thumb() {
    for index in 0..flair::FAST_DOTS {
        let dot = flair::fast_dot(index, 0.4, 0.5);
        assert!((0.0..1.0).contains(&dot.x));
        assert!((0.12..=0.88).contains(&dot.y));
        assert!((0.5..=0.95).contains(&dot.scale));
        assert!((0.0..=1.0).contains(&dot.opacity));
    }
    let a = flair::fast_dot(3, 1.0, 0.5);
    let b = flair::fast_dot(3, 1.1, 0.5);
    assert!(b.x > a.x || b.x < 0.1, "it moves back from the thumb");
}

/// Fast on: the filled stops slide 85 back, shrink and fade; the stops
/// above the fill stay. At the top stop all of them fade.
#[test]
fn the_stops_step_aside() {
    let gone = flair::stop(true, None, FastStops::On { since: 2.0 });
    assert!(near(gone.x, -85.0, 1e-3) && near(gone.opacity, 0.0, 1e-3));
    assert!(near(gone.scale, 0.75, 1e-3));
    let open = flair::stop(false, None, FastStops::On { since: 2.0 });
    assert!(near(open.opacity, 1.0, 1e-6));
    let back = flair::stop(true, None, FastStops::Back { since: 1.0 });
    assert!(near(back.opacity, 1.0, 1e-3) && near(back.x, 0.0, 1e-3));
    let top = flair::stop(false, Some(1.0), FastStops::Off);
    assert!(near(top.opacity, 0.0, 1e-3) && near(top.scale, 0.75, 1e-3));
    let leaving = flair::stop_leaving_top(1.0);
    assert!(near(leaving.opacity, 1.0, 1e-3));
}

/// The field stays in range, moves with time, and the blend keeps the
/// backdrop's hue with the field's light.
#[test]
fn the_field_is_lit_by_its_blobs() {
    let a = flair::field(0.3, 0.5, 1.0, 0.0);
    let b = flair::field(0.3, 0.5, 2.0, 0.0);
    assert!(a.iter().all(|channel| (0.0..=1.0).contains(channel)));
    assert!(a.iter().zip(b).any(|(x, y)| !near(*x, y, 1e-4)), "it moves");
    let purple = [0.678, 0.482, 0.976];
    let blue = [0.2, 0.612, 1.0];
    let under = flair::backdrop(0.55, blue, purple);
    assert!(under.iter().zip(purple).all(|(x, y)| near(*x, y, 1e-6)));
    let blended = flair::luminosity(under, a);
    let lum = |[r, g, b]: [f32; 3]| 0.3 * r + 0.59 * g + 0.11 * b;
    assert!(near(lum(blended), lum(a), 0.02), "{blended:?} {a:?}");
}

/// The reveal sweeps from the thumb's end to the start over 2 s.
#[test]
fn the_field_is_revealed_from_the_thumb() {
    assert!(near(flair::reveal(10.0, 200.0, 0.0), 0.0, 1e-6));
    assert!(near(
        flair::reveal(199.0, 200.0, 0.0),
        1.0 - 1.0 / 120.0,
        1e-3
    ));
    assert!(near(flair::reveal(0.0, 200.0, 2.0), 1.0, 1e-6));
    assert!(flair::reveal(150.0, 200.0, 0.5) > flair::reveal(50.0, 200.0, 0.5));
}

/// The warning's light band crosses once.
#[test]
fn the_warning_shimmers_once() {
    assert!(near(flair::shimmer(0.5, 0.0), 0.0, 1e-6));
    assert!(near(flair::shimmer(0.5, 2.0), 0.0, 1e-6));
    let lit = (0..=220_u8)
        .map(|step| flair::shimmer(0.5, f32::from(step) * 0.005))
        .fold(0.0_f32, f32::max);
    assert!(lit > 0.5, "{lit}");
}

fn input(current: Option<usize>, fast: bool, reduce: bool) -> Input {
    Input {
        stops: 6,
        current,
        maximum: Some(5),
        fast,
        reduce,
    }
}

/// A drag across stops sends one choice, on release, and only a change.
#[test]
fn a_drag_previews_live_and_sends_once() {
    let mut motion = Motion::default();
    motion.press(at(stop_x(2)), 6, Some(2));
    motion.drag(at(stop_x(3)), 6);
    motion.drag(at(stop_x(4)), 6);
    assert_eq!(motion.shown(Some(2)), Some(4), "the word follows live");
    assert_eq!(motion.release(), Some(4));
    motion.press(at(stop_x(4)), 6, Some(4));
    motion.drag(at(stop_x(3)), 6);
    motion.drag(at(stop_x(4)), 6);
    assert_eq!(motion.release(), None, "back where it began: nothing sent");
    assert_eq!(motion.release(), None);
}

/// A scripted drag at 60 frames a second, the pointer 2 points a frame
/// from stop 1 to stop 4: the thumb follows frame by frame (no step larger
/// than the pull allows), never goes back, the word changes at each
/// midpoint, and on release it springs onto stop 4 in under 0.3 s
/// without passing it by more than a point.
#[test]
fn a_dragged_thumb_follows_then_springs_onto_its_stop() {
    let frame = 1.0 / 60.0;
    let line = WIDTH - 2.0 * INSET;
    let mut motion = Motion::default();
    let mut now = 0.0;
    let _rest = motion.frame(input(Some(1), false, false), now);
    motion.press(at(stop_x(1)), 6, Some(1));
    let mut x = stop_x(1);
    let mut last = fraction(1, 6);
    let mut shown = Vec::new();
    // Released 10 to 12 points short of stop 4: past its sticky zone, so
    // the thumb is not on the stop yet.
    while x < stop_x(4) - 12.0 {
        x += 2.0;
        now += frame;
        motion.drag(at(x), 6);
        let drawn = motion.frame(input(Some(1), false, false), now);
        let place = drawn.fraction.expect("a place");
        assert!(place + 1e-6 >= last, "never back");
        assert!(
            (place - last) * line < 6.0,
            "a step of {}",
            (place - last) * line
        );
        last = place;
        shown.push(motion.shown(Some(1)));
    }
    shown.dedup();
    assert_eq!(shown, vec![Some(1), Some(2), Some(3), Some(4)]);
    let target = fraction(4, 6);
    assert!(last < target, "short of the stop at release");
    assert_eq!(motion.release(), Some(4));
    // Settled: within a quarter point of the stop from then on. (The
    // thumb's shrink after the release, a separate spring, runs a little
    // longer.)
    let mut settled = None;
    let mut furthest = last;
    let mut resting = false;
    for step in 1..=60_u8 {
        now += frame;
        let drawn = motion.frame(input(Some(4), false, false), now);
        let place = drawn.fraction.expect("a place");
        furthest = furthest.max(place);
        let close = (place - target).abs() * line < 0.25;
        if close && settled.is_none() {
            settled = Some(f32::from(step) * frame);
        }
        assert!(close || settled.is_none(), "it left the stop again");
        resting = !drawn.moving;
    }
    let settled = settled.expect("it comes to rest");
    assert!(settled < 0.3, "settled after {settled} s");
    assert!(resting, "nothing moves a second later");
    assert!(
        (furthest - target) * line < 1.0,
        "overshoot {}",
        (furthest - target) * line
    );
}

/// The release spring is critically damped: from 30 points away it
/// passes its stop by under a tenth of a point.
#[test]
fn the_release_spring_does_not_overshoot() {
    let mut spring = Spring::rest(0.0);
    let mut furthest = 0.0_f32;
    for _ in 0..60 {
        spring.step(30.0, RELEASE, 1.0 / 60.0);
        furthest = furthest.max(spring.value);
    }
    assert!(spring.settled(30.0));
    assert!(furthest < 30.1, "{furthest}");
}

/// A press then a move begins away from the thumb: the thumb closes the
/// gap instead of jumping; a press without a move glides like a click;
/// with reduced motion the thumb stays on stops.
#[test]
fn a_drag_catches_up_and_a_click_glides() {
    let frame = 1.0 / 60.0;
    let line = WIDTH - 2.0 * INSET;
    let mut motion = Motion::default();
    let _rest = motion.frame(input(Some(0), false, false), 0.0);
    motion.press(at(stop_x(3) + 12.0), 6, Some(0));
    motion.drag(at(stop_x(3) + 16.0), 6);
    let first = motion.frame(input(Some(0), false, false), frame);
    let pulled = magnet(at(stop_x(3) + 16.0).fraction(), 6, line);
    let place = first.fraction.expect("a place");
    assert!(place < pulled - 0.1, "starts near the thumb: {place}");
    assert!(first.moving);
    let later = motion.frame(input(Some(0), false, false), frame * 30.0);
    assert!(
        near(later.fraction.expect("a place"), pulled, 1e-3),
        "caught up"
    );
    let _sent = motion.release();
    let mut click = Motion::default();
    let _rest = click.frame(input(Some(0), false, false), 0.0);
    click.press(at(stop_x(5)), 6, Some(0));
    let _started = click.frame(input(Some(0), false, false), frame);
    let halfway = click.frame(input(Some(0), false, false), frame + GLIDE / 2.0);
    let x = halfway.fraction.expect("a place");
    assert!(x > 0.0 && x < 1.0 && halfway.moving, "{x}");
    assert_eq!(click.release(), Some(5));
    let mut still = Motion::default();
    let _rest = still.frame(input(Some(0), false, true), 0.0);
    still.press(at(stop_x(0)), 6, Some(0));
    still.drag(at(stop_x(2) - 4.0), 6);
    let held = still.frame(input(Some(0), false, true), frame);
    assert!(near(held.fraction.expect("a place"), fraction(2, 6), 1e-6));
    assert!(!held.moving);
}

/// The thumb glides to a new stop and rests; with reduced motion it
/// jumps and nothing asks for frames.
#[test]
fn the_thumb_glides_then_rests() {
    let mut motion = Motion::default();
    let first = motion.frame(input(Some(1), false, false), 0.0);
    assert!(near(first.fraction.expect("a stop"), 0.2, 1e-4));
    assert!(!first.moving, "at rest on opening");
    let going = motion.frame(input(Some(3), false, false), 0.1);
    assert!(going.moving);
    let halfway = motion.frame(input(Some(3), false, false), 0.2);
    let x = halfway.fraction.expect("a stop");
    assert!(x > 0.2 && x < 0.6, "{x}");
    let done = motion.frame(input(Some(3), false, false), 1.0);
    assert!(near(done.fraction.expect("a stop"), 0.6, 1e-4));
    assert!(!done.moving);
    let mut still = Motion::default();
    let _first = still.frame(input(Some(1), false, true), 0.0);
    let jumped = still.frame(input(Some(3), false, true), 0.1);
    assert!(near(jumped.fraction.expect("a stop"), 0.6, 1e-4));
    assert!(!jumped.moving);
}

/// Arriving on the top stop bursts and reveals the field; the field and
/// Fast's dots keep the frames coming, reduced motion stops them.
#[test]
fn the_top_stop_and_fast_animate_until_reduced() {
    let mut motion = Motion::default();
    let _below = motion.frame(input(Some(4), false, false), 0.0);
    let top = motion.frame(input(Some(5), false, false), 0.1);
    assert!(top.at_maximum && !top.burst.is_empty());
    assert_eq!(top.field.map(|(_, reveal)| reveal), Some(0.0));
    assert!(top.moving);
    let held = motion.frame(input(Some(5), false, false), 1.5);
    assert!(held.burst.is_empty() && held.moving, "the field moves on");
    let rested = motion.frame(input(Some(5), false, false), 5.0);
    assert!(!rested.moving, "left alone, it comes to rest");
    motion.engage(true, 5.0);
    let engaged = motion.frame(input(Some(5), false, false), 5.1);
    assert!(engaged.moving, "the pointer back: it moves");
    let fast = motion.frame(input(Some(2), true, false), 6.0);
    assert_eq!(fast.dots.len(), flair::FAST_DOTS);
    let mut still = Motion::default();
    let calm = still.frame(input(Some(5), true, true), 0.0);
    assert!(calm.at_maximum && calm.dots.is_empty() && !calm.moving);
    assert!(
        calm.stops
            .iter()
            .all(|(dot, _)| near(dot.opacity, 0.0, 1e-3))
    );
}

/// While dragged onto the top stop the warning shows; closing the card
/// drops everything.
#[test]
fn the_warning_shows_while_dragged_to_the_top() {
    let mut motion = Motion::default();
    let _rest = motion.frame(input(Some(3), false, false), 0.0);
    motion.press(at(stop_x(3)), 6, Some(3));
    motion.drag(at(stop_x(5)), 6);
    let dragged = motion.frame(input(Some(3), false, false), 0.1);
    assert_eq!(dragged.warning, Some(0.0));
    assert!(motion.dragging());
    motion.close();
    assert!(!motion.dragging());
    let reopened = motion.frame(input(Some(3), false, false), 9.0);
    assert!(reopened.warning.is_none() && !reopened.moving);
}

/// The decorations' clock slows to a stop a few seconds after the pointer
/// leaves the card, and the dots fade as it slows.
#[test]
fn the_decorations_come_to_rest_after_the_pointer_leaves() {
    let mut motion = Motion::default();
    motion.engage(true, 0.0);
    let on = motion.frame(input(Some(2), true, false), 0.0);
    assert!(on.moving && !on.dots.is_empty());
    motion.engage(false, 1.0);
    let held = motion.frame(input(Some(2), true, false), 2.5);
    assert!(held.moving, "2 s after it leaves, still going");
    let slowing = motion.frame(input(Some(2), true, false), 3.6);
    let faded = slowing
        .dots
        .iter()
        .map(|dot| dot.opacity)
        .fold(0.0_f32, f32::max);
    assert!(slowing.moving && faded < 0.9, "{faded}");
    let still = motion.frame(input(Some(2), true, false), 6.0);
    assert!(!still.moving && still.dots.is_empty());
}
