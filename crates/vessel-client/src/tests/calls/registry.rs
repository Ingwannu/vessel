//! Registry metadata admission at an old hop, with a supported negative control.

use super::super::*;
use futures_util::{SinkExt as _, StreamExt as _};
use vessel_proto::registry::{
    BlockedSource, GetRegistry, GetWorkspace, ListWorkspaces, PinWorkspace, RegistryRevision,
    ReleaseWorkspaceAutomation, RenameWorkspace, WorkspacePin,
};
use vessel_proto::{
    Call, ClientFrame, Deliver, Forward, Method, Origin, OriginBy, Outcome, ProtocolVersion,
    RelayCall, Reply, Request as _, Scope, ServerFrame, Welcome, WorkspaceId,
};
use vessel_rpc::{CallError, UnaryCall};

const MEMORY_CAPACITY: usize = 4;
const CASE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

fn release(workspace: WorkspaceId) -> ReleaseWorkspaceAutomation {
    ReleaseWorkspaceAutomation {
        workspace,
        expected: RegistryRevision::ZERO,
        source: BlockedSource::ChangeRequest {
            number: vessel_proto::forge::PullNumber::new(1).unwrap(),
            head_repository: vessel_proto::forge::RepoRef {
                kind: vessel_proto::forge::ForgeKind::Github,
                host: "github.com".parse().unwrap(),
                path: "fork/project".parse().unwrap(),
            },
        },
    }
}

fn requests(workspace: WorkspaceId) -> [Call; 6] {
    [
        GetRegistry::default().into_call(),
        ListWorkspaces::default().into_call(),
        GetWorkspace { workspace }.into_call(),
        RenameWorkspace {
            workspace,
            title: None,
        }
        .into_call(),
        PinWorkspace {
            workspace,
            pin: WorkspacePin::Unpinned,
        }
        .into_call(),
        release(workspace).into_call(),
    ]
}

fn requires(
    error: &CallError,
    method: Method,
    needs: ProtocolVersion,
    negotiated: ProtocolVersion,
) {
    assert!(
        matches!(error, CallError::NotInProtocol { method: actual, needs: required, negotiated: peer } if *actual == method && *required == needs && *peer == negotiated)
    );
}

async fn direct_refusals(
    client: &NodeClient,
    workspace: WorkspaceId,
    needed: ProtocolVersion,
    version: ProtocolVersion,
) {
    let direct = [
        (
            Method::GetRegistry,
            client.call(GetRegistry::default()).await.map(|_| ()),
        ),
        (
            Method::ListWorkspaces,
            client.call(ListWorkspaces::default()).await.map(|_| ()),
        ),
        (
            Method::GetWorkspace,
            client.call(GetWorkspace { workspace }).await.map(|_| ()),
        ),
        (
            Method::RenameWorkspace,
            client
                .call(RenameWorkspace {
                    workspace,
                    title: None,
                })
                .await
                .map(|_| ()),
        ),
        (
            Method::PinWorkspace,
            client
                .call(PinWorkspace {
                    workspace,
                    pin: WorkspacePin::Unpinned,
                })
                .await
                .map(|_| ()),
        ),
        (
            Method::ReleaseWorkspaceAutomation,
            client.call(release(workspace)).await.map(|_| ()),
        ),
    ];
    for (method, result) in direct {
        requires(&result.unwrap_err(), method, needed, version);
    }
}

#[tokio::test]
async fn registry_metadata_direct_and_every_envelope_send_no_frame_to_an_old_hop() {
    let needed = vessel_proto::registry::PROJECT_WORKSPACE_REGISTRY.unwrap();
    let previous = ProtocolVersion::new(needed.get().checked_sub(1).unwrap());
    let mut versions = vec![vessel_proto::MIN_PROTOCOL_VERSION, previous];
    versions.sort();
    versions.dedup();
    for version in versions {
        tokio::time::timeout(CASE_DEADLINE, scenario(version, needed))
            .await
            .unwrap();
    }
}

async fn scenario(version: ProtocolVersion, needed: ProtocolVersion) {
    let (client_end, node_end) =
        vessel_rpc::memory::pair(NonZeroUsize::new(MEMORY_CAPACITY).unwrap());
    let welcomed = node();
    let expected_node = welcomed.id;
    let peer = tokio::spawn(serve_old(node_end, version, welcomed));
    let client = NodeClient::over(client_end, client_info(), &ClientConfig::default())
        .await
        .unwrap();
    assert_eq!(client.protocol(), version);
    let workspace = WorkspaceId::random();
    direct_refusals(&client, workspace, needed, version).await;
    requires(
        &client
            .call_on(expected_node, GetRegistry::default())
            .await
            .unwrap_err(),
        Method::Forward,
        needed,
        version,
    );
    let origin = Origin {
        node: expected_node,
        by: OriginBy::Owner,
        scopes: Scope::ALL.to_vec(),
    };
    for inner in requests(workspace) {
        for call in [
            Forward {
                node: expected_node,
                call: Box::new(inner.clone()),
            }
            .into_call(),
            RelayCall {
                node: expected_node,
                call: Box::new(inner.clone()),
                origin: origin.clone(),
            }
            .into_call(),
            Deliver {
                call: Box::new(inner),
                origin: origin.clone(),
            }
            .into_call(),
        ] {
            let method = call.method();
            let error = client
                .call_any(UnaryCall::try_from(call).unwrap())
                .await
                .unwrap_err();
            requires(&error, method, needed, version);
        }
    }
    assert_eq!(client.node_info().await.unwrap().id, expected_node);
    drop(client);
    assert!(
        peer.await.unwrap().is_none(),
        "no registry frame reached the old peer"
    );
}

async fn serve_old(
    mut node_end: vessel_rpc::memory::MemoryTransport,
    version: ProtocolVersion,
    welcomed: NodeInfo,
) -> Option<Result<String, Disconnected>> {
    let _hello = node_end.next().await.unwrap().unwrap();
    node_end
        .send(
            serde_json::to_string(&ServerFrame::Welcome(Welcome {
                protocol: version,
                node: welcomed.clone(),
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    // The first actual request must be the supported negative control.
    let text = node_end.next().await.unwrap().unwrap();
    let ClientFrame::Request { id, call } = serde_json::from_str(&text).unwrap() else {
        panic!("expected a supported request");
    };
    assert_eq!(call.method(), Method::NodeInfo);
    node_end
        .send(
            serde_json::to_string(&ServerFrame::Response {
                id,
                outcome: Outcome::Ok(Reply::NodeInfo(welcomed)),
            })
            .unwrap(),
        )
        .await
        .unwrap();
    node_end.next().await
}
