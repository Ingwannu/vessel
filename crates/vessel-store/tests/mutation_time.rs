//! Canonical writer commits keep one clock, including rollback and direct metadata.

use chrono::{DateTime, TimeDelta, Utc};
use clock_fixture::{clock, inject, next};
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
#[expect(
    dead_code,
    reason = "shared store fixtures include rollback and paging helpers"
)]
mod fixtures;

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
    let marked_entry = store
        .mark_inbox_committed(MarkInbox {
            key: InboxKey {
                session: session.id,
                seq: EventSeq::ZERO,
            },
            read: InboxRead::Read,
        })
        .await
        .unwrap();
    assert_eq!(marked_entry.updated_at(), clock(&dir, session.id));
    let seen = clock(&dir, session.id).unwrap();
    assert!(seen > ended);
    let _marks = store.reopen(session.id, EventSeq::new(1)).await.unwrap();
    let reopened = clock(&dir, session.id).unwrap();
    assert!(reopened > seen);
    let detached = store.detach(session.id, at(-200)).await.unwrap();
    assert_eq!(detached.disposition, DetachDisposition::Detached);
    assert_eq!(detached.session.updated_at, clock(&dir, session.id));
    let changed = clock(&dir, session.id).unwrap();
    assert!(changed > reopened);
    let repeated = store.detach(session.id, at(-100)).await.unwrap();
    assert_eq!(repeated.disposition, DetachDisposition::AlreadyDetached);
    assert_eq!(repeated.receipt, detached.receipt);
    assert_eq!(repeated.session.updated_at, clock(&dir, session.id));
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

#[tokio::test]
async fn each_ack_keeps_its_own_marks_and_clock_before_later_queued_writes() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = new_session(0);
    let created = store
        .create_session_committed(session.clone())
        .await
        .unwrap();
    assert_eq!(created.updated_at(), clock(&dir, session.id));
    let (pinned, unpinned) = tokio::join!(
        store.mark_committed(session.id, Mark::Pin, at(0)),
        store.mark_committed(session.id, Mark::Unpin, at(0)),
    );
    let (pin_marks, pin_time) = pinned.unwrap().into_parts();
    let (unpin_marks, unpin_time) = unpinned.unwrap().into_parts();
    assert!(pin_marks.pinned);
    assert!(!unpin_marks.pinned);
    let pin_time = pin_time.unwrap();
    let unpin_time = unpin_time.unwrap();
    assert_ne!(pin_time, unpin_time);
    let latest = pin_time.max(unpin_time);
    assert_eq!(clock(&dir, session.id), Some(latest));
    let current = store.session(session.id).await.unwrap().unwrap();
    assert_eq!(current.marks.pinned, pin_time > unpin_time);
    store.close().await;
}

#[tokio::test]
async fn an_empty_ack_keeps_legacy_unknown_until_an_actual_mutation() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = new_session(0);
    store.create_session(session.clone()).await.unwrap();
    // Simulate a legacy row only after creation is acknowledged.
    let connection = Connection::open(dir.database_path()).unwrap();
    let _rows = connection
        .execute(
            "UPDATE sessions SET updated_at=NULL WHERE id=?1",
            [session.id.to_string()],
        )
        .unwrap();
    let empty = store
        .append_committed(session.id, Vec::new())
        .await
        .unwrap();
    assert_eq!(empty.updated_at(), None);
    let mark = store
        .mark_committed(session.id, Mark::Pin, at(0))
        .await
        .unwrap();
    assert!(mark.updated_at().is_some());
    assert_eq!(mark.updated_at(), clock(&dir, session.id));
    store.close().await;
}

#[tokio::test]
async fn supplied_creation_or_import_clock_cannot_choose_local_mutation_order() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let mut session = new_session(0);
    let supplied = SessionUpdatedAt::advance(
        None,
        Utc::now().checked_add_signed(TimeDelta::days(1)).unwrap(),
    )
    .unwrap();
    session.updated_at = Some(supplied);
    let created = store
        .create_session_committed(session.clone())
        .await
        .unwrap();
    let kept = created.updated_at().unwrap();
    assert!(kept < supplied);
    assert_ne!(kept, supplied);
    assert_eq!(
        store.session(session.id).await.unwrap().unwrap().updated_at,
        Some(kept)
    );
    store.close().await;
}

#[tokio::test]
async fn corrupt_precision_refuses_reads_and_rolls_back_a_metadata_write() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = new_session(0);
    store.create_session(session.clone()).await.unwrap();
    inject(&dir, session.id, at(0));
    assert!(matches!(store.session(session.id).await,
        Err(StoreError::MutationTime { session: id,
            source: InvalidSessionUpdatedAt::Submillisecond }) if id == session.id));
    assert!(matches!(store.mark(session.id, Mark::Pin, at(1)).await,
        Err(StoreError::MutationTime { session: id,
            source: InvalidSessionUpdatedAt::Submillisecond }) if id == session.id));
    let connection = Connection::open(dir.database_path()).unwrap();
    let (pinned, kept): (bool, i64) = connection
        .query_row(
            "SELECT pinned,updated_at FROM sessions WHERE id=?1",
            [session.id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert!(!pinned);
    assert_eq!(kept, at(0).timestamp_nanos_opt().unwrap());
    store.close().await;
}

// Keep corruption and clock-bound assertions wholly in the test configuration,
// as the shared store fixture does; none is an application timestamp setter.
#[cfg(test)]
mod clock_fixture {
    use super::*;

    /// Inspect the canonical row without adding a production timestamp setter.
    pub(super) fn clock(dir: &DataDir, id: SessionId) -> Option<SessionUpdatedAt> {
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

    /// Force corruption or clock exhaustion only after an acknowledged test write.
    /// No production setter accepts a caller's timestamp.
    pub(super) fn inject(dir: &DataDir, id: SessionId, time: DateTime<Utc>) {
        let connection = Connection::open(dir.database_path()).unwrap();
        let _rows = connection
            .execute(
                "UPDATE sessions SET updated_at=?2 WHERE id=?1",
                params![id.to_string(), time.timestamp_nanos_opt().unwrap()],
            )
            .unwrap();
    }

    /// Calculate one expected writer tick independently of the wall-clock sample.
    pub(super) fn next(previous: SessionUpdatedAt) -> SessionUpdatedAt {
        SessionUpdatedAt::try_from(
            previous
                .time()
                .checked_add_signed(SessionUpdatedAt::MIN_INCREMENT)
                .unwrap(),
        )
        .unwrap()
    }
}
