//! GPUI's test platform only: no portal, Windows power request or audio device.
use super::*;
use gpui::TestAppContext;
use std::time::Duration;
use vessel_app_view::voice::{Availability, State, Target};
use vessel_proto::{NodeId, SessionId};

fn generations() -> (Generation, Generation) {
    let mut state = State::default();
    let target = Target {
        node: NodeId::random(),
        session: SessionId::random(),
    };
    let first = state.start(target, Availability::Ready).unwrap();
    state.stop().unwrap();
    state.stopped(first).unwrap();
    let second = state.start(target, Availability::Ready).unwrap();
    (first, second)
}

#[gpui::test]
fn a_held_guard_is_owned_until_local_release_and_drop_is_idempotent(cx: &mut TestAppContext) {
    let (generation, _) = generations();
    let mut power = cx.update(|cx| Power::request(generation, || {}, cx));
    cx.run_until_parked();
    assert_eq!(power.generation(), generation);
    assert_eq!(power.status(), Status::Held);
    assert_eq!(cx.active_idle_sleep_preventions(), 1);
    power.release();
    assert_eq!(power.status(), Status::Released);
    assert_eq!(cx.active_idle_sleep_preventions(), 0);
    drop(power);
    cx.run_until_parked();
    assert_eq!(cx.active_idle_sleep_preventions(), 0);
}

#[gpui::test]
fn a_cancelled_delayed_acquisition_cannot_retire_the_next_generations_guard(
    cx: &mut TestAppContext,
) {
    let (old, new) = generations();
    cx.set_idle_sleep_prevention_delay(Duration::from_secs(1));
    let mut prior = cx.update(|cx| Power::request(old, || {}, cx));
    cx.run_until_parked();
    assert_eq!(prior.status(), Status::Pending);
    assert_eq!(cx.active_idle_sleep_preventions(), 0);
    prior.release();
    cx.set_idle_sleep_prevention_delay(Duration::ZERO);
    let current = cx.update(|cx| Power::request(new, || {}, cx));
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_secs(2));
    cx.run_until_parked();
    assert_eq!(prior.status(), Status::Released);
    assert_eq!(current.status(), Status::Held);
    assert_eq!(current.generation(), new);
    assert_eq!(cx.active_idle_sleep_preventions(), 1);
    drop(prior);
    assert_eq!(cx.active_idle_sleep_preventions(), 1);
    drop(current);
    cx.run_until_parked();
    assert_eq!(cx.active_idle_sleep_preventions(), 0);
}

#[gpui::test]
fn an_acquisition_failure_is_visible_without_logging_foreign_diagnostics(cx: &mut TestAppContext) {
    let (generation, _) = generations();
    cx.set_idle_sleep_prevention_fails(true);
    let power = cx.update(|cx| Power::request(generation, || {}, cx));
    cx.run_until_parked();
    assert_eq!(power.status(), Status::Unavailable);
    assert_eq!(cx.active_idle_sleep_preventions(), 0);
    drop(power);
    cx.run_until_parked();
    assert_eq!(cx.active_idle_sleep_preventions(), 0);
}
