//! The slider's logic against the reference's numbers.
#![expect(
    clippy::float_arithmetic,
    reason = "checking float formulas against their expected values"
)]

use super::flair::{self, FastStops};
use super::motion::{EASE, GLIDE, GROW, SETTLE, SLIDE, Spring, Tween, Wheel};
use super::{along, commits, fraction, stop_under};

fn near(a: f32, b: f32, within: f32) -> bool {
    (a - b).abs() <= within
}

/// The stop changes as the pointer crosses a midpoint of the whole width.
#[test]
fn the_pointer_picks_the_stop_by_rounding_over_the_whole_width() {
    // Five stops over 200 points: midpoints at 25, 75, 125, 175.
    assert_eq!(stop_under(24.0, 200.0, 5), 0);
    assert_eq!(stop_under(26.0, 200.0, 5), 1);
    assert_eq!(stop_under(124.0, 200.0, 5), 2);
    assert_eq!(stop_under(126.0, 200.0, 5), 3);
    assert_eq!(stop_under(-40.0, 200.0, 5), 0);
    assert_eq!(stop_under(400.0, 200.0, 5), 4);
    assert_eq!(stop_under(80.0, 200.0, 1), 0);
    assert_eq!(stop_under(80.0, 0.0, 5), 0);
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
