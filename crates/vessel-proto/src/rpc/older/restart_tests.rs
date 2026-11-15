//! Existing settings replies must not expose an unassigned feature on any hop.
use super::*;
use crate::parking::ParkAfter;
use crate::restart::{RESTART_RECOVERY_PROTOCOL, RestartRecovery};
use crate::settings::{AgentDriver, AgentDrivers, NodeSettings};
use crate::{Outcome, RequestId, ServerFrame};

#[test]
fn direct_get_and_legacy_set_hide_unassigned_saved_recovery() {
    for enabled in [false, true] {
        let mut original = settings();
        original.restart_recovery.enabled = enabled;
        let mut expected = original.clone();
        expected.restart_recovery = RestartRecovery::default();
        for protocol in unsupported_versions() {
            for (reply, projected) in replies(original.clone())
                .into_iter()
                .zip(replies(expected.clone()))
            {
                assert_eq!(reply.for_protocol(protocol), Some(projected.clone()));
                let frame = response(reply);
                assert_eq!(frame.for_protocol(protocol), Some(response(projected)));
            }
            for reply in replies(expected.clone()) {
                assert_eq!(reply.for_protocol(protocol), None);
            }
        }
    }
}

#[test]
fn forward_relay_deliver_and_nested_settings_use_the_same_projection() {
    let original = settings();
    let mut expected = original.clone();
    expected.restart_recovery = RestartRecovery::default();
    for protocol in unsupported_versions() {
        for (reply, projected) in replies(original.clone())
            .into_iter()
            .zip(replies(expected.clone()))
        {
            for (envelope, expected_envelope) in
                envelopes(reply).into_iter().zip(envelopes(projected))
            {
                assert_eq!(
                    envelope.for_protocol(protocol),
                    Some(expected_envelope.clone())
                );
                let nested = Reply::Forward(Forwarded {
                    reply: Box::new(envelope),
                });
                let expected_nested = Reply::Forward(Forwarded {
                    reply: Box::new(expected_envelope),
                });
                assert_eq!(nested.for_protocol(protocol), Some(expected_nested));
            }
        }
    }
}

fn unsupported_versions() -> impl Iterator<Item = ProtocolVersion> {
    (crate::MIN_PROTOCOL_VERSION.get()..=crate::PROTOCOL_VERSION.get())
        .map(ProtocolVersion::new)
        .filter(|protocol| RESTART_RECOVERY_PROTOCOL.is_none_or(|required| *protocol < required))
}

fn replies(settings: NodeSettings) -> [Reply; 2] {
    [
        Reply::GetNodeSettings(settings.clone()),
        Reply::SetNodeSettings(settings),
    ]
}

fn envelopes(reply: Reply) -> [Reply; 3] {
    [
        Reply::Forward(Forwarded {
            reply: Box::new(reply.clone()),
        }),
        Reply::Relay(Forwarded {
            reply: Box::new(reply.clone()),
        }),
        Reply::Deliver(Forwarded {
            reply: Box::new(reply),
        }),
    ]
}

fn response(reply: Reply) -> ServerFrame {
    ServerFrame::Response {
        id: RequestId::new(1),
        outcome: Outcome::Ok(reply),
    }
}

fn settings() -> NodeSettings {
    NodeSettings {
        park_after: ParkAfter::Never,
        drivers: AgentDrivers {
            codex: AgentDriver::Direct,
            ..AgentDrivers::default()
        },
        restart_recovery: RestartRecovery {
            enabled: true,
            prompt: "private custom startup prompt".parse().unwrap(),
        },
        metadata: None,
    }
}
