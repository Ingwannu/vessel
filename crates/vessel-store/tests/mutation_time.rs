//! Canonical writer commits keep one clock, including rollback and direct metadata.

use chrono::{DateTime, TimeDelta, Utc};
use fixtures::{at, data_dir, new_session, open, output, status};
use rusqlite::{Connection, params};
use vessel_proto::{
    Creator, EventSeq, InvalidSessionUpdatedAt, Mark, NodeId, SessionId, SessionStatus,
    SessionUpdatedAt,
    inbox::{InboxKey, InboxRead, MarkInbox},
    subagents::DetachDisposition,
};
use vessel_store::{DataDir, StoreConfig, StoreError};

#[cfg(test)]
#[path = "support/fixtures.rs"]
mod fixtures;

fn clock(dir: &DataDir, id: SessionId) -> Option<SessionUpdatedAt> {
    let connection = Connection::open(dir.database_path()).unwrap();
    let raw: Option<i64> = connection
        .query_row(
            "SELECT updated_at FROM sessions WHERE id=?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    raw.map(|nanos| SessionUpdatedAt::try_from(DateTime::from_timestamp_nanos(nanos)).unwrap())
}

// Test-only corruption/future-time injection, after an acknowledged write.
// No production setter accepts a caller's timestamp.
fn inject(dir: &DataDir, id: SessionId, time: DateTime<Utc>) {
    let connection = Connection::open(dir.database_path()).unwrap();
    let _rows = connection
        .execute(
            "UPDATE sessions SET updated_at=?2 WHERE id=?1",
            params![id.to_string(), time.timestamp_nanos_opt().unwrap()],
        )
        .unwrap();
}

fn next(previous: SessionUpdatedAt) -> SessionUpdatedAt {
    SessionUpdatedAt::try_from(
        previous
            .time()
            .checked_add_signed(SessionUpdatedAt::MIN_INCREMENT)
            .unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn one_nonempty_batch_and_each_mark_tick_once_without_using_record_time() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = new_session(0);
    store.create_session(session.clone()).await.unwrap();
    let first = clock(&dir, session.id).unwrap();
    assert_ne!(first.time(), session.created_at);
    let future = SessionUpdatedAt::advance(
        None,
        Utc::now().checked_add_signed(TimeDelta::days(1)).unwrap(),
    )
    .unwrap();
    inject(&dir, session.id, future.time());
    store
        .append(session.id, vec![output(0, 5), output(1, 7)])
        .await
        .unwrap();
    let appended = clock(&dir, session.id).unwrap();
    assert_eq!(appended, next(future), "one batch, not one tick per record");
    store.append(session.id, Vec::new()).await.unwrap();
    assert_eq!(clock(&dir, session.id), Some(appended));
    let _marks = store.mark(session.id, Mark::Pin, at(-100)).await.unwrap();
    let marked = clock(&dir, session.id).unwrap();
    assert_eq!(
        marked,
        next(appended),
        "a mark's supplied time is not ordering authority"
    );
    assert!(matches!(
        store.append(session.id, vec![output(4, 1)]).await,
        Err(StoreError::OutOfOrder { .. })
    ));
    assert_eq!(clock(&dir, session.id), Some(marked));
    let _read = store.session(session.id).await.unwrap();
    let _page = store
        .read_events(session.id, EventSeq::ZERO, 10)
        .await
        .unwrap();
    assert_eq!(clock(&dir, session.id), Some(marked));
    store.close().await;
}

#[tokio::test]
async fn inbox_reopen_and_first_detach_share_the_same_clock_but_repeat_receipts_do_not() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = new_session(0).with_creator(Creator::Agent {
        node: NodeId::random(),
        session: SessionId::random(),
    });
    store.create_session(session.clone()).await.unwrap();
    let first = clock(&dir, session.id).unwrap();
    store
        .append(
            session.id,
            vec![status(0, SessionStatus::Exited { code: Some(0) })],
        )
        .await
        .unwrap();
    let ended = clock(&dir, session.id).unwrap();
    assert!(ended > first);
    store
        .mark_inbox(MarkInbox {
            key: InboxKey {
                session: session.id,
                seq: EventSeq::ZERO,
            },
            read: InboxRead::Read,
        })
        .await
        .unwrap();
    let seen = clock(&dir, session.id).unwrap();
    assert!(seen > ended);
    let _marks = store.reopen(session.id, EventSeq::new(1)).await.unwrap();
    let reopened = clock(&dir, session.id).unwrap();
    assert!(reopened > seen);
    let detached = store.detach(session.id, at(-200)).await.unwrap();
    assert_eq!(detached.disposition, DetachDisposition::Detached);
    let changed = clock(&dir, session.id).unwrap();
    assert!(changed > reopened);
    let repeated = store.detach(session.id, at(-100)).await.unwrap();
    assert_eq!(repeated.disposition, DetachDisposition::AlreadyDetached);
    assert_eq!(repeated.receipt, detached.receipt);
    assert_eq!(clock(&dir, session.id), Some(changed));
    store.close().await;
}

#[tokio::test]
async fn exhausted_clock_rolls_back_both_the_append_and_mark() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = new_session(0);
    store.create_session(session.clone()).await.unwrap();
    let bound = DateTime::<Utc>::from_timestamp_nanos(i64::MAX);
    let last = DateTime::from_timestamp_millis(bound.timestamp_millis()).unwrap();
    inject(&dir, session.id, last);
    for refused in [
        store.append(session.id, vec![output(0, 5)]).await,
        store.mark(session.id, Mark::Pin, at(1)).await.map(drop),
    ] {
        assert!(matches!(refused, Err(StoreError::MutationTime {
            session: id, source: InvalidSessionUpdatedAt::Exhausted,
        }) if id == session.id));
    }
    let kept = store.session(session.id).await.unwrap().unwrap();
    assert_eq!(kept.next_seq, EventSeq::ZERO);
    assert!(!kept.marks.pinned);
    assert!(
        store
            .read_events(session.id, EventSeq::ZERO, 10)
            .await
            .unwrap()
            .records
            .is_empty()
    );
    assert_eq!(clock(&dir, session.id).unwrap().time(), last);
    store.close().await;
}

#[tokio::test]
async fn refused_creation_reopen_detach_and_missing_inbox_entry_do_not_tick() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = new_session(0);
    store.create_session(session.clone()).await.unwrap();
    let before = clock(&dir, session.id);
    assert!(matches!(
        store.create_session(session.clone()).await,
        Err(StoreError::SessionExists(_))
    ));
    assert!(matches!(
        store.reopen(session.id, EventSeq::ZERO).await,
        Err(StoreError::SessionLive(_))
    ));
    assert!(matches!(
        store.detach(session.id, at(0)).await,
        Err(StoreError::NotChild(_))
    ));
    assert!(matches!(
        store
            .mark_inbox(MarkInbox {
                key: InboxKey {
                    session: session.id,
                    seq: EventSeq::ZERO
                },
                read: InboxRead::Read,
            })
            .await,
        Err(StoreError::InboxGone)
    ));
    assert_eq!(clock(&dir, session.id), before);
    store.close().await;
}
