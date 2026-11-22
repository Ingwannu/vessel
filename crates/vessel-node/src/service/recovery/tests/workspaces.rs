//! An owner's member exercises the actual startup and workspace leases.

use super::*;
use vessel_proto::registry::{
    WorkspaceAutomation, WorkspaceIsolation, WorkspaceResource, WorkspaceVisibility,
};
use vessel_proto::{Creator, WorkspaceId, workspace::AddProject};
use vessel_store::{
    ArchiveSettlement, RegistryResourceReadiness, Store, StoreConfig, WorkspaceArchivePurpose,
    WorkspaceProvision,
};

async fn explicit_member(
    fixture: &Fixture,
    node: &Node,
    owner: &NodeClient,
) -> WorkspaceResource {
    let service = &node.secure.service.0;
    let project = owner
        .call(AddProject {
            path: fixture.work.clone(),
            name: None,
        })
        .await
        .unwrap();
    let workplace = service
        .workspaces
        .provision_workspace(
            WorkspaceId::random(),
            service.workspaces.admit_project(project.id).await.unwrap(),
            WorkspaceProvision {
                project: project.id,
                isolation: WorkspaceIsolation::Local,
                automation: WorkspaceAutomation::Allowed,
                title: None,
                visibility: WorkspaceVisibility::Foreground,
            },
            Creator::Owner,
        )
        .await
        .unwrap();
    let session = SessionId::random();
    let resource = WorkspaceResource::Session { session };
    let lease = service
        .workspaces
        .admit_workspace_start(&workplace, resource)
        .await
        .unwrap();
    let spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    )
    .with_place(workplace.place)
    .with_profile("recovery".parse().unwrap());
    let info = service
        .create_starting(
            &crate::principal::Principal::Owner,
            spec,
            (None, None),
            None,
            None,
            Some(&lease),
        )
        .await
        .unwrap();
    assert_eq!(info.id, session);
    service
        .sessions
        .store()
        .complete_registry_start(lease)
        .await
        .unwrap();
    let through = begin_turn(owner, session).await;
    let _original = committed(node, session, through).await;
    resource
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn startup_excludes_an_unarchived_session_whose_explicit_workspace_is_archived() {
    vessel_testkit::within(
        CASE_DEADLINE,
        Box::pin(async {
            let fixture = Fixture::new();
            let node = fixture.start().await;
            let owner = fixture.client(&node).await;
            set(&node, true).await;
            let resource = explicit_member(&fixture, &node, &owner).await;
            node.stop().await.unwrap();

            let store = open_store(&fixture).await;
            let candidate = store.next_restart().await.unwrap().unwrap();
            assert_eq!(resource, WorkspaceResource::Session { session: candidate.session });
            let workspace = store.registry_resource_run(resource).await.unwrap().unwrap().lease.workspace();
            let revision = store.registry_workspace(workspace).await.unwrap().unwrap().descriptor.revision;
            // The previous actor has joined. Settle only the workspace record
            // so the session's independent Archive mark cannot hide this case.
            let claim = store.claim_registry_archive(workspace, revision, WorkspaceArchivePurpose::Explicit, Utc::now()).await.unwrap();
            store.settle_registry_archive(claim, ArchiveSettlement::Archived { at: Utc::now() }).await.unwrap();
            store.close().await;

            let store = open_store(&fixture).await;
            let info = store.session(candidate.session).await.unwrap().unwrap();
            assert!(info.marks.archived_at.is_none());
            assert_eq!(info.status, SessionStatus::Interrupted);
            assert!(store.next_restart().await.unwrap().is_none());
            store.close().await;
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn startup_reopens_an_explicit_member_under_a_new_exact_workspace_lease() {
    vessel_testkit::within(
        CASE_DEADLINE,
        Box::pin(async {
            let fixture = Fixture::new();
            let node = fixture.start().await;
            let owner = fixture.client(&node).await;
            set(&node, true).await;
            let resource = explicit_member(&fixture, &node, &owner).await;
            let before = node.secure.service.0.sessions.store().registry_resource_run(resource).await.unwrap().unwrap();
            node.stop().await.unwrap();

            let node = fixture.start().await;
            done(&node).await;
            let owner = fixture.client(&node).await;
            let after = node.secure.service.0.sessions.store().registry_resource_run(resource).await.unwrap().unwrap();
            assert_eq!(after.lease.workspace(), before.lease.workspace());
            assert_eq!(after.lease.resource(), before.lease.resource());
            assert_ne!(after.lease, before.lease);
            assert_eq!(after.readiness, RegistryResourceReadiness::Ready);
            let WorkspaceResource::Session { session } = resource else { unreachable!() };
            let records = owner.read(session, EventSeq::ZERO, EVENT_PAGE).await.unwrap().records;
            assert_eq!(records.iter().filter(|record| restart::continuation(record)).count(), 1);
            node.stop().await.unwrap();
        }),
    )
    .await
    .unwrap();
}

async fn open_store(fixture: &Fixture) -> Store {
    Store::open_with_restart_policy(&fixture.data, StoreConfig::default(), true)
        .await
        .unwrap()
        .store
}
