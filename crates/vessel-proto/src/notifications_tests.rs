//! All optional-field paths are projected before reaching an older peer.
use crate::notifications::*;
use crate::terminals::{
    TerminalInfo, TerminalKind, TerminalList, TerminalPlace, TerminalSubscribed,
};
use crate::{
    Call, ClientFrame, Method, NodeId, Outcome, ProtocolVersion, Reply, Request as _, RequestId,
    ServerFrame, SessionId, SubscriptionId, TerminalId,
};
fn terminal() -> TerminalInfo {
    TerminalInfo::new(
        TerminalId::random(),
        TerminalPlace::Session {
            session: SessionId::random(),
        },
        std::path::PathBuf::from("/source-canary"),
        TerminalKind::Shell {
            program: std::path::PathBuf::from("shell"),
        },
        chrono::DateTime::UNIX_EPOCH,
    )
}
#[test]
fn pending_calls_and_terminal_focus_are_never_sent_as_an_older_field() {
    for method in [
        Method::GetInboxAttention,
        Method::ReadInboxThrough,
        Method::ClaimNotification,
        Method::ClearTerminalAttention,
        Method::ReportTerminalActivity,
    ] {
        assert_eq!(method.since(), ATTENTION_NOTIFICATIONS_PROTOCOL);
    }
    let source = NotificationSource::Terminal {
        terminal: TerminalId::random(),
        revision: TerminalAttentionRevision::new(1),
    };
    let claim = ClaimNotification { source }.into_call();
    assert_eq!(claim.since(), ATTENTION_NOTIFICATIONS_PROTOCOL);
    assert_eq!(
        Call::Forward(crate::Forward {
            node: NodeId::random(),
            call: Box::new(claim)
        })
        .since(),
        ATTENTION_NOTIFICATIONS_PROTOCOL
    );
    let focus = crate::push::ReportPushPresence {
        idle_age: None,
        notifies: true,
        focus: crate::push::PushFocus::Terminal {
            terminal: TerminalId::random(),
        },
        activity: crate::push::PushActivity::Unchanged,
    }
    .into_call();
    assert_eq!(focus.since(), ATTENTION_NOTIFICATIONS_PROTOCOL);
    assert!(!Method::ClaimNotification.forwardable());
    assert_eq!(Method::ClaimNotification.scope(), Some(crate::Scope::Read));
    assert_eq!(
        Method::ReportTerminalActivity.scope(),
        Some(crate::Scope::Operate)
    );
}
/// Below ADR 0089's protocol every terminal reply, direct or nested, drops
/// the attention; from it, the attention goes as it is.
#[test]
fn direct_lists_scripts_subscriptions_and_nested_fleet_omit_attention_below_its_protocol() {
    let terminal = terminal();
    let script = crate::scripts::ScriptRunOutcome::Started {
        terminal: Box::new(terminal.clone()),
    };
    let replies = [
        Reply::CreateTerminal(terminal.clone()),
        Reply::GetTerminal(terminal.clone()),
        Reply::RenameTerminal(terminal.clone()),
        Reply::ListTerminals(TerminalList {
            terminals: vec![terminal.clone()],
            revision: 0,
        }),
        Reply::SubscribeTerminal(TerminalSubscribed {
            subscription: SubscriptionId::new(1),
            terminal: terminal.clone(),
        }),
        Reply::ListScripts(crate::scripts::ScriptList {
            root: std::path::PathBuf::from("/source-canary"),
            commands: crate::workspace::ProjectCommands::None,
            runs: vec![terminal],
        }),
        Reply::RunScript(script.clone()),
        Reply::RestartScript(script),
    ];
    let below = ATTENTION_NOTIFICATIONS_PROTOCOL.map_or(crate::PROTOCOL_VERSION, |since| {
        ProtocolVersion::new(since.get().saturating_sub(1))
    });
    for protocol in [ProtocolVersion::new(11), below, crate::PROTOCOL_VERSION] {
        let kept = ATTENTION_NOTIFICATIONS_PROTOCOL.is_some_and(|since| protocol >= since);
        for reply in &replies {
            let wrappers = [
                reply.clone(),
                Reply::Forward(crate::Forwarded {
                    reply: Box::new(reply.clone()),
                }),
                Reply::Relay(crate::Forwarded {
                    reply: Box::new(reply.clone()),
                }),
                Reply::Deliver(crate::Forwarded {
                    reply: Box::new(reply.clone()),
                }),
            ];
            for wrapper in wrappers {
                let frame = ServerFrame::Response {
                    id: RequestId::new(1),
                    outcome: Outcome::Ok(wrapper),
                };
                let older = frame.for_protocol(protocol).unwrap_or(frame);
                let json = serde_json::to_value(&older).unwrap();
                assert_eq!(
                    serde_json::to_string(&json)
                        .unwrap()
                        .contains("\"attention\""),
                    kept,
                    "attention at {protocol:?}"
                );
                let _decoded: ServerFrame = serde_json::from_value(json).unwrap();
            }
        }
    }
}
#[test]
fn a_legacy_terminal_snapshot_has_no_fabricated_activity() {
    let mut json = serde_json::to_value(terminal()).unwrap();
    let _removed = json.as_object_mut().unwrap().remove("attention");
    let old: TerminalInfo = serde_json::from_value(json).unwrap();
    assert_eq!(old.attention, None);
    let clear = ClearTerminalAttention {
        terminal: old.id,
        revision: TerminalAttentionRevision::new(2),
    }
    .into_call();
    let encoded = serde_json::to_string(&ClientFrame::Request {
        id: RequestId::new(1),
        call: clear,
    })
    .unwrap();
    assert!(encoded.contains("terminal.attention_clear"));
}

#[test]
fn elapsed_idle_is_gated_even_when_focusing_no_terminal_and_cannot_encode_future_activity() {
    let request = crate::push::ReportPushPresence {
        focus: crate::push::PushFocus::Elsewhere,
        activity: crate::push::PushActivity::Unchanged,
        idle_age: Some(crate::push::PushIdleAge::new(42)),
        notifies: true,
    }
    .into_call();
    assert_eq!(request.since(), ATTENTION_NOTIFICATIONS_PROTOCOL);
    assert!(serde_json::from_str::<crate::push::PushIdleAge>("-1").is_err());
    let legacy: crate::push::ReportPushPresence =
        serde_json::from_str(r#"{"focus":{"focus":"background"},"activity":"unchanged"}"#).unwrap();
    assert_eq!(legacy.idle_age, None);
    assert!(!serde_json::to_string(&legacy).unwrap().contains("idle_age"));
}

#[test]
fn passive_reads_are_narrow_person_operations_and_idle_age_is_versioned() {
    use crate::inbox_attention::{GetInboxAttention, ReadInboxThrough};
    assert_eq!(Method::GetInboxAttention.scope(), Some(crate::Scope::Read));
    assert_eq!(Method::ReadInboxThrough.scope(), Some(crate::Scope::Read));
    assert!(!Method::ReadInboxThrough.forwardable());
    for call in [
        GetInboxAttention {
            sessions: vec![SessionId::random()],
        }
        .into_call(),
        ReadInboxThrough {
            session: SessionId::random(),
            through: crate::EventSeq::new(9),
        }
        .into_call(),
        crate::push::ReportPushPresence {
            focus: crate::push::PushFocus::Background,
            activity: crate::push::PushActivity::Unchanged,
            idle_age: Some(crate::push::PushIdleAge::new(57)),
            notifies: true,
        }
        .into_call(),
    ] {
        assert_eq!(call.since(), ATTENTION_NOTIFICATIONS_PROTOCOL);
        let encoded = serde_json::to_vec(&call).unwrap();
        assert_eq!(serde_json::from_slice::<Call>(&encoded).unwrap(), call);
    }
}

#[test]
fn the_attention_protocol_is_assigned_and_the_newest() {
    // Assigned 47 (ADR 0089, "Turned on"); only the newest protocol asserts
    // equality with `PROTOCOL_VERSION`, and a later one relaxes this to `<=`.
    assert_eq!(
        ATTENTION_NOTIFICATIONS_PROTOCOL,
        Some(crate::PROTOCOL_VERSION)
    );
}
