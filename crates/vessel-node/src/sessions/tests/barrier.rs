//! The usage barrier (`settled`) with a store that is slow when a turn ends,
//! as under gate 78b's load (`usage.spec.ts:127`): a machine read waits for
//! the turn the live log already shows, and a store slower than the bound
//! holds the read no longer than `SETTLE`.

use std::time::Duration;

use chrono::Utc;
use tokio::sync::mpsc;
use tokio::time::Instant;
use vessel_proto::usage::{QueryUsage, UsagePeriod, UsageScope};
use vessel_proto::{AgentEvent, ContentBlock, EventSeq, SessionEvent, SessionInput};
use vessel_session::{Created, DEFAULT_DURABLE_CAPACITY, LaunchOptions, SessionHandle};

use super::Fixture;
use crate::sessions::settled::SETTLE;

fn turn_end(event: &SessionEvent) -> bool {
    matches!(
        event,
        SessionEvent::Agent {
            event: AgentEvent::TurnEnded { .. }
        }
    )
}

/// A structured session whose `turn_ended` reaches the store `delay` after
/// the live log has it, as a loaded persist task would deliver it.
async fn slow_at_turn_end(fixture: &Fixture, delay: Duration) -> SessionHandle {
    let Created {
        initial,
        handle,
        durable,
    } = fixture
        .runtime
        .create_with(fixture.structured(), LaunchOptions::default())
        .await
        .unwrap();
    let durability = fixture.sessions.track(&handle, EventSeq::ZERO);
    let _committed = fixture
        .store
        .create_session_committed(*initial)
        .await
        .unwrap();
    let (sender, delayed) = mpsc::channel(DEFAULT_DURABLE_CAPACITY.get());
    drop(tokio::spawn(async move {
        let mut durable = durable;
        while let Some(record) = durable.recv().await {
            if turn_end(&record.event) {
                tokio::time::sleep(delay).await;
            }
            if sender.send(record).await.is_err() {
                break;
            }
        }
    }));
    fixture
        .sessions
        .persist(handle.clone(), delayed, durability);
    handle
}

/// Prompt `usage` (20 tokens in, 10 out) and wait until its turn's end is
/// in the session's live log.
async fn turn_with_usage(handle: &SessionHandle) {
    let follow = handle.follow(EventSeq::ZERO).unwrap();
    let mut live = follow.live;
    let mut ended = follow.cached.iter().any(|record| turn_end(&record.event));
    handle
        .send(SessionInput::Prompt {
            content: vec![ContentBlock::Text {
                text: "usage".to_owned(),
            }],
        })
        .await
        .unwrap();
    while !ended {
        ended = turn_end(&live.recv().await.unwrap().event);
    }
}

fn machine_today() -> QueryUsage {
    QueryUsage {
        scope: UsageScope::Machine,
        period: UsagePeriod::Day,
        after: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_machine_read_waits_for_the_turn_the_live_log_shows() {
    let fixture = Fixture::new().await;
    let handle = slow_at_turn_end(&fixture, Duration::from_millis(1500)).await;
    turn_with_usage(&handle).await;
    // Without the barrier the store would not have the turn yet.
    let before = fixture
        .store
        .usage(machine_today(), Utc::now())
        .await
        .unwrap();
    assert_eq!(before.totals.tokens.map(|tokens| tokens.input.get()), None);
    let late = fixture.sessions.settled().await;
    assert!(late.is_empty(), "{late:?}");
    let report = fixture
        .store
        .usage(machine_today(), Utc::now())
        .await
        .unwrap();
    assert_eq!(
        report.totals.tokens.map(|tokens| tokens.input.get()),
        Some(20)
    );
    handle.kill();
    fixture.settle().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_store_slower_than_the_bound_holds_the_read_no_longer() {
    let fixture = Fixture::new().await;
    let handle = slow_at_turn_end(&fixture, SETTLE.saturating_mul(3)).await;
    turn_with_usage(&handle).await;
    let started = Instant::now();
    let late = fixture.sessions.settled().await;
    let waited = started.elapsed();
    assert_eq!(late, [handle.id()]);
    assert!(waited >= SETTLE, "{waited:?}");
    assert!(
        waited < SETTLE.saturating_add(Duration::from_secs(1)),
        "{waited:?}"
    );
    handle.kill();
    fixture.settle().await;
}
