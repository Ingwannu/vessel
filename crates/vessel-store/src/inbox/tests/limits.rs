//! Real aggregate pool pressure, without falsifying its accounting counter.
use super::*;
use vessel_proto::conversation::{ChoiceOption, FieldKind, QuestionField};
use vessel_proto::{Question, QuestionId, QuestionOutcome};
fn form() -> Question {
    Question {
        lifetime: vessel_proto::timeline::QuestionLifetime::Turn,
        delivery_state: vessel_proto::timeline::QuestionDeliveryState::Pending,
        id: QuestionId::random(),
        message: "Choose a route.".to_owned(),
        tool_call: None,
        fields: (0..32)
            .map(|field| QuestionField {
                presentation: None,
                name: format!("field-{field}").parse().unwrap(),
                title: Some(format!("Field {field}")),
                description: None,
                required: true,
                kind: FieldKind::Choice {
                    options: (0..64)
                        .map(|option| ChoiceOption {
                            value: format!("choice-{option}"),
                            title:
                                "A realistic option label with enough detail to describe the choice"
                                    .to_owned(),
                            description: None,

                            preview: None,
                        })
                        .collect(),
                    multiple: false,
                    min_items: None,
                    max_items: None,
                    default: vec![],
                },
            })
            .collect(),
    }
}
fn bytes(db: &Connection) -> i64 {
    db.query_row(
        "SELECT pending_bytes FROM inbox_meta WHERE singleton=1",
        [],
        |row| row.get(0),
    )
    .unwrap()
}
#[test]
fn complete_payload_pool_is_bounded_compact_requests_survive_and_resolution_reuses_capacity() {
    let db = database();
    for _ in 0..4 {
        let id = session(&db);
        for sequence in 0..64 {
            append(&db, id, sequence, AgentEvent::QuestionAsked(form()));
        }
    }
    let count: i64 = db
        .query_row("SELECT COUNT(*) FROM inbox WHERE waiting=1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 256);
    let deferred: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM inbox WHERE json_extract(item,'$.kind')='in_session'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(deferred > 0);
    let before = bytes(&db);
    assert!(before <= 33_554_432);
    let calculated: i64 = db.query_row("SELECT SUM(length(CAST(item AS BLOB))) FROM inbox WHERE json_extract(item,'$.kind') IN ('permission','question')", [], |row| row.get(0)).unwrap();
    assert_eq!(before, calculated);
    let mut page = super::super::list(&db, ListInbox::default()).unwrap();
    let complete = loop {
        if let Some(entry) = page
            .entries
            .iter()
            .find(|entry| matches!(entry.item, InboxItem::Question { .. }))
        {
            break entry.clone();
        }
        let before = page
            .next
            .expect("a stored complete question must occur in a bounded page");
        page = super::super::list(
            &db,
            ListInbox {
                before: Some(before),
            },
        )
        .unwrap();
    };
    let InboxItem::Question { question } = complete.item else {
        panic!("full question expected");
    };
    append(
        &db,
        complete.key.session,
        64,
        AgentEvent::QuestionResolved {
            id: question.id,
            outcome: QuestionOutcome::Declined,
        },
    );
    assert!(bytes(&db) < before);
    append(
        &db,
        complete.key.session,
        65,
        AgentEvent::QuestionAsked(form()),
    );
    assert_eq!(bytes(&db), before);
    let newest = super::super::list(&db, ListInbox::default()).unwrap();
    assert!(matches!(newest.entries[0].item, InboxItem::Question { .. }));
    assert_eq!(newest.counts.waiting.get(), 256);
}

#[test]
fn omitted_detail_and_each_same_tool_request_participate_in_pool_admission() {
    let db = database();
    let permission_session = session(&db);
    let mut large = tool();
    large.content.push(vessel_proto::ToolCallContent::Content {
        content: ContentBlock::Text {
            text: "large".repeat(10_000),
        },
    });
    append(&db, permission_session, 0, AgentEvent::ToolCall(large));
    append(
        &db,
        permission_session,
        1,
        AgentEvent::PermissionRequested(permission()),
    );
    append(
        &db,
        permission_session,
        2,
        AgentEvent::PermissionRequested(permission()),
    );
    for _ in 0..4 {
        let id = session(&db);
        for sequence in 0..64 {
            append(&db, id, sequence, AgentEvent::QuestionAsked(form()));
        }
    }
    let remaining = usize::try_from(33_554_431 - bytes(&db)).unwrap();
    let mut filler = form();
    filler.message.clear();
    let encoded_size = |question: &Question| {
        serde_json::to_vec(&InboxItem::Question {
            question: question.clone(),
        })
        .unwrap()
        .len()
    };
    while encoded_size(&filler) > remaining {
        let _removed = filler
            .fields
            .pop()
            .expect("one valid field fits the remaining pool");
    }
    assert!(!filler.fields.is_empty());
    let mut gap = remaining - encoded_size(&filler);
    let message = gap.min(Question::MAX_MESSAGE);
    filler.message = "x".repeat(message);
    gap -= message;
    for field in &mut filler.fields {
        if let FieldKind::Choice { options, .. } = &mut field.kind {
            for option in options {
                let extra = gap.min(Question::MAX_LABEL - option.title.len());
                option.title.push_str(&"x".repeat(extra));
                gap -= extra;
            }
        }
    }
    assert_eq!(
        gap, 0,
        "valid bounded labels can fill the exact byte boundary"
    );
    assert_eq!(encoded_size(&filler), remaining);
    append(&db, session(&db), 0, AgentEvent::QuestionAsked(filler));
    assert_eq!(bytes(&db), 33_554_431);
    let update: vessel_proto::ToolCallUpdate =
        serde_json::from_str(&format!(r#"{{"id":"tool","title":"{}x"}}"#, tool().title)).unwrap();
    append(
        &db,
        permission_session,
        3,
        AgentEvent::ToolCallUpdate(update),
    );
    assert!(bytes(&db) <= 33_554_432);
    let kinds: Vec<String> = db
        .prepare("SELECT json_extract(item,'$.kind') FROM inbox WHERE session=?1 ORDER BY seq")
        .unwrap()
        .query_map([permission_session.to_string()], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        kinds,
        ["permission", "in_session"],
        "one byte of headroom permits only one of the two growing requests"
    );
}
