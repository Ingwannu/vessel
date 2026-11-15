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
            point: None,
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
    migrate_again_without_inbox(&mut db);
    let page = super::super::list(&db, ListInbox::default()).unwrap();
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].item, InboxItem::Limit);
    assert_eq!(page.entries[0].read, InboxRead::Read);
    schema::migrate(&mut db).unwrap();
    assert_eq!(super::super::list(&db, ListInbox::default()).unwrap(), page);
}

#[test]
fn rewind_withdraws_old_completion_and_tools_without_deleting_the_log() {
    let db = database();
    let id = session(&db);
    append(&db, id, 0, AgentEvent::ToolCall(tool()));
    append(
        &db,
        id,
        1,
        AgentEvent::TurnEnded {
            reason: StopReason::EndTurn,
            usage: None,
            point: None,
        },
    );
    let old = super::super::list(&db, ListInbox::default())
        .unwrap()
        .entries
        .remove(0);
    assert_eq!(old.item, InboxItem::Finished);
    let record = EventRecord {
        seq: EventSeq::new(2),
        at: now(),
        event: SessionEvent::Rewound {
            after: EventSeq::new(1),
            harness_session: vessel_proto::HarnessSessionId::new("rewound-conversation"),
            files_changed: 0,
        },
    };
    let _appended = log::append(&db, id, [&record]).unwrap();
    assert!(
        super::super::list(&db, ListInbox::default())
            .unwrap()
            .entries
            .is_empty()
    );
    assert!(super::super::get(&db, old.key).unwrap().is_none());
    let tools: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM inbox_tools WHERE session=?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tools, 0);
    let records: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM events WHERE session_id=?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(records, 3);
}

// The fixture removes later columns explicitly; the schema owner locates
// the inbox predecessor instead of guessing how many appends followed it.
fn migrate_again_without_inbox(db: &mut Connection) {
    db.execute_batch("DROP TABLE notification_claims;").unwrap();
    db.execute_batch("ALTER TABLE sessions DROP COLUMN listing;")
        .unwrap();
    db.execute_batch("DROP TABLE workspace_registry_resources; DROP TABLE workspace_registry_reservations; DROP TABLE workspace_registry; DROP TABLE workspace_registry_meta;").unwrap();
    db.execute_batch(
        "ALTER TABLE sessions DROP COLUMN detach_receipt;
     ALTER TABLE sessions DROP COLUMN title_origin;",
    )
    .unwrap();
    db.execute_batch("ALTER TABLE sessions DROP COLUMN labels; DROP TABLE inbox_tools; DROP TABLE inbox; DROP TABLE inbox_meta; DROP TABLE push_subscriptions;")
        .unwrap();
    schema::replay_inbox_after_rollback(db);
}

#[test]
fn backfill_never_revives_attention_before_a_new_turn_or_rewind() {
    for moved in [
        SessionEvent::Agent {
            event: AgentEvent::TurnStarted {
                content: Vec::new(),
            },
        },
        SessionEvent::Rewound {
            after: EventSeq::ZERO,
            harness_session: vessel_proto::HarnessSessionId::new("rewound-conversation"),
            files_changed: 0,
        },
    ] {
        let mut db = database();
        let id = session(&db);
        append(
            &db,
            id,
            0,
            AgentEvent::TurnEnded {
                reason: StopReason::MaxTokens,
                usage: None,
                point: None,
            },
        );
        let moved = EventRecord {
            seq: EventSeq::new(1),
            at: now(),
            event: moved,
        };
        let stopped = EventRecord {
            seq: EventSeq::new(2),
            at: now(),
            event: SessionEvent::Status {
                status: SessionStatus::Interrupted,
            },
        };
        let _appended = log::append(&db, id, [&moved, &stopped]).unwrap();
        assert!(
            super::super::list(&db, ListInbox::default())
                .unwrap()
                .entries
                .is_empty()
        );
        migrate_again_without_inbox(&mut db);
        assert!(
            super::super::list(&db, ListInbox::default())
                .unwrap()
                .entries
                .is_empty()
        );
    }
}
