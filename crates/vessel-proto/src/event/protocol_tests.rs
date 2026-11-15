//! Every agent variant participates in the same admission and projection gate.

use super::*;
use crate::agent::{
    PermissionOutcome, PermissionRequest, StopReason, ToolCall, ToolCallStatus, ToolCallUpdate,
    ToolKind,
};
use crate::conversation::{AttachmentKind, AttachmentRef, Question};
use crate::{
    AttachmentId, ContentBlock, EventPage, HarnessSessionId, PermissionId, QuestionId,
    QuestionOutcome, ToolCallId,
};

fn text() -> ContentBlock {
    ContentBlock::Text {
        text: "fixture".into(),
    }
}

fn original_messages() -> Vec<AgentEvent> {
    vec![
        AgentEvent::Ready {
            harness_session: HarnessSessionId::new("fixture"),
            abilities: None,
        },
        AgentEvent::TurnStarted {
            content: vec![text()],
            client_message_id: None,
        },
        AgentEvent::UserMessageChunk { content: text() },
        AgentEvent::MessageChunk { content: text() },
        AgentEvent::ThoughtChunk { content: text() },
        AgentEvent::Plan { entries: vec![] },
        AgentEvent::TurnEnded {
            reason: StopReason::EndTurn,
            usage: None,
            point: None,
        },
        AgentEvent::TurnFailed {
            message: "fixture".into(),
        },
        AgentEvent::HarnessLog {
            line: "fixture".into(),
        },
    ]
}

fn tool_events() -> Vec<AgentEvent> {
    vec![
        AgentEvent::ToolCall(ToolCall {
            scope: None,
            id: ToolCallId::new("call"),
            title: "fixture".into(),
            kind: ToolKind::Read,
            status: ToolCallStatus::Completed,
            content: vec![],
            locations: vec![],
            raw_input: None,

            detail: None,
            failure: None,
            agent_message: None,
        }),
        AgentEvent::ToolCallUpdate(ToolCallUpdate {
            scope: None,
            id: ToolCallId::new("call"),
            title: None,
            kind: None,
            status: None,
            content: None,
            locations: None,
            raw_input: None,
            raw_output: None,

            detail: None,
            failure: None,
            agent_message: None,
        }),
        AgentEvent::PermissionRequested(PermissionRequest {
            plan_proposal: None,
            delivery_state: Default::default(),
            tool_scope: None,
            id: PermissionId::random(),
            tool_call: Some(ToolCallId::new("call")),
            options: vec![],

            presentation: None,
        }),
        AgentEvent::PermissionResolved {
            id: PermissionId::random(),
            outcome: PermissionOutcome::Cancelled,
        },
    ]
}

fn newer_events() -> Vec<AgentEvent> {
    vec![
        AgentEvent::Steered {
            content: vec![text()],
            client_message_id: None,
        },
        AgentEvent::QuestionAsked(Question {
            tool_scope: None,
            id: QuestionId::random(),
            message: "fixture".into(),
            tool_call: None,
            fields: vec![],

            lifetime: crate::timeline::QuestionLifetime::Turn,
            delivery_state: crate::timeline::QuestionDeliveryState::Pending,
        }),
        AgentEvent::QuestionResolved {
            id: QuestionId::random(),
            outcome: QuestionOutcome::Cancelled,
        },
    ]
}

// This exhaustive match makes a new variant choose a test fixture/requirement
// deliberately instead of silently missing the all-variant contract test.
fn expected(event: &AgentEvent) -> Option<ProtocolVersion> {
    match event {
        AgentEvent::Notification(_)
        | AgentEvent::Compaction(_)
        | AgentEvent::Plugin(_)
        | AgentEvent::CommandAccepted { .. }
        | AgentEvent::Error { .. } => crate::timeline::TIMELINE_PARITY,
        AgentEvent::Ready { .. }
        | AgentEvent::TurnStarted { .. }
        | AgentEvent::UserMessageChunk { .. }
        | AgentEvent::MessageChunk { .. }
        | AgentEvent::ThoughtChunk { .. }
        | AgentEvent::ToolCall(_)
        | AgentEvent::ToolCallUpdate(_)
        | AgentEvent::Plan { .. }
        | AgentEvent::PermissionRequested(_)
        | AgentEvent::PermissionResolved { .. }
        | AgentEvent::TurnEnded { .. }
        | AgentEvent::TurnFailed { .. }
        | AgentEvent::HarnessLog { .. } => Some(ProtocolVersion::new(2)),
        AgentEvent::Steered { .. }
        | AgentEvent::QuestionAsked(_)
        | AgentEvent::QuestionResolved { .. } => Some(ProtocolVersion::new(10)),
    }
}

#[test]
fn every_agent_variant_propagates_its_assigned_requirement() {
    for event in original_messages()
        .into_iter()
        .chain(tool_events())
        .chain(newer_events())
    {
        let requirement = expected(&event).expect("legacy variant has an assigned version");
        assert_eq!(event.required_protocol(), Some(requirement), "{event:?}");
        assert_eq!(event.for_protocol(requirement), None);
        let enclosing = SessionEvent::Agent {
            event: event.clone(),
        };
        assert_eq!(enclosing.required_protocol(), Some(requirement));
        assert_eq!(enclosing.for_protocol(requirement), None);
        let encoded = serde_json::to_string(&event).unwrap();
        assert_eq!(serde_json::from_str::<AgentEvent>(&encoded).unwrap(), event);
        if requirement > crate::MIN_PROTOCOL_VERSION {
            assert_record_projection(enclosing, crate::MIN_PROTOCOL_VERSION);
        }
    }
}

fn attachment() -> ContentBlock {
    ContentBlock::Attachment {
        attachment: AttachmentRef {
            id: AttachmentId::random(),
            name: "fixture.txt".parse().unwrap(),
            kind: AttachmentKind::File,
            mime: "text/plain".parse().unwrap(),
            size: 4,
        },
    }
}

#[test]
fn attachment_content_propagates_and_projects_without_losing_the_cursor() {
    let content = attachment();
    for event in [
        AgentEvent::TurnStarted {
            content: vec![text(), content.clone()],
            client_message_id: None,
        },
        AgentEvent::UserMessageChunk { content },
    ] {
        assert_eq!(event.required_protocol(), Some(ProtocolVersion::new(10)));
        let enclosing = SessionEvent::Agent { event };
        assert_eq!(
            enclosing.required_protocol(),
            Some(ProtocolVersion::new(10))
        );
        assert_record_projection(enclosing, ProtocolVersion::new(9));
    }
}

fn assert_record_projection(event: SessionEvent, protocol: ProtocolVersion) {
    let SessionEvent::Agent { event: inner } = &event else {
        panic!("agent fixture")
    };
    let expected = inner
        .for_protocol(protocol)
        .expect("newer event needs its stand-in");
    let record = EventRecord {
        seq: EventSeq::new(7),
        at: DateTime::UNIX_EPOCH,
        event,
    };
    let adapted = record.for_protocol(protocol).unwrap();
    assert_eq!(adapted.seq, record.seq);
    assert_eq!(adapted.at, record.at);
    assert_eq!(adapted.event, SessionEvent::Agent { event: expected });
    assert!(
        adapted
            .event
            .required_protocol()
            .is_some_and(|since| since <= protocol)
    );
    let page = EventPage {
        records: vec![record],
        next: EventSeq::new(8),
        at_end: false,
    };
    let projected = page.for_protocol(protocol).unwrap();
    assert_eq!(projected.records, vec![adapted]);
    assert_eq!(projected.next, page.next);
    assert_eq!(projected.at_end, page.at_end);
}
