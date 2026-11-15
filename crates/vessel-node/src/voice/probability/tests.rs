use super::super::boundary::{Boundary, BoundaryEvent};
use super::*;
fn score(value: f32) -> Probability {
    Probability::new(value).unwrap()
}
fn confirmed() -> Gate {
    let mut gate = Gate::default();
    for _ in 0..4 {
        assert_eq!(gate.observe(score(1.0)), Ok(Detection::Quiet));
    }
    assert_eq!(gate.observe(score(1.0)), Ok(Detection::Speech));
    gate
}
#[test]
fn internal_minimum_speech_and_silence_use_sample_counts_without_wall_clock() {
    let mut gate = confirmed();
    for _ in 0..7 {
        assert_eq!(gate.observe(score(0.0)), Ok(Detection::Speech));
    }
    assert_eq!(gate.observe(score(0.0)), Ok(Detection::Quiet));
    gate.reset();
    assert_eq!(gate.observe(score(1.0)), Ok(Detection::Quiet));
}
#[test]
fn equality_and_hysteresis_keep_the_pinned_edges_including_a_sticky_silence_start() {
    let mut gate = Gate::default();
    assert_eq!(gate.observe(score(1.0)), Ok(Detection::Quiet));
    for _ in 0..3 {
        assert_eq!(gate.observe(score(0.5)), Ok(Detection::Quiet));
    }
    assert_eq!(gate.observe(score(1.0)), Ok(Detection::Speech));
    assert_eq!(gate.observe(score(0.0)), Ok(Detection::Speech));
    for _ in 0..6 {
        assert_eq!(gate.observe(score(0.4)), Ok(Detection::Speech));
    }
    assert_eq!(gate.observe(score(0.0)), Ok(Detection::Quiet));
    let mut gate = confirmed();
    assert_eq!(gate.observe(score(0.0)), Ok(Detection::Speech));
    for _ in 0..6 {
        assert_eq!(gate.observe(score(0.4)), Ok(Detection::Speech));
    }
    assert_eq!(gate.observe(score(1.0)), Ok(Detection::Speech));
    for _ in 0..7 {
        assert_eq!(gate.observe(score(0.0)), Ok(Detection::Speech));
    }
    assert_eq!(gate.observe(score(0.0)), Ok(Detection::Quiet));
}
#[test]
fn composed_fake_scores_retain_internal_filtering_before_outer_turn_confirmation() {
    let mut gate = Gate::default();
    let mut boundary = Boundary::default();
    for _ in 0..29 {
        assert_eq!(
            boundary.observe(gate.observe(score(1.0)).unwrap()).unwrap(),
            BoundaryEvent::NoChange
        );
    }
    assert_eq!(
        boundary.observe(gate.observe(score(1.0)).unwrap()).unwrap(),
        BoundaryEvent::SpeechStarted
    );
    for _ in 0..39 {
        assert_eq!(
            boundary.observe(gate.observe(score(0.0)).unwrap()).unwrap(),
            BoundaryEvent::NoChange
        );
    }
    assert_eq!(
        boundary.observe(gate.observe(score(0.0)).unwrap()).unwrap(),
        BoundaryEvent::SpeechStopped
    );
}
#[test]
fn invalid_probability_and_clock_overflow_never_become_speech() {
    for value in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
        assert!(Probability::new(value).is_err());
    }
    let mut gate = Gate {
        position: u64::MAX,
        phase: Phase::Quiet,
    };
    assert_eq!(gate.observe(score(0.9)), Err(ClockExhausted));
    assert_eq!(gate.position, u64::MAX);
    assert!(!format!("{:?}", score(0.9)).contains("0.9"));
}
