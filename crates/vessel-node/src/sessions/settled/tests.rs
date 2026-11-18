//! The barrier's bound, on a paused clock: commits are waited for together,
//! a store that never commits holds a read for `SETTLE` and no longer, and
//! a broken store is not waited for.

use std::time::Duration;

use tokio::sync::watch;
use tokio::time::Instant;
use vessel_proto::{EventSeq, SessionId};

use super::{SETTLE, settle};
use crate::persist::Durability;

fn behind() -> (watch::Sender<Durability>, watch::Receiver<Durability>) {
    watch::channel(Durability::Through(EventSeq::ZERO))
}

#[tokio::test(start_paused = true)]
async fn a_commit_within_the_bound_is_waited_for() {
    let (store, durability) = behind();
    let session = SessionId::random();
    let started = Instant::now();
    let commit = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        store.send_modify(|durability| *durability = Durability::Through(EventSeq::new(3)));
        store
    });
    let late = settle(vec![(session, EventSeq::new(3), durability)], SETTLE).await;
    assert!(late.is_empty());
    assert_eq!(started.elapsed(), Duration::from_millis(500));
    drop(commit.await.unwrap());
}

#[tokio::test(start_paused = true)]
async fn sessions_that_never_commit_hold_the_read_for_the_bound_together() {
    let (_first_store, first) = behind();
    let (_second_store, second) = behind();
    let (a, b) = (SessionId::random(), SessionId::random());
    let started = Instant::now();
    let late = settle(
        vec![(a, EventSeq::new(1), first), (b, EventSeq::new(1), second)],
        SETTLE,
    )
    .await;
    assert_eq!(late, [a, b]);
    assert_eq!(started.elapsed(), SETTLE, "one bound for all, not one each");
}

#[tokio::test(start_paused = true)]
async fn a_broken_store_is_not_waited_for() {
    let (_store, durability) = watch::channel(Durability::Broken);
    let started = Instant::now();
    let late = settle(
        vec![(SessionId::random(), EventSeq::new(9), durability)],
        SETTLE,
    )
    .await;
    assert!(late.is_empty());
    assert_eq!(started.elapsed(), Duration::ZERO);
}
