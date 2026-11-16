//! Observable attention controls activity on the actual writer connection.

use super::{Residency, mark_in_residency};
use crate::{log, mutation_time, schema};
use chrono::Utc;
use rusqlite::{Connection, params};
use vessel_proto::{
    EventSeq, HarnessKind, Mark, SessionId, SessionInfo, SessionMode, SessionSpec, TerminalSize,
    inbox::{InboxKey, InboxRead, MarkInbox},
};

fn database() -> (Connection, SessionId) {
    let mut db = Connection::open_in_memory().unwrap();
    schema::configure(&db).unwrap();
    schema::migrate(&mut db).unwrap();
    let info = SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            },
        ),
        Utc::now(),
    );
    log::create_session(&db, &info).unwrap();
    db.execute(
        "UPDATE sessions SET attention_seq=1, next_seq=3 WHERE id=?1",
        [info.id.to_string()],
    )
    .unwrap();
    (db, info.id)
}

fn clock(db: &Connection, id: SessionId) -> Option<vessel_proto::SessionUpdatedAt> {
    mutation_time::read(db, id).unwrap()
}

fn entry(db: &Connection, id: SessionId) -> InboxKey {
    let key = InboxKey {
        session: id,
        seq: EventSeq::new(1),
    };
    db.execute(
        "INSERT INTO inbox(session,seq,at,item,waiting,seen) VALUES(?1,1,0,?2,0,0)",
        params![id.to_string(), r#"{"kind":"finished"}"#],
    )
    .unwrap();
    key
}

#[test]
fn loaded_attention_preserves_time_and_stored_changes_tick_in_the_same_savepoint() {
    let (mut db, id) = database();
    let tx = db.transaction().unwrap();
    let at = Utc::now();
    let first = clock(&tx, id);
    let clear = mark_in_residency(
        &tx,
        id,
        Mark::Read {
            through: EventSeq::new(2),
        },
        at,
        Residency::Loaded,
    )
    .unwrap();
    assert!(!clear.marks(Some(EventSeq::new(1))).unread);
    assert_eq!(clock(&tx, id), first);
    let unread = mark_in_residency(&tx, id, Mark::Unread, at, Residency::Loaded).unwrap();
    assert!(unread.marks(Some(EventSeq::new(1))).unread);
    assert_eq!(clock(&tx, id), first);
    let clear = super::mark(
        &tx,
        id,
        Mark::Read {
            through: EventSeq::new(2),
        },
        at,
    )
    .unwrap();
    assert!(!clear.marks(Some(EventSeq::new(1))).unread);
    let cleared = clock(&tx, id);
    assert!(cleared > first);
    let _repeat = super::mark(
        &tx,
        id,
        Mark::Read {
            through: EventSeq::new(2),
        },
        at,
    )
    .unwrap();
    let farther = super::mark(
        &tx,
        id,
        Mark::Read {
            through: EventSeq::new(3),
        },
        at,
    )
    .unwrap();
    assert_eq!(farther.read, Some(EventSeq::new(3)));
    assert_eq!(clock(&tx, id), cleared);
    let _unread = super::mark(&tx, id, Mark::Unread, at).unwrap();
    let marked = clock(&tx, id);
    assert!(marked > cleared);
    let _repeat = super::mark(&tx, id, Mark::Unread, at).unwrap();
    assert_eq!(clock(&tx, id), marked);
    tx.commit().unwrap();
    assert_eq!(clock(&db, id), marked);
}

#[test]
fn pin_and_archive_keep_their_ticks_in_both_residency_paths() {
    let (db, id) = database();
    for residency in [Residency::Loaded, Residency::Stored] {
        for mark in [Mark::Pin, Mark::Unpin, Mark::Archive, Mark::Unarchive] {
            let before = clock(&db, id);
            let _kept = mark_in_residency(&db, id, mark, Utc::now(), residency).unwrap();
            assert!(clock(&db, id) > before);
        }
    }
}

#[test]
fn exact_inbox_seen_changes_tick_only_in_stored_residency() {
    let (db, id) = database();
    let key = entry(&db, id);
    let first = clock(&db, id);
    for read in [InboxRead::Read, InboxRead::Unread] {
        crate::inbox::mark_loaded(&db, &MarkInbox { key, read }).unwrap();
        assert_eq!(clock(&db, id), first);
    }
    crate::inbox::mark(
        &db,
        &MarkInbox {
            key,
            read: InboxRead::Unread,
        },
    )
    .unwrap();
    assert_eq!(clock(&db, id), first, "same seen value is not activity");
    crate::inbox::mark(
        &db,
        &MarkInbox {
            key,
            read: InboxRead::Read,
        },
    )
    .unwrap();
    let read = clock(&db, id);
    assert!(read > first);
    crate::inbox::mark(
        &db,
        &MarkInbox {
            key,
            read: InboxRead::Read,
        },
    )
    .unwrap();
    assert_eq!(clock(&db, id), read);
    crate::inbox::mark(
        &db,
        &MarkInbox {
            key,
            read: InboxRead::Unread,
        },
    )
    .unwrap();
    assert!(clock(&db, id) > read);
}

#[test]
fn a_watermark_only_read_propagates_seen_and_legacy_unknown_stays_unknown() {
    let (db, id) = database();
    let key = entry(&db, id);
    db.execute(
        "UPDATE sessions SET updated_at=NULL, read_seq=2 WHERE id=?1",
        [id.to_string()],
    )
    .unwrap();
    let kept = super::mark(
        &db,
        id,
        Mark::Read {
            through: EventSeq::new(3),
        },
        Utc::now(),
    )
    .unwrap();
    assert_eq!(kept.read, Some(EventSeq::new(3)));
    assert_eq!(clock(&db, id), None);
    assert_eq!(
        crate::inbox::get(&db, key).unwrap().unwrap().read,
        InboxRead::Read
    );
    let _loaded = super::mark_loaded(&db, id, Mark::Unread, Utc::now()).unwrap();
    assert_eq!(clock(&db, id), None);
    let _stored = super::mark(
        &db,
        id,
        Mark::Read {
            through: EventSeq::new(3),
        },
        Utc::now(),
    )
    .unwrap();
    assert!(clock(&db, id).is_some());
}
