//! ADR 0116: a list poll and a resize naming its intent need
//! `SHARED_TERMINALS_PROTOCOL`, where an older node would answer the poll
//! at once (a client asking again on each answer would spin) or apply
//! another client's update; without them the calls stay at 11.
use super::*;
use crate::session::SessionInput;
use crate::terminal::{ResizeIntent, TerminalInput};
use crate::terminals::{ListTerminals, SHARED_TERMINALS_PROTOCOL, SendTerminal};
use crate::{SendInput, SessionId, TerminalId};

fn resize(intent: Option<ResizeIntent>) -> TerminalInput {
    TerminalInput::Resize {
        size: TerminalSize::default(),
        intent,
    }
}

#[test]
fn a_list_poll_needs_the_shared_terminals_protocol() {
    let asked = |changed_from| {
        ListTerminals {
            place: None,
            changed_from,
        }
        .into_call()
    };
    assert_eq!(asked(None).since(), Some(ProtocolVersion::new(11)));
    assert_eq!(asked(Some(4)).since(), SHARED_TERMINALS_PROTOCOL);
    assert!(SHARED_TERMINALS_PROTOCOL <= Some(crate::PROTOCOL_VERSION));
}

#[test]
fn a_resize_naming_its_intent_needs_the_shared_terminals_protocol() {
    let terminal = |intent| {
        SendTerminal {
            terminal: TerminalId::random(),
            input: resize(intent),
        }
        .into_call()
    };
    assert_eq!(terminal(None).since(), Some(ProtocolVersion::new(11)));
    assert_eq!(
        terminal(Some(ResizeIntent::Update)).since(),
        SHARED_TERMINALS_PROTOCOL
    );
    let session = |intent| {
        SendInput {
            session: SessionId::random(),
            input: SessionInput::Terminal {
                input: resize(intent),
            },
        }
        .into_call()
    };
    assert_eq!(session(None).since(), Some(ProtocolVersion::new(2)));
    assert_eq!(
        session(Some(ResizeIntent::Claim)).since(),
        SHARED_TERMINALS_PROTOCOL
    );
}

#[test]
fn an_intent_and_a_poll_are_left_out_of_the_wire_when_absent() {
    let plain = serde_json::to_value(resize(None)).unwrap();
    assert_eq!(
        plain,
        serde_json::json!({"kind": "resize", "size": {"cols": 80, "rows": 24}})
    );
    let claimed = serde_json::to_value(resize(Some(ResizeIntent::Claim))).unwrap();
    assert_eq!(claimed["intent"], "claim");
    let listed = serde_json::to_value(ListTerminals::default()).unwrap();
    assert_eq!(listed, serde_json::json!({}));
}
