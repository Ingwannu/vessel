//! The drawing's geometry against the mark's ratios and the real screens'
//! measured boxes (the lead's build, captured at 1280 × 800, 1920 × 1080
//! and 390 × 844).
#![expect(
    clippy::float_arithmetic,
    reason = "tests measure distances between points"
)]

use super::layout::{NewSession, Session, new_session, session};
use super::motion::{Journey, between, ended, journey};
use super::{Area, Figure, Pt, Star, geometry, ratios, shapes};

fn area(x0: f32, y0: f32, x1: f32, y1: f32) -> Area {
    Area {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
    }
}

/// The new-session screen as measured at 1280 × 800.
fn new_1280() -> NewSession {
    NewSession {
        content: area(264.0, 36.0, 1271.0, 791.0),
        composer: area(358.0, 382.0, 1177.0, 497.0),
        heading: area(358.0, 346.0, 532.0, 365.0),
        pickers: area(377.0, 516.0, 586.0, 529.0),
    }
}

fn new_1920() -> NewSession {
    NewSession {
        content: area(264.0, 36.0, 1911.0, 1071.0),
        composer: area(678.0, 522.0, 1497.0, 637.0),
        heading: area(678.0, 486.0, 852.0, 505.0),
        pickers: area(697.0, 656.0, 906.0, 669.0),
    }
}

fn new_390() -> NewSession {
    NewSession {
        content: area(8.0, 36.0, 381.0, 835.0),
        composer: area(21.0, 394.0, 368.0, 513.0),
        heading: area(21.0, 358.0, 195.0, 377.0),
        pickers: area(40.0, 540.0, 257.0, 555.0),
    }
}

fn session_at(content: Area, left: f32, right: f32) -> Session {
    Session {
        content,
        column_left: left,
        column_right: right,
        air: ratios::CLEARANCE,
    }
}

/// The distance from `p` to the nearest point of `box_` (0 inside).
fn distance(p: Pt, box_: Area) -> f32 {
    let dx = (box_.x - p.x).max(p.x - box_.right()).max(0.0);
    let dy = (box_.y - p.y).max(p.y - box_.bottom()).max(0.0);
    dx.hypot(dy)
}

/// The least distance from any stroked line's outer edge to `box_`.
fn clearance(figure: &Figure, box_: Area) -> f32 {
    shapes(figure)
        .iter()
        .flat_map(|shape| {
            let half = shape.stroke.unwrap_or(0.0) / 2.0;
            shape.points.iter().map(move |p| distance(*p, box_) - half)
        })
        .fold(f32::INFINITY, f32::min)
}

#[test]
fn the_pitch_follows_the_content_width() {
    assert!((ratios::pitch_for(1007.0) - 7.0).abs() < 1e-6);
    assert!((ratios::pitch_for(1647.0) - 8.0).abs() < 1e-6);
    assert!((ratios::pitch_for(373.0) - 6.0).abs() < 1e-6);
    // the mark's line : gap stays 19 : 25.4 at every pitch
    assert!((ratios::GAP / ratios::LINE - 25.4 / 19.0).abs() < 1e-6);
}

#[test]
fn offsets_stay_one_pitch_apart_through_a_turn() {
    let line = [
        Pt { x: 0.0, y: 0.0 },
        Pt { x: 100.0, y: 173.2 },
        Pt { x: 200.0, y: 0.0 },
    ];
    let pitch = 7.0;
    let radius = 28.0;
    let inner = geometry::offset(&line, radius, -0.5 * pitch);
    let outer = geometry::offset(&line, radius, 0.5 * pitch);
    // every point of the outer line lies one pitch from the inner line
    for p in &outer {
        let nearest = inner
            .iter()
            .map(|q| (p.x - q.x).hypot(p.y - q.y))
            .fold(f32::INFINITY, f32::min);
        assert!((nearest - pitch).abs() < 0.2, "{nearest}");
    }
}

#[test]
fn the_innermost_loop_holds_one_gap() {
    let line = [Pt { x: 0.0, y: 0.0 }, Pt { x: 100.0, y: 0.0 }];
    let pitch = 7.0;
    let ring = geometry::loop_around(&line, 28.0, pitch / 2.0);
    let top = ring.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let bottom = ring.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
    // centre lines one pitch apart, so the hole is a pitch less a line
    assert!(((bottom - top) - pitch).abs() < 1e-3);
    let hole = (bottom - top) - ratios::LINE * pitch;
    assert!((hole - ratios::GAP * pitch).abs() < 1e-3);
}

#[test]
fn the_star_relaxes_into_the_circle_through_its_tips() {
    let centre = Pt { x: 50.0, y: 50.0 };
    let star = geometry::star(centre, 100.0, 0.0);
    let tip = star
        .iter()
        .map(|p| (p.x - 50.0).hypot(p.y - 50.0))
        .fold(0.0, f32::max);
    assert!((tip - 50.0).abs() < 0.1, "{tip}");
    for p in geometry::star(centre, 100.0, 1.0) {
        assert!(((p.x - 50.0).hypot(p.y - 50.0) - 50.0).abs() < 0.1);
    }
}

#[test]
fn the_wide_new_session_never_comes_near_text() {
    for screen in [new_1280(), new_1920()] {
        let figure = new_session(&screen);
        let v = figure.v.expect("a wide window draws the whole V");
        assert_eq!(v.loops, 4);
        assert!(figure.star.is_some());
        assert!(
            v.arms.apex.y > screen.pickers.bottom(),
            "the apex under the pickers"
        );
        for box_ in [screen.heading, screen.pickers] {
            let air = clearance(&figure, box_);
            assert!(air >= ratios::CLEARANCE - 0.5, "{air}");
        }
        let star = figure.star.expect("star");
        assert!(star.centre.y + star.span / 2.0 <= screen.heading.y.min(screen.composer.y));
    }
}

#[test]
fn a_phone_draws_the_star_above_and_the_bottom_below() {
    let screen = new_390();
    let figure = new_session(&screen);
    let star = figure.star.expect("the star above the heading");
    assert!(star.centre.y + star.span / 2.0 <= screen.heading.y - ratios::CLEARANCE + 0.5);
    let v = figure.v.expect("the V's bottom");
    assert_eq!(v.loops, 3);
    for box_ in [screen.heading, screen.pickers, screen.composer] {
        assert!(clearance(&figure, box_) >= ratios::CLEARANCE - 0.5);
    }
}

#[test]
fn the_frame_takes_what_the_margin_holds() {
    let wide = session(&session_at(
        area(264.0, 73.0, 1911.0, 1071.0),
        678.0,
        1497.0,
    ));
    assert_eq!(wide.v.map(|v| v.loops), Some(4));
    let mid = session(&session_at(area(264.0, 73.0, 1271.0, 791.0), 358.0, 1177.0));
    assert_eq!(mid.v.map(|v| v.loops), Some(4));
    let phone = session(&session_at(area(8.0, 81.0, 381.0, 835.0), 25.0, 364.0));
    assert_eq!(phone, Figure::default());
    // the frame keeps its air from the column (the avatar is inside it)
    let column = area(358.0, 73.0, 1177.0, 791.0);
    assert!(clearance(&mid, column) >= ratios::CLEARANCE - 0.5);
}

#[test]
fn the_journey_is_bounded_and_still_means_done() {
    assert_eq!(journey(0.1, true), Journey::DONE);
    let start = journey(0.0, false);
    assert!(start.moved.abs() < 1e-6 && start.morphed.abs() < 1e-6 && start.marked.abs() < 1e-6);
    assert_eq!(journey(0.6, false), Journey::DONE);
    assert!(!ended(0.59) && ended(0.6));
}

#[test]
fn the_transition_starts_and_ends_on_the_two_screens() {
    let from = new_session(&new_1280());
    let to = session(&session_at(area(264.0, 73.0, 1271.0, 791.0), 358.0, 1177.0));
    let avatar = Star {
        centre: Pt { x: 370.0, y: 164.0 },
        span: 24.0,
    };
    let at = |moved| Journey {
        moved,
        morphed: 0.0,
        marked: 0.0,
    };
    let first = between(&from, &to, avatar, at(0.0));
    assert_eq!(first.v.map(|v| v.arms), from.v.map(|v| v.arms));
    assert_eq!(first.star, from.star);
    let last = between(&from, &to, avatar, at(1.0));
    assert_eq!(last.v.map(|v| v.arms), to.v.map(|v| v.arms));
    assert_eq!(last.v.map(|v| (v.loops, v.outer)), Some((4, 1.0)));
    assert_eq!(last.star, None, "the avatar has it now");
}

#[test]
fn a_loop_only_one_screen_has_fades_out() {
    let from = new_session(&new_1280());
    // a narrower margin holds three loops a side
    let to = session(&session_at(area(264.0, 73.0, 1271.0, 791.0), 330.0, 1205.0));
    assert_eq!(to.v.map(|v| v.loops), Some(3));
    let avatar = Star {
        centre: Pt { x: 342.0, y: 164.0 },
        span: 24.0,
    };
    let at = |moved| Journey {
        moved,
        morphed: 0.0,
        marked: 0.0,
    };
    let half = between(&from, &to, avatar, at(0.5));
    assert_eq!(half.v.map(|v| (v.loops, v.outer)), Some((4, 0.5)));
    let last = between(&from, &to, avatar, at(1.0));
    assert_eq!(last.v.map(|v| (v.loops, v.outer)), Some((4, 0.0)));
}

/// The frame's left arm's lean from the vertical, in degrees.
fn lean(figure: &Figure) -> f32 {
    let arms = figure.v.map(|v| v.arms).expect("a frame");
    (arms.apex.x - arms.left.x)
        .atan2(arms.apex.y - arms.left.y)
        .to_degrees()
}

#[test]
fn the_frame_leans_in_as_the_sketch_does() {
    // 2560 x 1440: the margin would allow about 28°; the arms stop at the
    // sketch's lean and end short of the column
    let wide = session(&session_at(
        area(264.0, 73.0, 2551.0, 1431.0),
        998.0,
        1818.0,
    ));
    assert!((lean(&wide) - ratios::FRAME_LEAN_DEGREES).abs() < 0.5);
    let column = area(998.0, 73.0, 1818.0, 1431.0);
    assert!(clearance(&wide, column) > 100.0, "ends in the margin");
    // 1920 x 1080: the margin itself allows about 19°
    let full_hd = session(&session_at(
        area(264.0, 73.0, 1911.0, 1071.0),
        678.0,
        1498.0,
    ));
    assert!((15.0..=ratios::FRAME_LEAN_DEGREES).contains(&lean(&full_hd)));
    // 1280: the margin allows only a few degrees
    let narrow = session(&session_at(area(264.0, 73.0, 1271.0, 791.0), 358.0, 1177.0));
    assert!(lean(&narrow) < 5.0);
}

#[test]
fn a_union_holds_both_boxes() {
    let pickers = area(377.0, 516.0, 586.0, 529.0);
    let note = area(358.0, 541.0, 640.0, 556.0);
    assert_eq!(pickers.union(note), area(358.0, 516.0, 640.0, 556.0));
}
