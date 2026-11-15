use super::*;
use crate::timeline::{ClientMessageId, ReferenceContext, SubmissionDelivery, SubmitPrompt};
use crate::{
    ContentBlock, CreateSession, Creator, EventPage, EventRecord, Forwarded, HarnessKind, NodeId,
    Outcome, PromptQueue, ProtocolVersion, QueueChange, QueueEntryId, QueuedPrompt, Reply,
    RequestId, ServerFrame, SessionEvent, SessionId, SessionInput, SessionMode, SessionSpec,
};
use serde_json::json;

const MODERN: ProtocolVersion = ProtocolVersion::new(30);
const OLD: ProtocolVersion = ProtocolVersion::new(29);

fn association() -> Option<PromptProvenance> {
    Some(PromptProvenance::InternalNotice {
        notice: EventSeq::new(5),
    })
}
fn text() -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        text: "bounded private notice body".into(),
    }]
}
fn queue() -> PromptQueue {
    PromptQueue {
        entries: vec![QueuedPrompt {
            provenance: association(),
            client_message_id: None,
            id: QueueEntryId::random(),
            content: text(),
            queued_at: chrono::DateTime::UNIX_EPOCH,
            revision: 0,
            by: Creator::Owner,
        }],
        held: None,
    }
}
fn events() -> Vec<AgentEvent> {
    vec![
        AgentEvent::TurnStarted {
            provenance: association(),
            content: text(),
            client_message_id: None,
        },
        AgentEvent::Steered {
            provenance: association(),
            content: text(),
            client_message_id: None,
        },
        AgentEvent::UserMessageChunk {
            provenance: association(),
            content: text().remove(0),
        },
    ]
}

#[test]
fn ordinary_json_is_unchanged_and_provider_history_cannot_mint_an_association() {
    for mut event in events() {
        assert_eq!(event.prompt_provenance(), association());
        event.clear_foreign_prompt_provenance();
        assert_eq!(event.prompt_provenance(), None);
        let wire = serde_json::to_value(&event).unwrap();
        assert!(wire.get("provenance").is_none());
        let blocks = if let AgentEvent::TurnStarted { content, .. }
        | AgentEvent::Steered { content, .. } = &event
        {
            content.clone()
        } else if let AgentEvent::UserMessageChunk { content, .. } = &event {
            vec![content.clone()]
        } else {
            panic!("fixture");
        };
        assert_eq!(blocks, text(), "foreign words remain ordinary input");
    }
    let mut entry = queue().entries.remove(0);
    entry.provenance = None;
    assert!(
        serde_json::to_value(entry)
            .unwrap()
            .get("provenance")
            .is_none()
    );
    assert!(
        serde_json::from_value::<PromptProvenance>(
            json!({"kind":"provider_asserts_notice","notice":5})
        )
        .is_err()
    );
}

#[test]
fn public_submit_create_first_input_and_queue_requests_have_no_provenance_selector() {
    let submit = SubmitPrompt {
        session: SessionId::random(),
        message: ClientMessageId::random(),
        content: text(),
        delivery: SubmissionDelivery::Start,
        native_command: None,
    };
    let mut value = serde_json::to_value(&submit).unwrap();
    value["provenance"] = serde_json::to_value(association()).unwrap();
    assert_eq!(
        serde_json::from_value::<SubmitPrompt>(value).unwrap(),
        submit
    );
    let create = CreateSession {
        spec: SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: vec![],
            },
        ),
    };
    let mut value = serde_json::to_value(&create).unwrap();
    value["provenance"] = serde_json::to_value(association()).unwrap();
    value["first_input"] = json!({"provenance":association(),"content":text()});
    assert_eq!(
        serde_json::from_value::<CreateSession>(value).unwrap(),
        create
    );
    let input = SessionInput::Prompt { content: text() };
    let mut value = serde_json::to_value(&input).unwrap();
    value["provenance"] = serde_json::to_value(association()).unwrap();
    assert_eq!(
        serde_json::from_value::<SessionInput>(value).unwrap(),
        input
    );
    let change = QueueChange::Edit {
        entry: QueueEntryId::random(),
        revision: 0,
        content: text(),
    };
    let mut value = serde_json::to_value(&change).unwrap();
    value["provenance"] = serde_json::to_value(association()).unwrap();
    assert_eq!(
        serde_json::from_value::<QueueChange>(value).unwrap(),
        change
    );
}

fn page() -> EventPage {
    let mut records: Vec<_> = events()
        .into_iter()
        .enumerate()
        .map(|(index, event)| EventRecord {
            seq: EventSeq::new(10 + u64::try_from(index).unwrap()),
            at: chrono::DateTime::UNIX_EPOCH,
            event: SessionEvent::Agent { event },
        })
        .collect();
    records.push(EventRecord {
        seq: EventSeq::new(13),
        at: chrono::DateTime::UNIX_EPOCH,
        event: SessionEvent::QueueChanged { queue: queue() },
    });
    EventPage {
        records,
        next: EventSeq::new(14),
        at_end: true,
    }
}
fn hop(
    value: Forwarded,
    wrap: fn(Forwarded) -> Reply,
    peer: ProtocolVersion,
    assigned: Option<ProtocolVersion>,
) -> Forwarded {
    let original = wrap(value);
    let reply = original
        .for_protocol_with_reference(
            peer,
            ReferenceContext {
                origin: None,
                assigned,
            },
        )
        .unwrap_or(original);
    let frame = ServerFrame::Response {
        id: RequestId::new(8),
        outcome: Outcome::Ok(reply),
    };
    let bytes = serde_json::to_vec(&frame).unwrap();
    let ServerFrame::Response {
        outcome: Outcome::Ok(reply),
        ..
    } = serde_json::from_slice(&bytes).unwrap()
    else {
        panic!("response");
    };
    let (Reply::Forward(mut inner) | Reply::Relay(mut inner) | Reply::Deliver(mut inner)) = reply
    else {
        panic!("fleet envelope");
    };
    inner.received_with(peer, assigned);
    inner
}

#[test]
fn one_walk_masks_private_payloads_across_real_encoded_old_middle_hops() {
    for assigned in [None, Some(MODERN)] {
        for wrap in [Reply::Forward, Reply::Relay, Reply::Deliver] {
            let original = page();
            let value = Forwarded {
                node: NodeId::random(),
                reference_origin: None,
                reply: Box::new(Reply::ReadEvents(original.clone())),
            };
            let first = hop(value, wrap, OLD, assigned);
            let second = hop(first.clone(), wrap, MODERN, assigned);
            assert_eq!(first.reply, second.reply);
            let Reply::ReadEvents(masked) = *second.reply else {
                panic!("page");
            };
            assert_eq!(masked.next, original.next);
            assert_eq!(masked.at_end, original.at_end);
            for (before, after) in original.records.iter().zip(&masked.records) {
                assert_eq!((before.seq, before.at), (after.seq, after.at));
                assert_eq!(
                    std::mem::discriminant(&before.event),
                    std::mem::discriminant(&after.event)
                );
                if let (
                    SessionEvent::Agent { event: before },
                    SessionEvent::Agent { event: after },
                ) = (&before.event, &after.event)
                {
                    assert_eq!(
                        std::mem::discriminant(before),
                        std::mem::discriminant(after)
                    );
                }
            }
            let serialized = serde_json::to_string(&masked).unwrap();
            assert!(!serialized.contains("bounded private notice body"));
            assert!(!serialized.contains("provenance"));
        }
    }
    let original = page();
    let value = Forwarded {
        node: NodeId::random(),
        reference_origin: None,
        reply: Box::new(Reply::ReadEvents(original.clone())),
    };
    let first = hop(value, Reply::Forward, MODERN, Some(MODERN));
    let second = hop(first, Reply::Deliver, MODERN, Some(MODERN));
    assert_eq!(*second.reply, Reply::ReadEvents(original));
}

#[test]
fn typed_reader_leaves_keep_the_link_and_identity_without_an_extra_privacy_walk() {
    for mut event in events() {
        let kind = std::mem::discriminant(&event);
        event.clear_internal_notice_words();
        assert_eq!(event.prompt_provenance(), association());
        assert_eq!(std::mem::discriminant(&event), kind);
        assert!(
            !serde_json::to_string(&event)
                .unwrap()
                .contains("bounded private notice body")
        );
        let once = event.clone();
        event.clear_internal_notice_words();
        assert_eq!(event, once);
    }
    let mut entry = queue().entries.remove(0);
    let original = entry.clone();
    entry.clear_internal_notice_words();
    assert!(entry.content.is_empty());
    assert_eq!(
        (
            entry.id,
            entry.revision,
            entry.queued_at,
            entry.by,
            entry.provenance
        ),
        (
            original.id,
            original.revision,
            original.queued_at,
            original.by.clone(),
            original.provenance
        )
    );
    let mut ordinary = original;
    ordinary.provenance = None;
    let original = ordinary.clone();
    ordinary.clear_internal_notice_words();
    assert_eq!(ordinary, original);
}
