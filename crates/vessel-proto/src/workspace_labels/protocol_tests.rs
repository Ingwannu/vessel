//! Protocol 28 call admission and direct/nested registry field omission.

use super::*;
use crate::{
    Call, Deliver, Forward, Forwarded, Method, NodeId, Origin, OriginBy, RelayCall, Reply,
    Request as _, Scope, WorkspaceId,
    registry::{RegistryRevision, RegistrySnapshot},
};

#[test]
fn workspace_labels_calls_and_every_envelope_keep_the_shared_capability() {
    for (method, expected_scope) in [
        (Method::ListWorkspaceLabels, Scope::Read),
        (Method::AssignWorkspaceLabel, Scope::Operate),
        (Method::UpdateWorkspaceLabel, Scope::Operate),
        (Method::InspectWorkspaceLabelDelete, Scope::Read),
        (Method::DeleteWorkspaceLabel, Scope::Operate),
    ] {
        let call = crate::rpc::samples::call(method).unwrap();
        assert_eq!(method.since(), WORKSPACE_LABELS);
        assert_eq!(call.since(), WORKSPACE_LABELS);
        assert_eq!(method.scope(), Some(expected_scope));
        assert!(method.forwardable());
        let origin = Origin {
            node: NodeId::random(),
            by: OriginBy::Owner,
            scopes: vec![Scope::Read, Scope::Operate],
        };
        let envelopes: [Call; 3] = [
            Forward {
                node: NodeId::random(),
                call: Box::new(call.clone()),
            }
            .into_call(),
            RelayCall {
                node: NodeId::random(),
                call: Box::new(call.clone()),
                origin: origin.clone(),
            }
            .into_call(),
            Deliver {
                call: Box::new(call),
                origin,
            }
            .into_call(),
        ];
        for envelope in envelopes {
            assert_eq!(envelope.since(), WORKSPACE_LABELS);
        }
    }
}

fn snapshot() -> RegistrySnapshot {
    RegistrySnapshot {
        labels: vec![WorkspaceLabelAssignments {
            workspace: WorkspaceId::random(),
            labels: vec!["Ready".parse().unwrap()],
        }],
        node: NodeId::random(),
        revision: RegistryRevision::ZERO,
        workspaces: Vec::new(),
        memberships: Vec::new(),
    }
}

#[test]
fn workspace_labels_registry_projection_is_omitted_at_direct_and_nested_hops() {
    let original = snapshot();
    for protocol in [crate::MIN_PROTOCOL_VERSION, crate::PROTOCOL_VERSION] {
        let hidden = WORKSPACE_LABELS.is_none_or(|since| protocol < since);
        let projected = original.for_protocol(protocol);
        assert_eq!(projected.is_some(), hidden);
        let projected = projected.unwrap_or_else(|| original.clone());
        assert_eq!(projected.node, original.node);
        assert_eq!(projected.revision, original.revision);
        assert_eq!(projected.labels.is_empty(), hidden);
        assert_eq!(
            serde_json::to_string(&projected)
                .unwrap()
                .contains("\"labels\""),
            !hidden
        );
        for reply in [
            Reply::GetRegistry(original.clone()),
            Reply::Forward(Forwarded {
                reply: Box::new(Reply::GetRegistry(original.clone())),
            }),
            Reply::Relay(Forwarded {
                reply: Box::new(Reply::GetRegistry(original.clone())),
            }),
            Reply::Deliver(Forwarded {
                reply: Box::new(Reply::Forward(Forwarded {
                    reply: Box::new(Reply::GetRegistry(original.clone())),
                })),
            }),
        ] {
            let encoded =
                serde_json::to_string(&reply.for_protocol(protocol).unwrap_or(reply)).unwrap();
            assert_eq!(encoded.contains("\"labels\""), !hidden);
            assert_eq!(encoded.contains("Ready"), !hidden);
        }
    }
    assert_eq!(original.labels.len(), 1);
}

#[test]
fn workspace_labels_old_snapshot_without_projection_stays_decodable() {
    let mut empty = snapshot();
    empty.labels.clear();
    let encoded = serde_json::to_string(&empty).unwrap();
    assert!(!encoded.contains("\"labels\""));
    assert_eq!(
        serde_json::from_str::<RegistrySnapshot>(&encoded).unwrap(),
        empty
    );
    assert!(empty.for_protocol(crate::PROTOCOL_VERSION).is_none());
}
