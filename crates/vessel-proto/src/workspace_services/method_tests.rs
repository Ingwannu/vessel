//! Staged table rows refuse every envelope before an effect can be admitted.

use crate::{
    Call, DeviceId, Method, MethodShape, NodeId, Origin, OriginBy, Reply, Request as _, Scope,
    SessionId,
};

use super::*;

#[test]
fn all_service_rows_are_typed_unary_nonforwardable_and_unassigned() {
    for (method, scope) in [
        (Method::ListWorkspaceScripts, Scope::Read),
        (Method::StartWorkspaceScript, Scope::Operate),
        (Method::StopWorkspaceScript, Scope::Operate),
        (Method::RestartWorkspaceScript, Scope::Operate),
        (Method::GetWorkspaceServiceEnv, Scope::Read),
        (Method::GetServiceSettings, Scope::Admin),
        (Method::SetServiceGateway, Scope::Admin),
        (Method::SetServiceExposure, Scope::Admin),
    ] {
        assert!(method.is_staged());
        assert!(!method.forwardable());
        assert_eq!(method.scope(), Some(scope));
        assert_eq!(method.shape(), MethodShape::Unary);
        assert_eq!(method.since(), WORKSPACE_SERVICES);
        let call = crate::rpc::samples::call(method).unwrap();
        assert_eq!(call.method(), method);
        assert_eq!(call.required_protocol(), None);
        for by in [
            OriginBy::Owner,
            OriginBy::Device {
                id: DeviceId::random(),
            },
            OriginBy::Agent {
                session: SessionId::random(),
                depth: 1,
            },
        ] {
            let origin = Origin {
                node: NodeId::random(),
                by,
                scopes: Scope::ALL.to_vec(),
            };
            for envelope in [
                crate::Forward {
                    node: NodeId::random(),
                    call: Box::new(call.clone()),
                }
                .into_call(),
                crate::RelayCall {
                    node: NodeId::random(),
                    origin: origin.clone(),
                    call: Box::new(call.clone()),
                }
                .into_call(),
                crate::Deliver {
                    origin,
                    call: Box::new(call.clone()),
                }
                .into_call(),
            ] {
                assert_eq!(envelope.required_protocol(), None);
                let json = serde_json::to_string(&envelope).unwrap();
                assert_eq!(serde_json::from_str::<Call>(&json).unwrap(), envelope);
            }
        }
    }
}

fn hidden_commands() -> crate::workspace::ProjectCommands {
    crate::workspace::ProjectCommands::Listed {
        setup: vec![],
        teardown: vec![],
        digest: crate::workspace::CommandsDigest::from_sha256(Default::default()),
        approved: false,
        scripts: vec![],
        terminals: vec![],
        service_ports: Some(
            crate::workspace::ServicePorts::new(Some("9001-9002".parse().unwrap()), None).unwrap(),
        ),
        last_approved: None,
    }
}

#[test]
fn every_new_approval_reply_and_nested_envelope_hides_unassigned_digest() {
    let workspace = WorkspaceId::random();
    let commands = hidden_commands();
    let outcome = WorkspaceScriptOutcome::NeedsApproval {
        workspace,
        commands,
    };
    let configuration = ServiceConfiguration::Current {
        digest: crate::workspace::CommandsDigest::from_sha256(Default::default()),
        approval: ServiceApproval::Approved,
    };
    let page = ServicePage::new(
        workspace,
        ServiceSnapshotId::random(),
        configuration,
        vec![],
        None,
    )
    .unwrap();
    let replies = [
        Reply::ListWorkspaceScripts(WorkspaceScriptQueryResult::Snapshot { page }),
        Reply::StartWorkspaceScript(outcome.clone()),
        Reply::StopWorkspaceScript(outcome.clone()),
        Reply::RestartWorkspaceScript(outcome),
    ];
    for reply in replies {
        for reply in [
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
        ] {
            for protocol in [crate::MIN_PROTOCOL_VERSION, crate::PROTOCOL_VERSION] {
                let adapted = reply.for_protocol(protocol).unwrap();
                let json = serde_json::to_string(&adapted).unwrap();
                assert!(!json.contains(
                    crate::workspace::CommandsDigest::from_sha256(Default::default()).as_str()
                ));
                assert!(!json.contains("9001-9002"));
                assert!(!json.contains("\"approved\""));
            }
        }
    }
}
