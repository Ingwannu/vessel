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
