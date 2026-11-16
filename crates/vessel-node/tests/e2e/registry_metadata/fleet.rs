//! Real fleet paths retain the authenticated origin and both intersected grants.
use vessel_client::{Connector as _, NodeClient};
use vessel_node::fleet::DEFAULT_GRANT;
use vessel_proto::registry::{
    GetRegistry, GetWorkspace, ListWorkspaces, PinWorkspace, ReleaseWorkspaceAutomation,
    RenameWorkspace, WorkspaceAutomation, WorkspaceDescriptor, WorkspacePin,
};
use vessel_proto::{ErrorCode, HarnessKind, Scope, WorkspaceId};

use super::seed;
use crate::across::{Member, home, member_granting, member_joined};
use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::fleet::fleet_config;
use crate::grants::device_with;
use crate::secure::{Relay, relayed_with};
use crate::support::{Fixture, client, step, within};
use futures_util::FutureExt as _;
use std::panic::{AssertUnwindSafe, resume_unwind};

async fn seeded_member(
    relay: &Relay,
    home: &Member,
    descriptor: &WorkspaceDescriptor,
    label: &str,
    grant: Vec<Scope>,
) -> Member {
    let fixture = Fixture::new();
    seed::seed(&fixture, std::slice::from_ref(descriptor)).await;
    let node = step(
        "registry seeded member relay registration",
        relayed_with(&fixture, &[&relay.url], |config| {
            config.name = Some(label.to_owned());
            config.fleet = fleet_config(HarnessKind::Codex, fixture.cwd().join("member-profile"));
            config.vessel_mcp = Some(vessel_mcp(&fixture));
        }),
    )
    .await;
    step(
        "registry seeded member fleet join",
        member_joined(home, fixture, node, label, vec![Scope::Sync], grant),
    )
    .await
}

async fn read(client: &NodeClient, member: &Member, workspace: WorkspaceId) {
    let snapshot = client
        .call_on(member.id(), GetRegistry::default())
        .await
        .unwrap();
    assert_eq!(snapshot.node, member.id());
    assert!(snapshot.workspaces.iter().any(|item| item.id == workspace));
    let same = step(
        "delegated registry reads do not retain a long-poll",
        client.call_on(
            member.id(),
            GetRegistry {
                changed_from: Some(snapshot.revision),
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(same, snapshot);
    let descriptor = client
        .call_on(member.id(), GetWorkspace { workspace })
        .await
        .unwrap();
    assert_eq!(descriptor.id, workspace);
}

async fn write(client: &NodeClient, member: &Member, workspace: WorkspaceId) {
    let result = client
        .call_on(
            member.id(),
            RenameWorkspace {
                workspace,
                title: Some("Fleet title".parse().unwrap()),
            },
        )
        .await
        .unwrap();
    assert_eq!(result.id, workspace);
    assert_eq!(result.title.unwrap().as_str(), "Fleet title");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn registry_metadata_forward_relay_deliver_keep_person_origin_and_intersected_scopes() {
    within(async {
        let relay = step("registry relay listen/start", Relay::start()).await;
        let sources = tempfile::tempdir().unwrap();
        let home = step(
            "registry home relay/fleet start",
            home(&relay, sources.path()),
        )
        .await;
        let mut descriptor = seed::descriptor(vessel_proto::ProjectId::random());
        descriptor.automation = WorkspaceAutomation::Blocked {
            source: seed::source(),
        };
        let target = seeded_member(
            &relay,
            &home,
            &descriptor,
            "registry-target",
            DEFAULT_GRANT.to_vec(),
        )
        .await;
        let readonly = seeded_member(
            &relay,
            &home,
            &descriptor,
            "registry-readonly",
            vec![Scope::Read],
        )
        .await;
        let relaying = step(
            "registry relaying member join",
            member_granting(
                &relay,
                &home,
                "registry-relaying",
                vec![Scope::Sync, Scope::Read, Scope::Operate, Scope::Relay],
                DEFAULT_GRANT.to_vec(),
            ),
        )
        .await;
        let result = AssertUnwindSafe(async {
            read(&home.owner, &target, descriptor.id).await;
            write(&home.owner, &target, descriptor.id).await;
            read(&relaying.owner, &target, descriptor.id).await;
            write(&relaying.owner, &target, descriptor.id).await;
            read(&home.owner, &readonly, descriptor.id).await;
            assert_eq!(
                code(
                    home.owner
                        .call_on(
                            readonly.id(),
                            RenameWorkspace {
                                workspace: descriptor.id,
                                title: None
                            }
                        )
                        .await
                ),
                ErrorCode::Forbidden
            );
            devices(&relay, &home, &target, descriptor.id).await;
            agent(&home, &target, descriptor.id).await;
            remaining(&home.owner, &target, descriptor.id).await;
        })
        .catch_unwind()
        .await;
        // Keep real node/relay handles outside assertions, and drain all of
        // them even if one close fails before rethrowing the original failure.
        let readonly_stop =
            AssertUnwindSafe(step("registry readonly stop/join", readonly.node.stop()))
                .catch_unwind();
        let relaying_stop =
            AssertUnwindSafe(step("registry relaying stop/join", relaying.node.stop()))
                .catch_unwind();
        let target_stop =
            AssertUnwindSafe(step("registry target stop/join", target.node.stop())).catch_unwind();
        let home_stop =
            AssertUnwindSafe(step("registry home stop/join", home.node.stop())).catch_unwind();
        let (readonly_stop, relaying_stop, target_stop, home_stop) =
            tokio::join!(readonly_stop, relaying_stop, target_stop, home_stop);
        step("registry relay stop/join", relay.stop()).await;
        if let Err(panic) = result {
            resume_unwind(panic);
        }
        for stopped in [readonly_stop, relaying_stop, target_stop, home_stop] {
            match stopped {
                Ok(stopped) => stopped.unwrap(),
                Err(panic) => resume_unwind(panic),
            }
        }
    })
    .await;
}

async fn devices(relay: &Relay, home: &Member, target: &Member, workspace: WorkspaceId) {
    let connector = step(
        "registry relayed device pair",
        device_with(relay, home, vec![Scope::Read, Scope::Operate]),
    )
    .await;
    let writer = step(
        "registry relayed device connect",
        connector.connect(client()),
    )
    .await
    .unwrap();
    read(&writer, target, workspace).await;
    write(&writer, target, workspace).await;
    // Keep one explicitly relayed device positive path. Remaining first hops
    // use the same real paired authority over direct transport, avoiding the
    // relay's unchanged four-client admission share while all devices are live.
    // Their delegated Forward target hop still uses the actual fleet/relay path.
    let reader = super::authority::paired(&home.owner, &home.node, vec![Scope::Read]).await;
    read(&reader, target, workspace).await;
    assert_eq!(
        code(
            reader
                .call_on(
                    target.id(),
                    RenameWorkspace {
                        workspace,
                        title: None
                    }
                )
                .await
        ),
        ErrorCode::Forbidden
    );
    let operator = super::authority::paired(&home.owner, &home.node, vec![Scope::Operate]).await;
    // The envelope independently needs Read; Operate does not imply it.
    assert_eq!(
        code(
            operator
                .call_on(
                    target.id(),
                    RenameWorkspace {
                        workspace,
                        title: None
                    }
                )
                .await
        ),
        ErrorCode::Forbidden
    );
    let admin = super::authority::paired(&home.owner, &home.node, vec![Scope::Admin]).await;
    assert_eq!(
        code(admin.call_on(target.id(), GetRegistry::default()).await),
        ErrorCode::Forbidden
    );
}

async fn agent(home: &Member, target: &Member, workspace: WorkspaceId) {
    let mut running = step(
        "registry fleet agent harness start",
        structured(&home.owner, &home.fixture),
    )
    .await;
    let (_, socket, auth) = step(
        "registry fleet agent credential",
        credential(&home.owner, &mut running),
    )
    .await;
    let agent = step("registry fleet agent connect", as_agent(&socket, &auth)).await;
    for id in [workspace, WorkspaceId::random()] {
        for inner in seed::requests(id) {
            use vessel_proto::Request as _;
            let forward = vessel_proto::Forward {
                node: target.id(),
                call: Box::new(inner),
            }
            .into_call();
            assert_eq!(
                code(
                    agent
                        .call_any(vessel_rpc::UnaryCall::try_from(forward).unwrap())
                        .await
                ),
                ErrorCode::Forbidden
            );
        }
    }
}

async fn remaining(client: &NodeClient, target: &Member, workspace: WorkspaceId) {
    let listed = client
        .call_on(target.id(), ListWorkspaces::default())
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    let pinned = client
        .call_on(
            target.id(),
            PinWorkspace {
                workspace,
                pin: WorkspacePin::Pinned,
            },
        )
        .await
        .unwrap();
    assert_eq!(pinned.pin, WorkspacePin::Pinned);
    let released = client
        .call_on(
            target.id(),
            ReleaseWorkspaceAutomation {
                workspace,
                expected: pinned.revision,
                source: seed::source(),
            },
        )
        .await
        .unwrap();
    assert_eq!(released.automation, WorkspaceAutomation::Allowed);
    assert_eq!(released.id, workspace);
}
