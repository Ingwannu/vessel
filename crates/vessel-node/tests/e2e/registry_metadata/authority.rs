//! Real paired devices and authenticated fake-agent credentials exercise person policy.
use vessel_client::secure::{RouteTimeouts, pair_over};
use vessel_client::{Connector as _, NodeClient};
use vessel_proto::registry::{GetWorkspace, WorkspaceAutomation};
use vessel_proto::{ErrorCode, ProjectId, Scope, WorkspaceId};
use vessel_rpc::UnaryCall;

use super::{lifetime, seed};
use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::secure::{advertise_placeholder_route, device_via, dialer, direct_route};
use crate::support::{Fixture, client, step};
use std::sync::Arc;

pub(super) async fn paired(
    owner: &NodeClient,
    node: &vessel_node::Node,
    scopes: Vec<Scope>,
) -> NodeClient {
    const INVITE_SECONDS: u32 = 600;
    let mut offer = owner
        .invite_with_scopes(None, INVITE_SECONDS, scopes)
        .await
        .unwrap()
        .link
        .decode()
        .unwrap();
    let route = direct_route(node);
    offer.direct = vec![route.clone()];
    let (paired, _route) = step(
        "registry device pair handshake",
        pair_over(
            &dialer(),
            &offer,
            "registry-test-device",
            RouteTimeouts::default(),
        ),
    )
    .await
    .unwrap();
    step(
        "registry paired device connect",
        device_via(dialer(), &paired, &[&route], &[]).connect(client()),
    )
    .await
    .unwrap()
}

async fn refused(client: &NodeClient, workspace: WorkspaceId) {
    for request in seed::requests(workspace) {
        assert_eq!(
            code(client.call_any(UnaryCall::try_from(request).unwrap()).await),
            ErrorCode::Forbidden
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registry_metadata_paired_scopes_do_not_imply_each_other() {
    lifetime::run(lifetime::Case::Paired).await;
}

pub(super) async fn paired_case(resources: &mut lifetime::Resources) {
    let fixture = Arc::new(Fixture::new());
    let mut descriptor = seed::descriptor(ProjectId::random());
    descriptor.automation = WorkspaceAutomation::Blocked {
        source: seed::source(),
    };
    resources
        .seed(Arc::clone(&fixture), vec![descriptor.clone()])
        .await;
    let node = resources
        .node(Arc::clone(&fixture), advertise_placeholder_route)
        .await;
    let owner = step("registry paired owner connect", fixture.connect(&node)).await;
    let reader = paired(&owner, &node, vec![Scope::Read]).await;
    let operator = paired(&owner, &node, vec![Scope::Operate]).await;
    let admin = paired(&owner, &node, vec![Scope::Admin]).await;
    for request in seed::requests(descriptor.id) {
        let read = request.method().scope() == Some(Scope::Read);
        let result = reader.call_any(UnaryCall::try_from(request).unwrap()).await;
        if read {
            assert!(result.is_ok());
        } else {
            assert_eq!(code(result), ErrorCode::Forbidden);
        }
    }
    for mut request in seed::requests(descriptor.id) {
        let read = request.method().scope() == Some(Scope::Read);
        if let vessel_proto::Call::ReleaseWorkspaceAutomation(release) = &mut request {
            release.expected = owner
                .call(GetWorkspace {
                    workspace: descriptor.id,
                })
                .await
                .unwrap()
                .revision;
        }
        let result = operator
            .call_any(UnaryCall::try_from(request).unwrap())
            .await;
        if read {
            assert_eq!(code(result), ErrorCode::Forbidden);
        } else {
            assert!(result.is_ok());
        }
    }
    refused(&admin, descriptor.id).await;
    refused(&admin, WorkspaceId::random()).await;
    let saved = owner
        .call(GetWorkspace {
            workspace: descriptor.id,
        })
        .await
        .unwrap();
    assert_eq!(saved.automation, WorkspaceAutomation::Allowed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registry_metadata_real_agent_is_refused_for_existing_and_absent_identity() {
    lifetime::run(lifetime::Case::Agent).await;
}

pub(super) async fn agent_case(resources: &mut lifetime::Resources) {
    let fixture = Arc::new(Fixture::new());
    let descriptor = seed::descriptor(ProjectId::random());
    resources
        .seed(Arc::clone(&fixture), vec![descriptor.clone()])
        .await;
    let config_fixture = Arc::clone(&fixture);
    let node = resources
        .node(Arc::clone(&fixture), move |config| {
            config.vessel_mcp = Some(vessel_mcp(&config_fixture));
        })
        .await;
    let owner = step("registry agent owner connect", fixture.connect(&node)).await;
    let mut running = step("registry agent harness start", structured(&owner, &fixture)).await;
    let (_, socket, auth) = step(
        "registry agent credential",
        credential(&owner, &mut running),
    )
    .await;
    let agent = step("registry agent connect", as_agent(&socket, &auth)).await;
    refused(&agent, descriptor.id).await;
    refused(&agent, WorkspaceId::random()).await;
    assert_eq!(
        owner
            .call(GetWorkspace {
                workspace: descriptor.id
            })
            .await
            .unwrap(),
        descriptor
    );
}
