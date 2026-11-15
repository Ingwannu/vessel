//! Agent tool settings stay hidden on direct, legacy mutation and routed replies.

use super::*;
use crate::mcp::{Injection, MCP_PARITY, McpSettings, McpSettingsPatch, ProviderPolicies};
use crate::restart::RestartRecovery;
use crate::settings::{AgentDrivers, NodeSettings, SetNodeSettings};
use crate::{Outcome, Request, RequestId, ServerFrame};

#[test]
fn a_pending_settings_edit_cannot_be_silently_sent_as_an_old_method() {
    let call = SetNodeSettings {
        mcp: Some(McpSettingsPatch {
            inject: Some(Injection::On),
            policy: None,
        }),
        ..Default::default()
    }
    .into_call();
    assert_eq!(
        call.required_protocol(),
        MCP_PARITY.map(|version| version.max(ProtocolVersion::new(12)))
    );
    let node = crate::NodeId::random();
    let forwarded = crate::Forward {
        node,
        call: Box::new(call),
    }
    .into_call();
    assert_eq!(
        forwarded.required_protocol(),
        MCP_PARITY.map(|version| version.max(ProtocolVersion::new(12)))
    );
}

#[test]
fn settings_projection_covers_every_routed_and_direct_reply() {
    let original = NodeSettings {
        park_after: crate::parking::ParkAfter::Never,
        drivers: AgentDrivers::default(),
        restart_recovery: RestartRecovery::default(),
        mcp: Some(McpSettings {
            inject: Injection::On,
            policy: ProviderPolicies::default(),
        }),
    };
    let mut expected = original.clone();
    expected.mcp = None;
    for number in crate::MIN_PROTOCOL_VERSION.get()..=crate::PROTOCOL_VERSION.get() {
        let version = ProtocolVersion::new(number);
        if MCP_PARITY.is_some_and(|required| version >= required) {
            continue;
        }
        for (reply, result) in [
            (
                Reply::GetNodeSettings(original.clone()),
                Reply::GetNodeSettings(expected.clone()),
            ),
            (
                Reply::SetNodeSettings(original.clone()),
                Reply::SetNodeSettings(expected.clone()),
            ),
        ] {
            let variants = |reply: Reply| {
                vec![
                    reply.clone(),
                    Reply::Forward(Forwarded {
                        reply: Box::new(reply.clone()),
                    }),
                    Reply::Relay(Forwarded {
                        reply: Box::new(reply.clone()),
                    }),
                    Reply::Deliver(Forwarded {
                        reply: Box::new(reply.clone()),
                    }),
                    Reply::Forward(Forwarded {
                        reply: Box::new(Reply::Deliver(Forwarded {
                            reply: Box::new(reply),
                        })),
                    }),
                ]
            };
            for (reply, result) in variants(reply).into_iter().zip(variants(result)) {
                assert_eq!(reply.for_protocol(version), Some(result.clone()));
                let frame = |reply| ServerFrame::Response {
                    id: RequestId::new(1),
                    outcome: Outcome::Ok(reply),
                };
                assert_eq!(frame(reply).for_protocol(version), Some(frame(result)));
            }
        }
    }
}
