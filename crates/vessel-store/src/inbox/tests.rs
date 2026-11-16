//! Projection rules exercised against the real `SQLite` writer-side operations.
use crate::{StoreError, log, schema};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use std::sync::Arc;
use vessel_proto::inbox::{InboxDetail, InboxItem, InboxKey, InboxRead, ListInbox, MarkInbox};
use vessel_proto::{
    AgentEvent, ContentBlock, EventRecord, EventSeq, HarnessKind, PermissionId, PermissionOption,
    PermissionOptionId, PermissionOptionKind, PermissionOutcome, PermissionRequest, SessionEvent,
    SessionId, SessionInfo, SessionMode, SessionSpec, SessionStatus, StopReason, ToolCall,
    ToolCallId, ToolCallStatus, ToolKind,
};

fn database() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    schema::configure(&db).unwrap();
    schema::migrate(&mut db).unwrap();
    db
}
fn now() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).unwrap()
}
fn session(db: &Connection) -> SessionId {
    let info = SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        now(),
    );
    log::create_session(db, &info).unwrap();
    info.id
}
fn append(db: &Connection, session: SessionId, seq: u64, event: AgentEvent) {
    let record = EventRecord {
        seq: EventSeq::new(seq),
        at: now(),
        event: SessionEvent::Agent { event },
    };
    let _appended = log::append(db, session, [&record]).unwrap();
}
fn tool() -> ToolCall {
    ToolCall {
        id: ToolCallId::new("tool"),
        title: "Read a long Korean project title 프로젝트의 변경 내역".to_owned(),
        kind: ToolKind::Read,
        status: ToolCallStatus::Pending,
        content: Vec::new(),
        locations: Vec::new(),
        raw_input: None,
    }
}
fn permission() -> PermissionRequest {
    PermissionRequest {
        id: PermissionId::random(),
        tool_call: ToolCallId::new("tool"),
        options: vec![PermissionOption {
            id: PermissionOptionId::new("allow"),
            name: "Allow once".to_owned(),
            kind: PermissionOptionKind::AllowOnce,
        }],
    }
}

#[test]
fn reads_do_not_answer_and_late_marks_do_not_consume_new_attention() {
    let db = database();
    let id = session(&db);
    let request = permission();
    append(&db, id, 0, AgentEvent::ToolCall(tool()));
    append(&db, id, 1, AgentEvent::PermissionRequested(request.clone()));
    let page = super::list(&db, ListInbox::default()).unwrap();
    let entry = &page.entries[0];
    assert_eq!(entry.read, InboxRead::Unread);
    assert!(entry.item.waiting());
    super::mark(
        &db,
        &MarkInbox {
            key: entry.key,
            read: InboxRead::Read,
        },
        crate::Residence::Stored,
    )
    .unwrap();
    assert_eq!(
        super::list(&db, ListInbox::default()).unwrap().entries[0].read,
        InboxRead::Read
    );
    append(
        &db,
        id,
        2,
        AgentEvent::PermissionResolved {
            id: request.id,
            outcome: PermissionOutcome::Cancelled,
        },
    );
    append(
        &db,
        id,
        3,
        AgentEvent::TurnEnded {
            reason: StopReason::EndTurn,
            usage: None,
            point: None,
        },
    );
    assert!(matches!(
        super::mark(
            &db,
            &MarkInbox {
                key: entry.key,
                read: InboxRead::Read
            },
            crate::Residence::Stored,
        ),
        Err(StoreError::InboxGone)
    ));
    let page = super::list(&db, ListInbox::default()).unwrap();
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].read, InboxRead::Unread);
    assert_eq!(page.entries[0].item, InboxItem::Finished);
    append(
        &db,
        id,
        4,
        AgentEvent::TurnStarted {
            content: Vec::new(),
        },
    );
    assert!(
        super::list(&db, ListInbox::default())
            .unwrap()
            .entries
            .is_empty()
    );
}

#[test]
fn outcomes_distinguish_failure_limits_refusal_and_cancelled_turns() {
    for (reason, expected) in [
        (StopReason::EndTurn, Some(InboxItem::Finished)),
        (StopReason::MaxTokens, Some(InboxItem::Limit)),
        (StopReason::MaxTurnRequests, Some(InboxItem::Limit)),
        (StopReason::Refusal, Some(InboxItem::Failed)),
        (StopReason::Cancelled, None),
    ] {
        let db = database();
        let id = session(&db);
        append(
            &db,
            id,
            0,
            AgentEvent::TurnEnded {
                reason,
                usage: None,
                point: None,
            },
        );
        let page = super::list(&db, ListInbox::default()).unwrap();
        assert_eq!(
            page.entries.first().map(|entry| entry.item.clone()),
            expected
        );
    }
    let db = database();
    let id = session(&db);
    append(
        &db,
        id,
        0,
        AgentEvent::TurnFailed {
            message: "private error detail".to_owned(),
        },
    );
    assert_eq!(
        super::list(&db, ListInbox::default()).unwrap().entries[0].item,
        InboxItem::Failed
    );
}

#[test]
fn huge_tool_details_are_explicit_and_an_updated_pending_title_keeps_read_state() {
    let db = database();
    let id = session(&db);
    let mut large = tool();
    large.content.push(vessel_proto::ToolCallContent::Content {
        content: ContentBlock::Text {
            text: "content".repeat(10_000),
        },
    });
    append(&db, id, 0, AgentEvent::ToolCall(large));
    append(&db, id, 1, AgentEvent::PermissionRequested(permission()));
    let entry = super::list(&db, ListInbox::default())
        .unwrap()
        .entries
        .remove(0);
    let InboxItem::Permission {
        tool: Some(display),
        detail,
        ..
    } = entry.item
    else {
        panic!("permission should keep its tool title");
    };
    assert_eq!(display.title, tool().title);
    assert!(display.content.is_empty());
    assert_eq!(detail, InboxDetail::InSession);
    super::mark(
        &db,
        &MarkInbox {
            key: entry.key,
            read: InboxRead::Read,
        },
        crate::Residence::Stored,
    )
    .unwrap();
    let update: vessel_proto::ToolCallUpdate =
        serde_json::from_str(r#"{"id":"tool","title":"Updated title"}"#).unwrap();
    append(&db, id, 2, AgentEvent::ToolCallUpdate(update));
    let updated = super::list(&db, ListInbox::default())
        .unwrap()
        .entries
        .remove(0);
    assert_eq!(updated.key, entry.key);
    assert_eq!(updated.read, InboxRead::Read);
    let InboxItem::Permission {
        tool: Some(display),
        detail,
        ..
    } = updated.item
    else {
        panic!("a pending card must reflect the update");
    };
    assert_eq!(display.title, "Updated title");
    assert_eq!(detail, InboxDetail::InSession);
}

#[test]
fn descending_pages_never_repeat_after_deletions_reads_or_newer_inserts() {
    let db = database();
    let mut keys = Vec::new();
    for _ in 0..35 {
        let id = session(&db);
        append(
            &db,
            id,
            0,
            AgentEvent::TurnEnded {
                reason: StopReason::EndTurn,
                usage: None,
                point: None,
            },
        );
        keys.push(id);
    }
    let first = super::list(&db, ListInbox::default()).unwrap();
    assert_eq!(first.entries.len(), 16);
    let before = first.next.unwrap();
    let _rows = db
        .execute("DELETE FROM inbox WHERE session=?1", [keys[1].to_string()])
        .unwrap();
    super::mark(
        &db,
        &MarkInbox {
            key: first.entries[0].key,
            read: InboxRead::Read,
        },
        crate::Residence::Stored,
    )
    .unwrap();
    let newer = session(&db);
    append(
        &db,
        newer,
        0,
        AgentEvent::TurnEnded {
            reason: StopReason::EndTurn,
            usage: None,
            point: None,
        },
    );
    let second = super::list(
        &db,
        ListInbox {
            before: Some(before),
        },
    )
    .unwrap();
    assert_eq!(second.entries.len(), 16);
    assert!(second.entries.iter().all(|entry| entry.cursor < before));
    let third = super::list(
        &db,
        ListInbox {
            before: second.next,
        },
    )
    .unwrap();
    assert_eq!(third.entries.len(), 2);
    assert!(third.next.is_none());
    assert!(
        first
            .entries
            .iter()
            .all(|first| !second.entries.iter().any(|second| second.key == first.key))
    );
}

#[test]
fn huge_forms_keep_a_pending_reference_instead_of_clipping_choices() {
    let db = database();
    let id = session(&db);
    let mut request = permission();
    request.options[0].name = "choice".repeat(50_000);
    append(&db, id, 0, AgentEvent::PermissionRequested(request.clone()));
    let page = super::list(&db, ListInbox::default()).unwrap();
    assert_eq!(
        page.entries[0].item,
        InboxItem::InSession {
            request: vessel_proto::inbox::InboxRequest::Permission { id: request.id }
        }
    );
    assert!(page.entries[0].item.waiting());
    assert!(serde_json::to_string(&page).unwrap().len() < 1024);
}

mod lifecycle;

mod limits;

mod tool_updates;

mod delayed_marks;
