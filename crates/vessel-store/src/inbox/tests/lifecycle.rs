//! Crash, deletion and one-time migration lifecycles share the projection fixtures.
use super::*;
#[test]
fn recovery_and_session_delete_remove_pending_rows_transactionally() {
    let mut db = database();
    let id = session(&db);
    append(&db, id, 0, AgentEvent::PermissionRequested(permission()));
    assert_eq!(
        super::super::list(&db, ListInbox::default())
            .unwrap()
            .entries
            .len(),
        1
    );
    let recovered = crate::recovery::interrupt_live(&mut db, now()).unwrap();
    assert_eq!(recovered.len(), 1);
    assert!(
        super::super::list(&db, ListInbox::default())
            .unwrap()
            .entries
            .is_empty()
    );
    crate::marks::delete(&db, id).unwrap();
    assert!(
        super::super::list(&db, ListInbox::default())
            .unwrap()
            .entries
            .is_empty()
    );
}

#[test]
fn an_append_rollback_cannot_leave_a_ghost_request() {
    let mut db = database();
    let id = session(&db);
    let transaction = db.transaction().unwrap();
    let records = [
        Arc::new(EventRecord {
            seq: EventSeq::ZERO,
            at: now(),
            event: SessionEvent::Agent {
                event: AgentEvent::PermissionRequested(permission()),
            },
        }),
        Arc::new(EventRecord {
            seq: EventSeq::new(2),
            at: now(),
            event: SessionEvent::Agent {
                event: AgentEvent::TurnFailed {
                    message: String::new(),
                },
            },
        }),
    ];
    assert!(log::append(&transaction, id, records.iter().map(Arc::as_ref)).is_err());
    transaction.rollback().unwrap();
    assert!(
        super::super::list(&db, ListInbox::default())
            .unwrap()
            .entries
            .is_empty()
    );
    assert!(matches!(
        super::super::mark(
            &db,
            &MarkInbox {
                key: InboxKey {
                    session: id,
                    seq: EventSeq::ZERO
                },
                read: InboxRead::Read
            }
        ),
        Err(StoreError::InboxGone)
    ));
}

#[test]
fn migration_backfills_retained_outcomes_once_and_preserves_existing_read_attention() {
    let mut db = database();
    let id = session(&db);
    append(
        &db,
        id,
        0,
        AgentEvent::TurnEnded {
            reason: StopReason::MaxTokens,
            usage: None,
        },
    );
    let status = EventRecord {
        seq: EventSeq::new(1),
        at: now(),
        event: SessionEvent::Status {
            status: SessionStatus::Exited { code: Some(0) },
        },
    };
    let _appended = log::append(&db, id, [&status]).unwrap();
    let _marks = crate::marks::mark(
        &db,
        id,
        vessel_proto::Mark::Read {
            through: EventSeq::new(2),
        },
        now(),
    )
    .unwrap();
    // Simulate the preceding schema, independent of the migration's number
    // when parallel feature migrations are joined.
    let version: i64 = db
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    db.execute_batch("DROP TABLE inbox_tools; DROP TABLE inbox; DROP TABLE inbox_meta; DROP TABLE push_subscriptions;")
        .unwrap();
    db.pragma_update(None, "user_version", version.saturating_sub(1))
        .unwrap();
    schema::migrate(&mut db).unwrap();
    let page = super::super::list(&db, ListInbox::default()).unwrap();
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].item, InboxItem::Limit);
    assert_eq!(page.entries[0].read, InboxRead::Read);
    schema::migrate(&mut db).unwrap();
    assert_eq!(super::super::list(&db, ListInbox::default()).unwrap(), page);
}
