//! Tests of the event attachment, kept apart from standalone receipt values.
use crate::native_commands::{NativeCommandIntent, NativeCommandStatus};
use crate::timeline::{ClientMessageId, PromptReceipt, SubmissionAcceptedKind};
use crate::{AgentEvent, ContentBlock, EventSeq};
#[test]
fn command_acceptance_echo_is_not_a_turn_and_keeps_one_client_identity() {
    let message = ClientMessageId::random();
    let receipt = PromptReceipt {
        message: message.clone(),
        accepted: SubmissionAcceptedKind::Command {
            status: NativeCommandStatus::CompactStarted,
            echo: EventSeq::new(5),
        },
    };
    let event = AgentEvent::CommandAccepted {
        client_message_id: message,
        content: vec![ContentBlock::Text {
            text: "/compact".to_owned(),
        }],
        status: NativeCommandStatus::CompactStarted,
    };
    assert_eq!(event.required_protocol(), crate::timeline::TIMELINE_PARITY);
    assert!(matches!(
        event.for_protocol(crate::PROTOCOL_VERSION),
        Some(AgentEvent::HarnessLog { .. })
    ));
    let json = serde_json::to_string(&receipt).unwrap();
    assert_eq!(
        serde_json::from_str::<PromptReceipt>(&json).unwrap(),
        receipt
    );
    assert!(
        NativeCommandIntent::Compact.matches_content(&[ContentBlock::Text {
            text: "/compact".to_owned()
        }])
    );
    assert!(
        !NativeCommandIntent::Compact.matches_content(&[ContentBlock::Text {
            text: "/goal pause".to_owned()
        }])
    );
}
