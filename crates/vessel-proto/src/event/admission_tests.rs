//! The availability accessor preserves every assigned session-event threshold
//! and each event's existing stand-in, including its record/page cursor.

use super::*;
use crate::{EventPage, HarnessSessionId, MIN_PROTOCOL_VERSION, SessionId, TerminalBytes};

fn original_events() -> Vec<(SessionEvent, ProtocolVersion)> {
    let version = MIN_PROTOCOL_VERSION;
    vec![
        (
            SessionEvent::Status {
                status: SessionStatus::Running,
            },
            version,
        ),
        (
            SessionEvent::Terminal(TerminalOutput {
                data: TerminalBytes::default(),
            }),
            version,
        ),
        (
            SessionEvent::Agent {
                event: AgentEvent::HarnessLog {
                    line: "fixture".to_owned(),
                },
            },
            version,
        ),
    ]
}

fn metadata_events() -> Vec<(SessionEvent, ProtocolVersion)> {
    let call = serde_json::from_str(
        r#"{
        "route":"fixture","model":"fixture","api":"openai_chat_completions",
        "usage":{},"stop":"end_turn","duration_ms":0
    }"#,
    )
    .unwrap();
    vec![
        (
            SessionEvent::ProfileChanged {
                from: None,
                to: None,
            },
            ProtocolVersion::new(5),
        ),
        (SessionEvent::ModelCall(call), ProtocolVersion::new(7)),
        (
            SessionEvent::OptionsChanged {
                options: Vec::new(),
            },
            ProtocolVersion::new(9),
        ),
        (
            SessionEvent::Renamed {
                title: "fixture".parse().unwrap(),
                origin: crate::TitleOrigin::default(),
            },
            ProtocolVersion::new(9),
        ),
        (
            SessionEvent::QueueChanged {
                queue: PromptQueue::default(),
            },
            ProtocolVersion::new(10),
        ),
        (
            SessionEvent::CommandsChanged {
                commands: Vec::new(),
            },
            ProtocolVersion::new(10),
        ),
        (
            SessionEvent::UsageChanged {
                usage: ContextUsage {
                    observation: crate::conversation::UsageObservation::default(),
                    used: crate::model::TokenCount::default(),
                    size: crate::model::TokenCount::new(1),
                    cost: None,
                },
            },
            ProtocolVersion::new(10),
        ),
    ]
}

fn lifecycle_events() -> Vec<(SessionEvent, ProtocolVersion)> {
    vec![
        (
            SessionEvent::Parked {
                reason: crate::parking::ParkReason::Asked,
            },
            ProtocolVersion::new(12),
        ),
        (SessionEvent::Woken, ProtocolVersion::new(12)),
        (
            SessionEvent::Forked {
                from: SessionId::random(),
                after: None,
            },
            ProtocolVersion::new(14),
        ),
        (
            SessionEvent::Rewound {
                after: EventSeq::new(2),
                harness_session: HarnessSessionId::new("fixture"),
                files_changed: 0,
            },
            ProtocolVersion::new(14),
        ),
    ]
}

fn assert_record_and_page(event: SessionEvent, projected: Option<SessionEvent>) {
    let record = EventRecord {
        seq: EventSeq::new(7),
        at: DateTime::UNIX_EPOCH,
        event,
    };
    let expected = projected.map(|event| EventRecord {
        event,
        ..record.clone()
    });
    assert_eq!(record.for_protocol(MIN_PROTOCOL_VERSION), expected);
    let page = EventPage {
        records: vec![record],
        next: EventSeq::new(8),
        at_end: true,
    };
    let projected_page = page.for_protocol(MIN_PROTOCOL_VERSION);
    assert_eq!(projected_page.is_some(), expected.is_some());
    if let Some(projected_page) = projected_page {
        assert_eq!(projected_page.records, vec![expected.unwrap()]);
        assert_eq!(projected_page.next, page.next);
        assert_eq!(projected_page.at_end, page.at_end);
    }
}

#[test]
fn every_assigned_session_variant_keeps_its_requirement_and_gap_free_projection() {
    for (event, expected) in original_events()
        .into_iter()
        .chain(metadata_events())
        .chain(lifecycle_events())
    {
        assert_eq!(event.required_protocol(), Some(expected), "{event:?}");
        let encoded = serde_json::to_string(&event).unwrap();
        assert_eq!(
            serde_json::from_str::<SessionEvent>(&encoded).unwrap(),
            event
        );
        assert_eq!(event.for_protocol(expected), None);
        let projected = event.for_protocol(MIN_PROTOCOL_VERSION);
        if expected == MIN_PROTOCOL_VERSION {
            assert_eq!(projected, None);
        } else {
            let stand_in = projected.as_ref().expect("a newer event needs a stand-in");
            assert_eq!(stand_in.required_protocol(), Some(MIN_PROTOCOL_VERSION));
        }
        assert_record_and_page(event, projected);
    }
}
