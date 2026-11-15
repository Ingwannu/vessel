//! Rollups are independent of pagination, exact-entry reads and log marks.
use super::*;

#[test]
fn badges_include_off_page_requests_and_exact_reads_do_not_consume_another() {
    let db = database();
    let target = session(&db);
    let empty = session(&db);
    let first = permission();
    let second = permission();
    append(
        &db,
        target,
        0,
        AgentEvent::PermissionRequested(first.clone()),
    );
    append(
        &db,
        target,
        1,
        AgentEvent::PermissionRequested(second.clone()),
    );
    for _entry in 0..20 {
        let other = session(&db);
        append(
            &db,
            other,
            0,
            AgentEvent::TurnEnded {
                reason: StopReason::EndTurn,
                usage: None,
                point: None,
            },
        );
    }
    let page = crate::inbox::list(&db, ListInbox::default()).unwrap();
    assert!(!page.entries.iter().any(|entry| entry.key.session == target));
    let missing = SessionId::random();
    let read = || crate::inbox::attention::read(&db, &[target, empty, missing]).unwrap();
    let before = read();
    assert_eq!(before.len(), 2);
    assert_eq!(before[0].unread.get(), 2);
    assert_eq!(before[0].waiting.get(), 2);
    assert_eq!(before[1].unread.get(), 0);
    crate::inbox::mark(
        &db,
        &MarkInbox {
            key: InboxKey {
                session: target,
                seq: EventSeq::ZERO,
            },
            read: InboxRead::Read,
        },
    )
    .unwrap();
    let after = read();
    assert_eq!(after[0].unread.get(), 1);
    assert_eq!(after[0].waiting.get(), 2);
    let watermark: Option<i64> = db
        .query_row(
            "SELECT read_seq FROM sessions WHERE id=?1",
            [target.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(watermark, None);
    for (seq, request) in [(2, first), (3, second)] {
        append(
            &db,
            target,
            seq,
            AgentEvent::PermissionResolved {
                id: request.id,
                outcome: PermissionOutcome::Cancelled,
            },
        );
    }
    let resolved = read();
    assert_eq!(resolved[0].unread.get(), 0);
    assert_eq!(resolved[0].waiting.get(), 0);
}

#[test]
fn failure_bucket_is_separate_from_unread_and_clears_when_work_restarts() {
    let db = database();
    let target = session(&db);
    append(
        &db,
        target,
        0,
        AgentEvent::TurnFailed {
            message: "fixture failure".to_owned(),
        },
    );
    crate::inbox::mark(
        &db,
        &MarkInbox {
            key: InboxKey {
                session: target,
                seq: EventSeq::ZERO,
            },
            read: InboxRead::Read,
        },
    )
    .unwrap();
    let attention = crate::inbox::attention::read(&db, &[target]).unwrap();
    assert_eq!(attention[0].failed.get(), 1);
    assert_eq!(attention[0].unread.get(), 0);
    append(
        &db,
        target,
        1,
        AgentEvent::TurnStarted {
            content: Vec::new(),
        },
    );
    let attention = crate::inbox::attention::read(&db, &[target]).unwrap();
    assert_eq!(attention[0].failed.get(), 0);
}

#[test]
fn deliberate_unread_survives_empty_inbox_and_exact_seen_until_whole_read() {
    let db = database();
    let target = session(&db);
    let now = DateTime::UNIX_EPOCH;
    let read = || {
        crate::inbox::attention::read(&db, &[target])
            .unwrap()
            .remove(0)
    };
    let _marked = crate::marks::mark(&db, target, vessel_proto::Mark::Unread, now).unwrap();
    assert!(read().explicit_unread);
    assert_eq!(read().unread.get(), 0);
    append(
        &db,
        target,
        0,
        AgentEvent::TurnEnded {
            reason: StopReason::EndTurn,
            usage: None,
            point: None,
        },
    );
    assert!(read().explicit_unread);
    assert_eq!(read().unread.get(), 1);
    crate::inbox::mark(
        &db,
        &MarkInbox {
            key: InboxKey {
                session: target,
                seq: EventSeq::ZERO,
            },
            read: InboxRead::Read,
        },
    )
    .unwrap();
    assert!(read().explicit_unread);
    assert_eq!(read().unread.get(), 0);
    let _marked = crate::marks::mark(
        &db,
        target,
        vessel_proto::Mark::Read {
            through: EventSeq::new(1),
        },
        now,
    )
    .unwrap();
    assert!(!read().explicit_unread);
    assert_eq!(read().unread.get(), 0);
}
