//! Synthetic-time transition tests; no connector, socket or process is started.

use std::future::{Future as _, pending, ready};
use std::pin::pin;
use std::task::{Context, Waker};
use std::time::Duration;

use super::{FAILURES_BEFORE_UNHEALTHY, GRACE, Health, HealthClock, Instant, Probe};

#[tokio::test]
async fn grace_is_observation_based_and_does_not_invoke_the_probe() {
    let earlier = Instant::now();
    let observed = earlier + GRACE;
    let mut clock = HealthClock::new(observed);
    assert_eq!(clock.state(), Health::Pending);
    for now in [
        earlier,
        observed,
        observed + GRACE - Duration::from_nanos(1),
    ] {
        let change = clock
            .poll(now, || -> std::future::Ready<Probe> {
                panic!("a probe started before the observation grace")
            })
            .await;
        assert_eq!(change, None);
    }
    let at_boundary = observed + GRACE;
    assert_eq!(
        clock.poll(at_boundary, || ready(Probe::Connected)).await,
        Some(Health::Healthy)
    );
}

#[tokio::test]
async fn failures_require_the_threshold_and_unchanged_health_is_not_emitted() {
    let observed = Instant::now();
    let now = observed + GRACE;
    let mut clock = HealthClock::new(observed);
    for count in 1..=FAILURES_BEFORE_UNHEALTHY {
        let expected = (count == FAILURES_BEFORE_UNHEALTHY).then_some(Health::Unhealthy);
        assert_eq!(clock.poll(now, || ready(Probe::Failed)).await, expected);
    }
    // Exceed the counter's storage range: an outage stays saturated.
    for _ in 0..=u8::MAX {
        assert_eq!(clock.poll(now, || ready(Probe::Failed)).await, None);
    }
    assert_eq!(clock.state(), Health::Unhealthy);
}

#[tokio::test]
async fn success_recovers_and_resets_consecutive_failures() {
    let observed = Instant::now();
    let now = observed + GRACE;
    let mut clock = HealthClock::new(observed);
    assert_eq!(
        clock.poll(now, || ready(Probe::Connected)).await,
        Some(Health::Healthy)
    );
    for _ in 1..FAILURES_BEFORE_UNHEALTHY {
        assert_eq!(clock.poll(now, || ready(Probe::Failed)).await, None);
    }
    assert_eq!(clock.poll(now, || ready(Probe::Connected)).await, None);
    for count in 1..=FAILURES_BEFORE_UNHEALTHY {
        let expected = (count == FAILURES_BEFORE_UNHEALTHY).then_some(Health::Unhealthy);
        assert_eq!(clock.poll(now, || ready(Probe::Failed)).await, expected);
    }
    assert_eq!(
        clock.poll(now, || ready(Probe::Connected)).await,
        Some(Health::Healthy)
    );
    assert_eq!(clock.poll(now, || ready(Probe::Connected)).await, None);
}

#[tokio::test]
async fn a_cancelled_probe_commits_no_failure_or_health_change() {
    let observed = Instant::now();
    let now = observed + GRACE;
    let mut clock = HealthClock::new(observed);
    let mut entered = false;
    {
        let mut poll = pin!(clock.poll(now, || {
            entered = true;
            pending::<Probe>()
        }));
        let mut context = Context::from_waker(Waker::noop());
        assert!(poll.as_mut().poll(&mut context).is_pending());
    }
    assert!(entered);
    assert_eq!(clock.state(), Health::Pending);
    for count in 1..=FAILURES_BEFORE_UNHEALTHY {
        let expected = (count == FAILURES_BEFORE_UNHEALTHY).then_some(Health::Unhealthy);
        assert_eq!(clock.poll(now, || ready(Probe::Failed)).await, expected);
    }
}

#[tokio::test]
async fn a_replacement_cannot_inherit_health_or_an_elapsed_grace() {
    let observed = Instant::now();
    let replaced_at = observed + GRACE;
    let mut previous = HealthClock::new(observed);
    assert_eq!(
        previous.poll(replaced_at, || ready(Probe::Connected)).await,
        Some(Health::Healthy)
    );
    let mut replacement = HealthClock::new(replaced_at);
    assert_eq!(replacement.state(), Health::Pending);
    assert_eq!(
        replacement
            .poll(replaced_at, || -> std::future::Ready<Probe> {
                panic!("replacement grace was inherited")
            })
            .await,
        None
    );
    assert_eq!(previous.state(), Health::Healthy);
}
