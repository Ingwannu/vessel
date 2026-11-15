//! Compatibility tests exercise real outgoing owners, not just isolated field copies.

use serde_json::json;

use super::*;
use super::{conversation as adapt_conversation, event as adapt_event};
use crate::conversation::AttachmentLimits;
use crate::inbox::{InboxCounts, InboxCursor, InboxDetail, InboxKey, InboxPage, InboxRead};
use crate::timeline::{
    ClientMessageId, PermissionAction, PermissionBehavior, PermissionKind, PermissionPresentation,
    PlanDetail, ToolCallDetail,
};
use crate::{
    Creator, Delivered, EventPage, EventRecord, EventSeq, Forwarded, PermissionId,
    PermissionOption, PermissionOptionId, PermissionOptionKind, QuestionId, QueueEntryId,
    QueuedPrompt, Reply, SessionEvent,
};

fn rich_tool() -> ToolCall {
    serde_json::from_value(
        json!({"id":"call","title":"run","kind":"execute","status":"canceled",
        "detail":{"type":"shell","command":"echo ok"},"failure":{"message":"canceled"},
        "agent_message":{"event":"message","sender":{"id":"source"},"text":"check"}}),
    )
    .unwrap()
}

fn permission() -> PermissionRequest {
    PermissionRequest {
        id: PermissionId::random(),
        tool_call: crate::ToolCallId::new("call"),
        options: vec![PermissionOption {
            id: PermissionOptionId::new("implement"),
            name: "Implement".to_owned(),
            kind: PermissionOptionKind::AllowOnce,
            action: Some(PermissionAction {
                behavior: PermissionBehavior::Allow,
                variant: None,
                intent: Some(crate::timeline::PermissionActionIntent::Implement),
            }),
        }],
        presentation: Some(PermissionPresentation {
            kind: PermissionKind::Plan,
            title: Some("Plan".to_owned()),
            description: None,
            detail: Some(ToolCallDetail::Plan(PlanDetail {
                text: "Plan".to_owned(),
            })),
            plan_text: Some("Plan".to_owned()),
        }),
    }
}

fn question(lifetime: QuestionLifetime) -> Question {
    serde_json::from_value(
        json!({"id":QuestionId::random(),"message":"Choose","lifetime":lifetime,"fields":[{
            "name":"route","title":"Question 1","required":true,"kind":"choice","multiple":false,
            "presentation":{"empty":"require_text","placeholder":"Type","dismiss_label":"Later"},
            "options":[{"value":"left","title":"Left","preview":"Preview"}]
        }]}),
    )
    .unwrap()
}

fn queued() -> PromptQueue {
    PromptQueue {
        entries: vec![QueuedPrompt {
            id: QueueEntryId::random(),
            content: vec![],
            queued_at: chrono::DateTime::UNIX_EPOCH,
            revision: 0,
            by: Creator::Owner,
            client_message_id: Some(ClientMessageId::random()),
        }],
        held: None,
    }
}

#[test]
fn optional_fields_and_canceled_status_obey_unassigned_and_synthetic_assigned_thresholds() {
    let version = ProtocolVersion::new(30);
    let older = ProtocolVersion::new(29);
    let event = AgentEvent::ToolCall(rich_tool());
    for (peer, assigned) in [(version, None), (older, Some(version))] {
        let AgentEvent::ToolCall(masked) = adapt_event(&event, peer, assigned).unwrap() else {
            panic!("tool retained")
        };
        assert_eq!(masked.status, ToolCallStatus::Failed);
        assert!(
            masked.detail.is_none() && masked.failure.is_none() && masked.agent_message.is_none()
        );
    }
    for peer in [version, ProtocolVersion::new(31)] {
        assert!(adapt_event(&event, peer, Some(version)).is_none());
    }
}

#[test]
fn question_snapshots_hide_nonblocking_or_uncertain_requests_instead_of_false_actionability() {
    let protocol = crate::PROTOCOL_VERSION;
    let value = Conversation {
        queue: queued(),
        commands: vec![],
        usage: None,
        permissions: vec![permission()],
        questions: vec![
            question(QuestionLifetime::Turn),
            question(QuestionLifetime::Session),
        ],
        abilities: None,
        limits: AttachmentLimits::DEFAULT,
    };
    let masked = adapt_conversation(&value, protocol, None).unwrap();
    assert_eq!(masked.questions.len(), 1);
    assert!(masked.permissions[0].presentation.is_none());
    assert!(masked.permissions[0].options[0].action.is_none());
    assert!(masked.questions[0].fields[0].presentation.is_none());
    let crate::conversation::FieldKind::Choice { options, .. } =
        &masked.questions[0].fields[0].kind
    else {
        panic!("choice")
    };
    assert!(options[0].preview.is_none());
    assert!(masked.queue.entries[0].client_message_id.is_none());
    let mut uncertain = question(QuestionLifetime::Turn);
    uncertain.delivery_state = QuestionDeliveryState::OutcomeUnknown;
    let masked = adapt_event(&AgentEvent::QuestionAsked(uncertain), protocol, None).unwrap();
    assert!(matches!(masked, AgentEvent::HarnessLog { .. }));
    assert!(adapt_conversation(&value, protocol, Some(protocol)).is_none());
}

#[test]
fn direct_records_pages_and_every_nested_envelope_use_the_same_optional_field_mask() {
    let protocol = crate::PROTOCOL_VERSION;
    let event = SessionEvent::Agent {
        event: AgentEvent::ToolCall(rich_tool()),
    };
    let record = EventRecord {
        seq: EventSeq::new(7),
        at: chrono::DateTime::UNIX_EPOCH,
        event,
    };
    let masked = record.for_protocol(protocol).unwrap();
    assert_eq!(masked.seq, record.seq);
    assert_eq!(masked.at, record.at);
    let page = EventPage {
        records: vec![record],
        next: EventSeq::new(8),
        at_end: true,
    };
    let expected = Reply::ReadEvents(page.clone())
        .for_protocol(protocol)
        .unwrap();
    let wrappers: [fn(Forwarded) -> Reply; 3] = [Reply::Forward, Reply::Relay, Reply::Deliver];
    for wrap in wrappers {
        let nested = wrap(Forwarded {
            reply: Box::new(wrap(Forwarded {
                reply: Box::new(Reply::ReadEvents(page.clone())),
            })),
        });
        let masked = nested.for_protocol(protocol).unwrap();
        let inner = match masked {
            Reply::Forward(value) | Reply::Relay(value) | Reply::Deliver(value) => value.reply,
            other => panic!("unexpected {other:?}"),
        };
        let inner = match *inner {
            Reply::Forward(value) | Reply::Relay(value) | Reply::Deliver(value) => value.reply,
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(*inner, expected);
    }
    let queue_reply = Reply::ChangeQueue(crate::conversation::QueueChanged {
        queue: queued(),
        delivered: Some(Delivered::Started),
    });
    let Reply::ChangeQueue(masked) = queue_reply.for_protocol(protocol).unwrap() else {
        panic!("queue")
    };
    assert!(masked.queue.entries[0].client_message_id.is_none());
}

#[test]
fn inbox_page_and_live_recovery_strip_metadata_and_hide_async_actionability() {
    let protocol = crate::PROTOCOL_VERSION;
    let key = InboxKey {
        session: crate::SessionId::random(),
        seq: EventSeq::new(9),
    };
    let entry = InboxEntry {
        cursor: InboxCursor::new(1),
        key,
        title: None,
        project: None,
        project_name: None,
        at: chrono::DateTime::UNIX_EPOCH,
        read: InboxRead::Unread,
        item: InboxItem::Permission {
            request: permission(),
            tool: Some(rich_tool()),
            detail: InboxDetail::Complete,
        },
    };
    let reply = Reply::ListInbox(InboxPage {
        counts: InboxCounts::default(),
        entries: vec![entry.clone()],
        next: None,
    });
    let Reply::ListInbox(masked) = reply.for_protocol(protocol).unwrap() else {
        panic!("page")
    };
    let InboxItem::Permission {
        request,
        tool: Some(tool),
        ..
    } = &masked.entries[0].item
    else {
        panic!("permission")
    };
    assert!(request.presentation.is_none());
    assert!(tool.detail.is_none());
    assert_eq!(tool.status, ToolCallStatus::Failed);
    let mut async_entry = entry;
    async_entry.item = InboxItem::Question {
        question: question(QuestionLifetime::Session),
    };
    let Reply::GetInboxEntry(Some(masked)) = Reply::GetInboxEntry(Some(async_entry))
        .for_protocol(protocol)
        .unwrap()
    else {
        panic!("live entry")
    };
    assert_eq!(masked.item, InboxItem::Unknown);
}

#[test]
fn echo_mask_and_attachment_downgrade_compose_for_the_oldest_peer() {
    let attachment = crate::AttachmentRef {
        id: crate::AttachmentId::random(),
        name: "note.txt".parse().unwrap(),
        kind: crate::AttachmentKind::File,
        mime: "text/plain".parse().unwrap(),
        size: 4,
    };
    let event = AgentEvent::TurnStarted {
        content: vec![crate::ContentBlock::Attachment { attachment }],
        client_message_id: Some(ClientMessageId::random()),
    };
    let AgentEvent::TurnStarted {
        content,
        client_message_id,
    } = event.for_protocol(ProtocolVersion::new(2)).unwrap()
    else {
        panic!("turn")
    };
    assert!(client_message_id.is_none());
    assert!(matches!(
        content.as_slice(),
        [crate::ContentBlock::ResourceLink { .. }]
    ));
}
