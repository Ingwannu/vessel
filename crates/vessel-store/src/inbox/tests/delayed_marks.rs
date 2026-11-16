//! Marks can commit before records published by the runtime are persisted.
use super::*;
use vessel_proto::Mark;

#[test]
fn archive_then_delayed_requests_and_final_status_cannot_recreate_attention() {
    let mut db = database();
    let id = session(&db);
    append(&db, id, 0, AgentEvent::PermissionRequested(permission()));
    let old = super::super::list(&db, ListInbox::default())
        .unwrap()
        .entries
        .remove(0);
    let transaction = db.transaction().unwrap();
    let _marks = crate::marks::mark(
        &transaction,
        id,
        Mark::Archive,
        now(),
        crate::Residence::Loaded,
    )
    .unwrap();
    transaction.commit().unwrap();
    for (seq, event) in [
        (
            1,
            SessionEvent::Agent {
                event: AgentEvent::PermissionRequested(permission()),
            },
        ),
        (
            2,
            SessionEvent::Agent {
                event: AgentEvent::TurnEnded {
                    reason: StopReason::EndTurn,
                    usage: None,
                    point: None,
                },
            },
        ),
        (
            3,
            SessionEvent::Status {
                status: SessionStatus::Exited { code: Some(0) },
            },
        ),
    ] {
        let transaction = db.transaction().unwrap();
        let _appended = log::append(
            &transaction,
            id,
            [&EventRecord {
                seq: EventSeq::new(seq),
                at: now(),
                event,
            }],
        )
        .unwrap();
        transaction.commit().unwrap();
        let page = super::super::list(&db, ListInbox::default()).unwrap();
        assert!(page.entries.is_empty());
        assert_eq!(page.counts.total.get(), 0);
        assert_eq!(page.counts.unread.get(), 0);
        assert!(super::super::get(&db, old.key).unwrap().is_none());
    }
}

#[test]
fn read_then_delayed_append_inherits_the_kept_watermark_but_newer_source_is_unread() {
    let mut db = database();
    let id = session(&db);
    // The runtime already published seq 0..2, while durable next_seq is 0.
    let transaction = db.transaction().unwrap();
    let _marks = crate::marks::mark(
        &transaction,
        id,
        Mark::Read {
            through: EventSeq::new(2),
        },
        now(),
        crate::Residence::Loaded,
    )
    .unwrap();
    transaction.commit().unwrap();
    for seq in 0..3 {
        let transaction = db.transaction().unwrap();
        let _appended = log::append(
            &transaction,
            id,
            [&EventRecord {
                seq: EventSeq::new(seq),
                at: now(),
                event: SessionEvent::Agent {
                    event: AgentEvent::PermissionRequested(permission()),
                },
            }],
        )
        .unwrap();
        transaction.commit().unwrap();
    }
    let page = super::super::list(&db, ListInbox::default()).unwrap();
    assert_eq!(page.counts.total.get(), 3);
    assert_eq!(page.counts.waiting.get(), 3);
    assert_eq!(page.counts.unread.get(), 1);
    for entry in page.entries {
        assert_eq!(
            entry.read,
            if entry.key.seq < EventSeq::new(2) {
                InboxRead::Read
            } else {
                InboxRead::Unread
            }
        );
    }
}

#[test]
fn explicit_unread_overrides_the_watermark_for_delayed_inserts_until_the_next_read() {
    let db = database();
    let id = session(&db);
    let _marks = crate::marks::mark(
        &db,
        id,
        Mark::Read {
            through: EventSeq::new(3),
        },
        now(),
        crate::Residence::Loaded,
    )
    .unwrap();
    let _marks =
        crate::marks::mark(&db, id, Mark::Unread, now(), crate::Residence::Loaded).unwrap();
    append(&db, id, 0, AgentEvent::PermissionRequested(permission()));
    append(&db, id, 1, AgentEvent::PermissionRequested(permission()));
    assert_eq!(
        super::super::list(&db, ListInbox::default())
            .unwrap()
            .counts
            .unread
            .get(),
        2
    );
    // An older read clears explicit unread but must retain the farther read.
    let _marks = crate::marks::mark(
        &db,
        id,
        Mark::Read {
            through: EventSeq::new(1),
        },
        now(),
        crate::Residence::Loaded,
    )
    .unwrap();
    append(&db, id, 2, AgentEvent::PermissionRequested(permission()));
    assert_eq!(
        super::super::list(&db, ListInbox::default())
            .unwrap()
            .counts
            .unread
            .get(),
        0
    );
    append(&db, id, 3, AgentEvent::PermissionRequested(permission()));
    assert_eq!(
        super::super::list(&db, ListInbox::default())
            .unwrap()
            .counts
            .unread
            .get(),
        1
    );
}
