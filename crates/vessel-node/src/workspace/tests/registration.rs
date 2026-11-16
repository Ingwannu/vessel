//! Runtime registration (ADR 0106) never refuses the caller's start.
use super::*;
use std::time::Duration;
use vessel_proto::WorkspaceId;
use vessel_proto::registry::{
    RegistryRevision, WorkspaceAutomation, WorkspaceBacking, WorkspaceDescriptor,
    WorkspaceLifecycle, WorkspacePin, WorkspaceVisibility,
};
use vessel_store::RegistryRecord;

const REGISTRATION_DEADLINE: Duration = Duration::from_secs(60);

/// A live workspace of some other, unregistered project.
fn elsewhere() -> RegistryRecord {
    RegistryRecord {
        descriptor: WorkspaceDescriptor {
            id: WorkspaceId::random(),
            project: ProjectId::random(),
            path: "/elsewhere".into(),
            automation: WorkspaceAutomation::Allowed,
            title: None,
            pin: WorkspacePin::Unpinned,
            backing: WorkspaceBacking::Local,
            lifecycle: WorkspaceLifecycle::Active,
            visibility: WorkspaceVisibility::Foreground,
            created_at: chrono::DateTime::UNIX_EPOCH,
            revision: RegistryRevision::ZERO,
        },
        recovery: None,
    }
}

#[tokio::test]
async fn registration_finds_one_root_workspace_and_a_full_registry_refuses_nothing() {
    vessel_testkit::within(REGISTRATION_DEADLINE, async {
        let fixture = Fixture::new().await;
        let project = fixture.project("root", None).await;
        // Two starts at once: lookup and insert share one writer savepoint.
        let ((), ()) = tokio::join!(
            fixture.workspaces.register_root(project.id),
            fixture.workspaces.register_root(project.id)
        );
        fixture.workspaces.register_root(project.id).await;
        let stored = fixture.store.workspace_registry().await.unwrap();
        let [workspace] = stored.workspaces.as_slice() else {
            panic!("one workspace per root, found {:?}", stored.workspaces);
        };
        assert_eq!(workspace.path, project.root);
        // An unknown project registers nothing and refuses nothing.
        fixture.workspaces.register_root(ProjectId::random()).await;
        let full = fixture.project("full", None).await;
        for _ in 1..vessel_store::MAX_WORKSPACES {
            fixture
                .store
                .put_registry_workspace(elsewhere(), None)
                .await
                .unwrap();
        }
        // At the live cap the start goes on; the failure is only logged.
        fixture.workspaces.register_root(full.id).await;
        let stored = fixture.store.workspace_registry().await.unwrap();
        assert_eq!(stored.workspaces.len(), vessel_store::MAX_WORKSPACES);
        assert!(!stored.workspaces.iter().any(|item| item.project == full.id));
        fixture.stop.cancel();
    })
    .await
    .expect("registration did not settle within its safety deadline");
}
