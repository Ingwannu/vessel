//! Assigned metadata methods on a real node: exact identities, filtering and CAS.
//! Offline seeds do not provision directories or attest physical resource state.
mod authority;
mod fleet;
mod seed;

use chrono::DateTime;
use vessel_client::NodeClient;
use vessel_proto::registry::{
    GetRegistry, GetWorkspace, ListWorkspaces, PinWorkspace, ReleaseWorkspaceAutomation,
    RenameWorkspace, WorkspaceAutomation, WorkspaceDescriptor, WorkspaceLifecycle, WorkspaceList,
    WorkspacePin, WorkspaceVisibility, WorkspaceVisibilityFilter,
};
use vessel_proto::{ErrorCode, ProjectId, WorkspaceId};

use crate::agents::code;
use crate::support::{Fixture, within};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registry_metadata_owner_round_trips_all_six_methods_and_exact_refusals() {
    within(async {
        let fixture = Fixture::new();
        let project = ProjectId::random();
        let first = seed::descriptor(project);
        let mut blocked = seed::descriptor(project);
        blocked.visibility = WorkspaceVisibility::Background;
        blocked.automation = WorkspaceAutomation::Blocked {
            source: seed::source(),
        };
        let mut archived = seed::descriptor(ProjectId::random());
        archived.lifecycle = WorkspaceLifecycle::Archived {
            at: DateTime::UNIX_EPOCH,
        };
        seed::seed(
            &fixture,
            &[first.clone(), blocked.clone(), archived.clone()],
        )
        .await;
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        assert_eq!(owner.protocol(), vessel_proto::PROTOCOL_VERSION);
        reads(&owner, &[first.clone(), blocked.clone(), archived.clone()]).await;
        presentation(&owner, &first, &blocked, &archived).await;
        release(&owner, &blocked, &archived).await;
        node.stop().await.unwrap();
    })
    .await;
}

async fn reads(client: &NodeClient, expected: &[WorkspaceDescriptor]) {
    let snapshot = client.call(GetRegistry::default()).await.unwrap();
    assert_eq!(snapshot.node, client.node().id);
    assert_eq!(
        client
            .call_on(client.node().id, GetRegistry::default())
            .await
            .unwrap(),
        snapshot
    );
    assert_eq!(snapshot.workspaces.len(), expected.len());
    assert!(snapshot.memberships.is_empty());
    for descriptor in expected {
        assert!(snapshot.workspaces.contains(descriptor));
        assert_eq!(
            client
                .call(GetWorkspace {
                    workspace: descriptor.id
                })
                .await
                .unwrap(),
            *descriptor
        );
    }
    let visible = client.call(ListWorkspaces::default()).await.unwrap();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible.first(), expected.first());
    let all = client
        .call(ListWorkspaces {
            project: None,
            lifecycle: WorkspaceList::All,
            visibility: WorkspaceVisibilityFilter::All,
        })
        .await
        .unwrap();
    assert_eq!(all.len(), expected.len());
    let unrelated = client
        .call(ListWorkspaces {
            project: Some(ProjectId::random()),
            lifecycle: WorkspaceList::All,
            visibility: WorkspaceVisibilityFilter::All,
        })
        .await
        .unwrap();
    assert!(unrelated.is_empty());
    assert_eq!(
        code(
            client
                .call(GetWorkspace {
                    workspace: WorkspaceId::random()
                })
                .await
        ),
        ErrorCode::NotFound
    );
}

async fn presentation(
    client: &NodeClient,
    first: &WorkspaceDescriptor,
    other: &WorkspaceDescriptor,
    archived: &WorkspaceDescriptor,
) {
    let before = client.call(GetRegistry::default()).await.unwrap();
    let renamed = client
        .call(RenameWorkspace {
            workspace: first.id,
            title: Some("Manual override".parse().unwrap()),
        })
        .await
        .unwrap();
    assert_eq!(renamed.id, first.id);
    assert_eq!(renamed.title.as_ref().unwrap().as_str(), "Manual override");
    let changed = client
        .call(GetRegistry {
            changed_from: Some(before.revision),
        })
        .await
        .unwrap();
    assert!(changed.revision > before.revision);
    let reset = client
        .call(RenameWorkspace {
            workspace: first.id,
            title: None,
        })
        .await
        .unwrap();
    assert_eq!(reset.title, None);
    for pin in [WorkspacePin::Pinned, WorkspacePin::Unpinned] {
        let saved = client
            .call(PinWorkspace {
                workspace: first.id,
                pin,
            })
            .await
            .unwrap();
        assert_eq!(saved.pin, pin);
    }
    assert_eq!(
        client
            .call(GetWorkspace {
                workspace: other.id
            })
            .await
            .unwrap(),
        *other
    );
    assert_eq!(
        code(
            client
                .call(RenameWorkspace {
                    workspace: archived.id,
                    title: None
                })
                .await
        ),
        ErrorCode::Conflict
    );
    assert_eq!(
        code(
            client
                .call(PinWorkspace {
                    workspace: archived.id,
                    pin: WorkspacePin::Pinned
                })
                .await
        ),
        ErrorCode::Conflict
    );
    assert_eq!(
        code(
            client
                .call(RenameWorkspace {
                    workspace: WorkspaceId::random(),
                    title: None
                })
                .await
        ),
        ErrorCode::NotFound
    );
}

async fn release(
    client: &NodeClient,
    blocked: &WorkspaceDescriptor,
    archived: &WorkspaceDescriptor,
) {
    let exact = ReleaseWorkspaceAutomation {
        workspace: blocked.id,
        expected: blocked.revision,
        source: seed::source(),
    };
    let stale = ReleaseWorkspaceAutomation {
        expected: exact.expected.next().unwrap(),
        ..exact.clone()
    };
    assert_eq!(code(client.call(stale).await), ErrorCode::Conflict);
    let mut wrong_source = exact.clone();
    let vessel_proto::registry::BlockedSource::ChangeRequest { number, .. } =
        &mut wrong_source.source;
    *number = vessel_proto::forge::PullNumber::new(2).unwrap();
    assert_eq!(code(client.call(wrong_source).await), ErrorCode::Conflict);
    assert_eq!(
        client
            .call(GetWorkspace {
                workspace: blocked.id
            })
            .await
            .unwrap(),
        *blocked
    );
    let saved = client.call(exact.clone()).await.unwrap();
    assert_eq!(saved.id, blocked.id);
    assert_eq!(saved.automation, WorkspaceAutomation::Allowed);
    assert_eq!(saved.revision, blocked.revision.next().unwrap());
    assert_eq!(code(client.call(exact.clone()).await), ErrorCode::Conflict);
    assert_eq!(
        code(
            client
                .call(ReleaseWorkspaceAutomation {
                    workspace: archived.id,
                    ..exact
                })
                .await
        ),
        ErrorCode::Conflict
    );
}
