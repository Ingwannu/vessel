//! In-memory admission proves no staged service frame or effect is issued.

use super::*;
use futures_util::{SinkExt as _, StreamExt as _};
use vessel_proto::workspace_services::*;
use vessel_proto::{
    Call, Method, Origin, OriginBy, Request as _, ServerFrame, Welcome, WorkspaceId,
};

fn target() -> WorkspaceScriptRef {
    WorkspaceScriptRef {
        workspace: WorkspaceId::random(),
        script: "synthetic-service".parse().unwrap(),
    }
}

fn start() -> StartWorkspaceScript {
    StartWorkspaceScript {
        target: target(),
        snapshot: ServiceSnapshotId::random(),
        definition: vessel_proto::workspace::CommandsDigest::from_sha256(Default::default()),
    }
}

fn unassigned(error: crate::CallError, expected: Method) {
    assert!(matches!(error, crate::CallError::UnassignedProtocol { method } if method == expected));
}

async fn direct_reads(client: &NodeClient) -> Vec<Call> {
    let request = start();
    let query = WorkspaceScriptQuery {
        workspace: request.target.workspace,
        query: ServiceQuery::Initial,
    };
    let env = GetWorkspaceServiceEnv {
        target: request.target,
        snapshot: request.snapshot,
    };
    unassigned(
        client.workspace_scripts(query.clone()).await.unwrap_err(),
        Method::ListWorkspaceScripts,
    );
    unassigned(
        client.workspace_service_env(env.clone()).await.unwrap_err(),
        Method::GetWorkspaceServiceEnv,
    );
    vec![query.into_call(), env.into_call()]
}

async fn direct_operations(client: &NodeClient) -> Vec<Call> {
    let request = start();
    let stop = StopWorkspaceScript {
        target: request.target.clone(),
        snapshot: request.snapshot,
    };
    let restart = RestartWorkspaceScript {
        target: request.target.clone(),
        snapshot: request.snapshot,
        definition: request.definition.clone(),
    };
    unassigned(
        client
            .start_workspace_script(request.clone())
            .await
            .unwrap_err(),
        Method::StartWorkspaceScript,
    );
    unassigned(
        client
            .stop_workspace_script(stop.clone())
            .await
            .unwrap_err(),
        Method::StopWorkspaceScript,
    );
    unassigned(
        client
            .restart_workspace_script(restart.clone())
            .await
            .unwrap_err(),
        Method::RestartWorkspaceScript,
    );
    vec![request.into_call(), stop.into_call(), restart.into_call()]
}

async fn direct_admin(client: &NodeClient) -> Vec<Call> {
    let gateway = SetServiceGateway {
        expected: None,
        gateway: ServiceGateway::LocalOnly,
    };
    let exposure = SetServiceExposure {
        target: target(),
        expected: None,
        change: ServiceExposureChange::Private,
    };
    unassigned(
        client.service_settings().await.unwrap_err(),
        Method::GetServiceSettings,
    );
    unassigned(
        client
            .set_service_gateway(gateway.clone())
            .await
            .unwrap_err(),
        Method::SetServiceGateway,
    );
    unassigned(
        client
            .set_service_exposure(exposure.clone())
            .await
            .unwrap_err(),
        Method::SetServiceExposure,
    );
    vec![
        GetServiceSettings {}.into_call(),
        gateway.into_call(),
        exposure.into_call(),
    ]
}

async fn forged_envelopes(client: &NodeClient, id: NodeId, call: &Call) {
    for by in [
        OriginBy::Owner,
        OriginBy::Device {
            id: vessel_proto::DeviceId::random(),
        },
        OriginBy::Agent {
            session: SessionId::random(),
            depth: 1,
        },
    ] {
        let origin = Origin {
            node: NodeId::random(),
            by,
            scopes: vessel_proto::Scope::ALL.to_vec(),
        };
        for envelope in [
            vessel_proto::Forward {
                node: id,
                call: Box::new(call.clone()),
            }
            .into_call(),
            vessel_proto::RelayCall {
                node: id,
                origin: origin.clone(),
                call: Box::new(call.clone()),
            }
            .into_call(),
            vessel_proto::Deliver {
                origin,
                call: Box::new(call.clone()),
            }
            .into_call(),
        ] {
            let method = envelope.method();
            let unary = crate::UnaryCall::try_from(envelope).unwrap();
            unassigned(client.call_any(unary).await.unwrap_err(), method);
        }
    }
}

async fn no_frame_at(protocol: vessel_proto::ProtocolVersion) {
    let (client_end, mut node_end) = vessel_rpc::memory::pair(NonZeroUsize::MIN);
    let node_info = node();
    let id = node_info.id;
    let peer = tokio::spawn(async move {
        let _hello = node_end.next().await.unwrap().unwrap();
        node_end
            .send(
                serde_json::to_string(&ServerFrame::Welcome(Welcome {
                    protocol,
                    node: node_info,
                }))
                .unwrap(),
            )
            .await
            .unwrap();
        node_end.next().await
    });
    let client = NodeClient::over(client_end, client_info(), &ClientConfig::default())
        .await
        .unwrap();
    let mut calls = direct_reads(&client).await;
    calls.extend(direct_operations(&client).await);
    calls.extend(direct_admin(&client).await);
    for call in calls {
        forged_envelopes(&client, id, &call).await;
    }
    drop(client);
    assert!(
        peer.await.unwrap().is_none(),
        "only the handshake reached the peer"
    );
}

#[tokio::test]
async fn service_wrappers_and_forged_envelopes_never_send_a_staged_frame() {
    let budget = ServerConfig::default().handshake_timeout;
    vessel_testkit::within(budget, async {
        for protocol in [
            vessel_proto::MIN_PROTOCOL_VERSION,
            vessel_proto::PROTOCOL_VERSION,
        ] {
            no_frame_at(protocol).await;
        }
    })
    .await
    .unwrap();
}

fn unsupported_service<T>(result: Result<T, WireError>) {
    let Err(error) = result else {
        panic!("a staged service was implemented")
    };
    assert_eq!(error.code, ErrorCode::Unsupported);
    assert!(!error.message.contains("synthetic-service"));
}

async fn default_reads(fake: &FakeNode, context: &Context) {
    let request = start();
    unsupported_service(
        fake.workspace_scripts(
            context,
            WorkspaceScriptQuery {
                workspace: request.target.workspace,
                query: ServiceQuery::Initial,
            },
        )
        .await,
    );
    unsupported_service(
        fake.workspace_service_env(
            context,
            GetWorkspaceServiceEnv {
                target: request.target,
                snapshot: request.snapshot,
            },
        )
        .await,
    );
}

async fn default_operations(fake: &FakeNode, context: &Context) {
    let request = start();
    unsupported_service(fake.start_workspace_script(context, request.clone()).await);
    unsupported_service(
        fake.stop_workspace_script(
            context,
            StopWorkspaceScript {
                target: request.target.clone(),
                snapshot: request.snapshot,
            },
        )
        .await,
    );
    unsupported_service(
        fake.restart_workspace_script(
            context,
            RestartWorkspaceScript {
                target: request.target,
                snapshot: request.snapshot,
                definition: request.definition,
            },
        )
        .await,
    );
}

async fn default_admin(fake: &FakeNode, context: &Context) {
    unsupported_service(fake.service_settings(context, GetServiceSettings {}).await);
    unsupported_service(
        fake.set_service_gateway(
            context,
            SetServiceGateway {
                expected: None,
                gateway: ServiceGateway::LocalOnly,
            },
        )
        .await,
    );
    unsupported_service(
        fake.set_service_exposure(
            context,
            SetServiceExposure {
                target: target(),
                expected: None,
                change: ServiceExposureChange::Private,
            },
        )
        .await,
    );
}

#[tokio::test]
async fn real_staged_handler_defaults_refuse_without_a_service_implementation() {
    let budget = ServerConfig::default().handshake_timeout;
    vessel_testkit::within(budget, async {
        let fake = Arc::new(FakeNode::new(0));
        let server = server(Arc::clone(&fake));
        let (client_end, server_end) = vessel_rpc::memory::pair(NonZeroUsize::MIN);
        let (send_context, receive_context) = tokio::sync::oneshot::channel();
        let serving = tokio::spawn(async move {
            server
                .serve(server_end, |peer| {
                    let context = Context {
                        outbound: peer.outbound,
                    };
                    send_context
                        .send(Context {
                            outbound: context.outbound.clone(),
                        })
                        .unwrap_or_else(|_closed| panic!("fixture receiver closed"));
                    context
                })
                .await
        });
        let client = NodeClient::over(client_end, client_info(), &ClientConfig::default())
            .await
            .unwrap();
        let context = receive_context.await.unwrap();
        default_reads(&fake, &context).await;
        default_operations(&fake, &context).await;
        default_admin(&fake, &context).await;
        assert_eq!(fake.reads.load(Ordering::Acquire), 0);
        assert_eq!(fake.kills.load(Ordering::Acquire), 0);
        assert_eq!(*fake.stopped.lock().unwrap(), None);
        drop(context);
        drop(client);
        serving.await.unwrap().unwrap();
    })
    .await
    .unwrap();
}
