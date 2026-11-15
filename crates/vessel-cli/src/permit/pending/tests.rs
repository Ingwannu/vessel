//! Payload-budget references remain visible instead of truncating the list.
use super::*;
use vessel_proto::EventSeq;
use vessel_proto::inbox::{InboxCursor, InboxRead};

fn entry(item: InboxItem) -> InboxEntry {
    InboxEntry {
        cursor: InboxCursor::new(1),
        key: InboxKey {
            session: SessionId::random(),
            seq: EventSeq::ZERO,
        },
        title: None,
        project: None,
        project_name: None,
        at: chrono::DateTime::UNIX_EPOCH,
        read: InboxRead::Unread,
        item,
    }
}

#[test]
fn compact_questions_and_permissions_keep_full_ids_and_their_live_lookup_key() {
    let permission = PermissionId::random();
    let question = QuestionId::random();
    for (item, id) in [
        (
            InboxItem::InSession {
                request: InboxRequest::Permission { id: permission },
            },
            PendingId::Permission(permission),
        ),
        (
            InboxItem::InSession {
                request: InboxRequest::Question { id: question },
            },
            PendingId::Question(question),
        ),
    ] {
        let entry = entry(item);
        let row = Row::of(&entry).unwrap().unwrap();
        assert_eq!(row.id, id);
        assert_eq!(row.source, entry.key);
        assert_eq!(row.short_request().chars().count(), REQUEST_PREFIX);
        assert_eq!(row.agent_short_id.chars().count(), AGENT_PREFIX);
        let encoded = serde_json::to_string(&row).unwrap();
        assert!(encoded.contains(&id.to_string()));
        assert!(encoded.contains(&entry.key.session.to_string()));
        assert!(!encoded.contains("source"));
    }
}

#[test]
fn outcomes_are_not_pending_and_unknown_kinds_are_an_explicit_refusal() {
    for item in [InboxItem::Finished, InboxItem::Failed, InboxItem::Limit] {
        assert_eq!(Row::of(&entry(item)).unwrap(), None);
    }
    assert_eq!(Row::of(&entry(InboxItem::Unknown)), Err(UnsupportedKind));
}

#[test]
fn permit_rows_use_the_reviewed_formatter_with_correct_identity_fidelity() {
    use crate::args::parity::{Presentation, output::Headers};
    use crate::parity::output::render_list;
    let session = SessionId::random();
    let id = PendingId::Question(QuestionId::random());
    let row = Row {
        id,
        agent_id: session,
        agent_short_id: session.to_string().chars().take(AGENT_PREFIX).collect(),
        name: "Question".into(),
        description: "Line one\nLine two".into(),
        source: InboxKey {
            session,
            seq: EventSeq::new(41),
        },
    };
    let table =
        String::from_utf8(render_list(Presentation::Table(Headers::Show), &[row.clone()]).unwrap())
            .unwrap();
    assert!(table.contains("AGENT") && table.contains("REQ_ID") && table.contains("DESCRIPTION"));
    assert!(table.contains(&row.agent_short_id) && table.contains(&row.short_request()));
    assert!(!table.contains(&session.to_string()) && !table.contains(&id.to_string()));
    assert!(
        table.contains("Line one\\nLine two"),
        "shared escaping is applied exactly once"
    );
    let quiet =
        String::from_utf8(render_list(Presentation::Quiet, &[row.clone()]).unwrap()).unwrap();
    assert_eq!(quiet.trim_end(), id.to_string());
    for mode in [Presentation::Json, Presentation::Yaml] {
        let structured = String::from_utf8(render_list(mode, &[row.clone()]).unwrap()).unwrap();
        assert!(structured.contains(&session.to_string()));
        assert!(structured.contains(&id.to_string()));
        assert!(!structured.contains("source") && !structured.contains("seq"));
    }
}
