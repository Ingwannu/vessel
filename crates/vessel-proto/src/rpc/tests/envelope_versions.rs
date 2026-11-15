//! Every typed decoder along a fleet route must understand the full call.
use super::*;
use crate::{Deliver, Forward, NodeId, Origin, OriginBy, RelayCall};

fn envelopes(call: Call) -> [Call; 3] {
    let node = NodeId::random();
    let origin = Origin {
        node,
        by: OriginBy::Owner,
        scopes: Vec::new(),
    };
    [
        Forward {
            node,
            call: Box::new(call.clone()),
        }
        .into_call(),
        RelayCall {
            node,
            call: Box::new(call.clone()),
            origin: origin.clone(),
        }
        .into_call(),
        Deliver {
            call: Box::new(call),
            origin,
        }
        .into_call(),
    ]
}

#[test]
fn every_envelope_recursively_requires_the_inner_method_or_field_version() {
    use crate::inbox::ListInbox;
    use crate::settings::{AgentDriver, SetAgentDrivers, SetNodeSettings};
    for (inner, needed) in [
        (GetNode.into_call(), V6),
        (ListInbox::default().into_call(), ProtocolVersion::new(22)),
        (
            SetNodeSettings {
                park_after: None,
                drivers: Some(SetAgentDrivers {
                    codex: Some(AgentDriver::Acp),
                    ..SetAgentDrivers::default()
                }),
            }
            .into_call(),
            ProtocolVersion::new(18),
        ),
    ] {
        for outer in envelopes(inner) {
            assert_eq!(outer.since(), Some(needed));
            // Nested envelopes are rejected by forwarding policy, but their
            // version must still be checked before any handler sees them.
            for nested in envelopes(outer) {
                assert_eq!(nested.since(), Some(needed));
            }
        }
    }
}
