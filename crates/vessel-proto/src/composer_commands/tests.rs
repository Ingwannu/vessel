//! Pending metadata cannot leak through direct or forwarded old-protocol paths.

use super::*;
use crate::forward::Forwarded;
use crate::rpc::Reply;
use crate::{AttachmentLimits, EventRecord, EventSeq, PromptQueue, SessionEvent};

fn command() -> AgentCommand {
    AgentCommand {
        name: "review".parse().unwrap(),
        description: "Review changes".to_owned(),
        hint: None,
        kind: Some(CommandKind::Skill),
    }
}

fn conversation() -> Conversation {
    Conversation {
        queue: PromptQueue::default(),
        commands: vec![command()],
        usage: None,
        permissions: Vec::new(),
        questions: Vec::new(),
        abilities: None,
        limits: AttachmentLimits::DEFAULT,
    }
}

#[test]
fn old_reports_decode_without_kind_and_future_kinds_stay_ineligible() {
    let old: AgentCommand =
        serde_json::from_str(r#"{"name":"review","description":"review","hint":null}"#).unwrap();
    assert_eq!(old.kind, None);
    let future: AgentCommand =
        serde_json::from_str(r#"{"name":"review","description":"review","kind":"future"}"#)
            .unwrap();
    assert_eq!(future.kind, Some(CommandKind::Unknown));
    assert_ne!(future.kind, Some(CommandKind::Skill));
}

#[test]
fn assignment_gate_strips_only_metadata_and_keeps_command_order_and_words() {
    let peer = ProtocolVersion::new(30);
    let commands = vec![command()];
    for assigned in [
        None,
        Some(ProtocolVersion::new(peer.get().saturating_add(1))),
    ] {
        let projected = commands_for(&commands, peer, assigned).unwrap();
        assert_eq!(projected[0].kind, None);
        assert_eq!(projected[0].name, commands[0].name);
        assert_eq!(projected[0].description, commands[0].description);
    }
    assert_eq!(commands_for(&commands, peer, Some(peer)), None);
    assert_eq!(
        commands_for(
            &commands,
            peer,
            Some(ProtocolVersion::new(peer.get().saturating_sub(1)))
        ),
        None
    );
}

#[test]
fn pending_metadata_is_hidden_in_direct_snapshot_and_every_envelope() {
    assert_eq!(COMPOSER_COMMANDS, None);
    let peer = crate::PROTOCOL_VERSION;
    let reply = Reply::GetConversation(conversation());
    let projected = reply.for_protocol(peer).unwrap();
    let Reply::GetConversation(conversation) = projected.clone() else {
        panic!("snapshot");
    };
    assert_eq!(conversation.commands[0].kind, None);
    let nested = Reply::Forward(Forwarded {
        reply: Box::new(Reply::Relay(Forwarded {
            reply: Box::new(Reply::Deliver(Forwarded {
                reply: Box::new(reply),
            })),
        })),
    });
    let encoded = serde_json::to_string(&nested.for_protocol(peer).unwrap()).unwrap();
    assert!(!encoded.contains("skill"));
    assert!(encoded.contains("review"));
}

#[test]
fn pending_metadata_is_hidden_in_events_records_and_legacy_log_projection() {
    let event = SessionEvent::CommandsChanged {
        commands: vec![command()],
    };
    let projected = event.for_protocol(crate::PROTOCOL_VERSION).unwrap();
    let SessionEvent::CommandsChanged { commands } = &projected else {
        panic!("commands");
    };
    assert_eq!(commands[0].kind, None);
    let record = EventRecord {
        seq: EventSeq::new(7),
        at: chrono::DateTime::UNIX_EPOCH,
        event,
    };
    let projected = record.for_protocol(crate::PROTOCOL_VERSION).unwrap();
    assert_eq!(record.seq, projected.seq);
    assert_eq!(record.at, projected.at);
    let legacy = record.for_protocol(ProtocolVersion::new(9)).unwrap();
    assert!(matches!(
        legacy.event,
        SessionEvent::Agent {
            event: crate::AgentEvent::HarnessLog { .. }
        }
    ));
}
