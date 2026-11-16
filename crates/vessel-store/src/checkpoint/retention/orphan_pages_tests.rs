//! Restart pagination must make progress after earlier rows settle. SQL-only:
//! no worker, private filesystem cleanup or process-death proof is modeled here.
use super::super::{journal, orphans, quota, writer_tests};
use chrono::DateTime;
use rusqlite::Connection;
use std::collections::BTreeSet;
use vessel_proto::{
    SessionId,
    checkpoint::{CHECKPOINT_PAGE_ENTRIES, CaptureId, CaptureKey},
};

fn candidate(db: &Connection, session: SessionId) -> CaptureKey {
    let plan = writer_tests::plan(session);
    // The model leaves no payload. Its reserved metadata survives invalidation;
    // this does not bypass the node's fixed real-capture escrow or worker bound.
    journal::candidate(
        db,
        plan.key,
        crate::ByteCount::new(quota::METADATA_BYTES),
        DateTime::UNIX_EPOCH,
    )
    .unwrap();
    plan.key
}

fn ids(page: &orphans::OrphanedPage) -> Vec<CaptureId> {
    page.captures
        .iter()
        .map(|capture| capture.key().capture)
        .collect()
}

#[test]
fn checkpoint_restart_keyset_scan_survives_settled_page_rows_without_losing_later_work() {
    let mut db = Connection::open_in_memory().unwrap();
    crate::schema::configure_writer(&db).unwrap();
    crate::schema::migrate(&mut db).unwrap();
    let session = writer_tests::info();
    crate::log::create_session(&db, &session).unwrap();
    let count = CHECKPOINT_PAGE_ENTRIES.checked_add(3).unwrap();
    let expected = (0..count)
        .map(|_row| candidate(&db, session.id).capture)
        .collect::<BTreeSet<_>>();
    super::startup(&mut db).unwrap();
    let current = candidate(&db, session.id);
    let first = orphans::page(&db, None).unwrap();
    assert_eq!(first.captures.len(), CHECKPOINT_PAGE_ENTRIES);
    assert_eq!(first.next, ids(&first).last().copied());
    assert!(
        first
            .captures
            .iter()
            .all(|capture| matches!(capture, orphans::OrphanedCapture::Discard { .. }))
    );
    for orphan in &first.captures {
        // Model a later actual cleanup acknowledgement, removing these rows
        // from the next scan before its saved cursor is consumed.
        quota::capture_settled(&db, orphan.key()).unwrap();
    }
    let second = orphans::page(&db, first.next).unwrap();
    assert_eq!(second.captures.len(), 3);
    assert!(second.next.is_none());
    let seen = ids(&first)
        .into_iter()
        .chain(ids(&second))
        .collect::<Vec<_>>();
    assert_eq!(seen.len(), expected.len());
    assert_eq!(seen.into_iter().collect::<BTreeSet<_>>(), expected);
    assert_eq!(orphans::page(&db, None).unwrap(), second);
    assert!(!ids(&second).contains(&current.capture));
    for orphan in second.captures {
        quota::capture_settled(&db, orphan.key()).unwrap();
    }
    let empty = orphans::page(&db, None).unwrap();
    assert!(empty.captures.is_empty());
    assert!(empty.next.is_none());
    assert_eq!(
        quota::used(&db).unwrap().get(),
        u64::try_from(count.checked_add(1).unwrap())
            .unwrap()
            .checked_mul(quota::METADATA_BYTES)
            .unwrap()
    );
}
