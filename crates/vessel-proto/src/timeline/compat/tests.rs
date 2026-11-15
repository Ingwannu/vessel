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
        "scope":{"kind":"provider_turn","thread":"thread-1","turn":"turn-1"},"detail":{"type":"shell","command":"echo ok"},"failure":{"message":"canceled"},
        "agent_message":{"event":"message","sender":{"id":"source"},"text":"check"}}),
    )
    .unwrap()
}

fn permission() -> PermissionRequest {
    PermissionRequest {
        plan_proposal: None,
        delivery_state: crate::timeline::PermissionDeliveryState::Pending,
        tool_scope: Some(Box::new(crate::timeline::ToolCallScope::ProviderTurn {
            thread: "thread-1".parse().unwrap(),
            turn: "turn-1".parse().unwrap(),
        })),
        id: PermissionId::random(),
        tool_call: Some(crate::ToolCallId::new("call")),
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
            detail: Some(Box::new(ToolCallDetail::Plan(PlanDetail {
                text: "Plan".to_owned(),
            }))),
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
        through: Some(EventSeq::new(11)),
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
    assert!(masked.through.is_none());
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
            reference_origin: None,
            reply: Box::new(wrap(Forwarded {
                reference_origin: None,
                reply: Box::new(Reply::ReadEvents(page.clone())),
            })),
        });
        let masked = nested.for_protocol(protocol).unwrap();
        let (Reply::Forward(value) | Reply::Relay(value) | Reply::Deliver(value)) = masked else {
            panic!("outer envelope");
        };
        let (Reply::Forward(inner) | Reply::Relay(inner) | Reply::Deliver(inner)) = *value.reply
        else {
            panic!("inner envelope");
        };
        let inner = inner.reply;
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
            tool: Some(Box::new(rich_tool())),
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

#[test]
fn old_raw_id_folds_keep_reused_scoped_calls_and_permission_references_distinct() {
    use crate::timeline::ToolCallScope;
    use std::collections::HashMap;
    let mut first = rich_tool();
    first.id = crate::ToolCallId::new("reused");
    first.title = "A".into();
    let mut second = first.clone();
    second.title = "B".into();
    second.scope = Some(Box::new(ToolCallScope::ProviderTurn {
        thread: "thread-1".parse().unwrap(),
        turn: "turn-2".parse().unwrap(),
    }));
    let update: ToolCallUpdate =
        serde_json::from_value(json!({"id":"reused","scope":first.scope,"title":"late A"}))
            .unwrap();
    let events = [
        AgentEvent::ToolCall(first.clone()),
        AgentEvent::ToolCall(second.clone()),
        AgentEvent::ToolCallUpdate(update),
    ];
    let assigned = ProtocolVersion::new(30);
    for (peer, gate) in [
        (crate::PROTOCOL_VERSION, None),
        (ProtocolVersion::new(29), Some(assigned)),
    ] {
        let masked: Vec<_> = events
            .iter()
            .map(|event| adapt_event(event, peer, gate).unwrap())
            .collect();
        let mut old = HashMap::new();
        for event in &masked {
            if let AgentEvent::ToolCall(call) = event {
                let _prior = old.insert(call.id.clone(), call.title.clone());
                continue;
            }
            if let AgentEvent::ToolCallUpdate(update) = event {
                old.get_mut(&update.id)
                    .unwrap()
                    .clone_from(update.title.as_ref().unwrap());
                continue;
            }
            panic!("tool lifecycle");
        }
        let [AgentEvent::ToolCall(a), AgentEvent::ToolCall(b), _] = masked.as_slice() else {
            panic!("announcements")
        };
        assert_ne!(a.id, b.id);
        assert_eq!(old[&a.id], "late A");
        assert_eq!(old[&b.id], "B");
        let mut request = permission();
        request.tool_call = Some(second.id.clone());
        request.tool_scope.clone_from(&second.scope);
        let AgentEvent::PermissionRequested(request) =
            adapt_event(&AgentEvent::PermissionRequested(request), peer, gate).unwrap()
        else {
            panic!("permission")
        };
        assert_eq!(request.tool_call, Some(b.id.clone()));
        let mut collision = first.clone();
        collision.id = a.id.clone();
        collision.scope = None;
        let AgentEvent::ToolCall(escaped) =
            adapt_event(&AgentEvent::ToolCall(collision), peer, gate).unwrap()
        else {
            panic!("legacy collision")
        };
        assert_ne!(escaped.id, a.id);
        assert_ne!(escaped.id, b.id);
    }
}

#[test]
fn scoped_legacy_identity_is_identical_on_direct_page_and_nested_outgoing_paths() {
    let event = AgentEvent::ToolCall(rich_tool());
    let protocol = crate::PROTOCOL_VERSION;
    let direct = event.for_protocol(protocol).unwrap();
    let record = EventRecord {
        seq: EventSeq::new(7),
        at: chrono::DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent { event },
    };
    let live = crate::ServerFrame::Event {
        subscription: crate::SubscriptionId::new(3),
        record: record.clone(),
    };
    let crate::ServerFrame::Event {
        subscription,
        record: live_record,
    } = live.for_protocol(protocol).unwrap()
    else {
        panic!("live frame")
    };
    assert_eq!(subscription, crate::SubscriptionId::new(3));
    let masked_record = record.for_protocol(protocol).unwrap();
    assert_eq!(live_record, masked_record);
    assert_eq!(masked_record.seq, record.seq);
    let SessionEvent::Agent { event: from_record } = &masked_record.event else {
        panic!("agent")
    };
    assert_eq!(*from_record, direct);
    let page = EventPage {
        records: vec![record],
        next: EventSeq::new(8),
        at_end: true,
    };
    let masked_page = page.for_protocol(protocol).unwrap();
    assert_eq!(masked_page.next, page.next);
    assert_eq!(masked_page.records, [masked_record]);
    for wrap in [Reply::Forward, Reply::Relay, Reply::Deliver] {
        let nested = wrap(Forwarded {
            reference_origin: None,
            reply: Box::new(Reply::ReadEvents(page.clone())),
        })
        .for_protocol(protocol)
        .unwrap();
        let (Reply::Forward(inner) | Reply::Relay(inner) | Reply::Deliver(inner)) = nested else {
            panic!("nested")
        };
        assert_eq!(*inner.reply, Reply::ReadEvents(masked_page.clone()));
    }
}

#[test]
fn standalone_synthetic_and_admitted_permissions_are_inert_before_old_decode() {
    use crate::timeline::{PermissionDeliveryState, PermissionReference, PlanProposalKey};
    let proposal: PlanProposalKey = serde_json::from_value(json!({
        "thread":"thread", "turn":"turn", "source":{"kind":"legacy_turn_plan"}
    }))
    .unwrap();
    for assigned in [None, Some(ProtocolVersion::new(31))] {
        for state in [
            PermissionDeliveryState::Pending,
            PermissionDeliveryState::Answering,
            PermissionDeliveryState::OutcomeUnknown,
        ] {
            let mut request = permission();
            request.tool_call = None;
            request.plan_proposal = Some(Box::new(proposal.clone()));
            request.delivery_state = state;
            let event = AgentEvent::PermissionRequested(request.clone());
            assert!(matches!(
                adapt_event(&event, ProtocolVersion::new(30), assigned),
                Some(AgentEvent::HarnessLog { .. })
            ));
            let mut snapshot = Conversation {
                through: Some(EventSeq::new(7)),
                queue: PromptQueue::default(),
                commands: Vec::new(),
                usage: None,
                permissions: vec![request.clone()],
                questions: Vec::new(),
                abilities: None,
                limits: AttachmentLimits::DEFAULT,
            };
            snapshot = adapt_conversation(&snapshot, ProtocolVersion::new(30), assigned).unwrap();
            assert!(snapshot.permissions.is_empty());
            let entry = InboxEntry {
                cursor: InboxCursor::new(3),
                key: InboxKey {
                    session: crate::SessionId::random(),
                    seq: EventSeq::new(6),
                },
                title: None,
                project: None,
                project_name: None,
                at: chrono::DateTime::UNIX_EPOCH,
                read: InboxRead::Unread,
                item: InboxItem::Permission {
                    request: request.clone(),
                    tool: None,
                    detail: InboxDetail::Complete,
                },
            };
            let context = ReferenceContext {
                origin: None,
                assigned,
            };
            assert_eq!(
                inbox_with_reference(&entry, ProtocolVersion::new(30), context)
                    .unwrap()
                    .item,
                InboxItem::Unknown
            );
            let mut pointer = entry;
            pointer.item = InboxItem::InSession {
                request: crate::inbox::InboxRequest::Permission { id: request.id },
                question: None,
                permission: PermissionReference::from_request(&request),
            };
            assert_eq!(
                inbox_with_reference(&pointer, ProtocolVersion::new(30), context)
                    .unwrap()
                    .item,
                InboxItem::Unknown
            );
        }
        let mut standalone = permission();
        standalone.tool_call = None;
        assert!(matches!(
            adapt_event(
                &AgentEvent::PermissionRequested(standalone),
                ProtocolVersion::new(30),
                assigned
            ),
            Some(AgentEvent::HarnessLog { .. })
        ));
        let mut admitted = permission();
        admitted.delivery_state = PermissionDeliveryState::Answering;
        assert!(matches!(
            adapt_event(
                &AgentEvent::PermissionRequested(admitted),
                ProtocolVersion::new(30),
                assigned
            ),
            Some(AgentEvent::HarnessLog { .. })
        ));
    }
}

#[test]
fn ordinary_tool_permission_keeps_legacy_json_and_plan_presentation_is_not_a_lifetime() {
    let original = json!({"id": PermissionId::random(), "tool_call":"actual", "options":[]});
    let decoded: PermissionRequest = serde_json::from_value(original.clone()).unwrap();
    assert_eq!(serde_json::to_value(&decoded).unwrap(), original);
    assert_eq!(
        crate::timeline::PermissionReference::from_request(&decoded),
        None
    );
    let mut ordinary_plan = permission();
    ordinary_plan.tool_scope = None;
    let AgentEvent::PermissionRequested(masked) = adapt_event(
        &AgentEvent::PermissionRequested(ordinary_plan),
        ProtocolVersion::new(30),
        None,
    )
    .unwrap() else {
        panic!("ordinary tool request");
    };
    assert_eq!(masked.tool_call, Some(crate::ToolCallId::new("call")));
    assert!(masked.plan_proposal.is_none());
}
