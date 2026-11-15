//! Real durable registry with isolated Git; no provider session is launched.

use super::Fixture;
use crate::workspace::registry::{Provisioned, RegistryError};
use vessel_proto::registry::{WorkspaceBacking, WorkspaceIsolation, WorkspaceVisibility};
use vessel_proto::workspace::{SetupChoice, WorktreeBranch};
use vessel_proto::{Creator, WorkspaceId};
use vessel_store::{ReservationStage, WorkspaceProvision};

fn local(project: vessel_proto::ProjectId) -> WorkspaceProvision {
    WorkspaceProvision {
        project,
        isolation: WorkspaceIsolation::Local,
        automation: vessel_proto::registry::WorkspaceAutomation::Allowed,
        title: None,
        visibility: WorkspaceVisibility::Foreground,
    }
}

#[tokio::test]
async fn a_project_can_have_two_empty_local_workspaces_at_the_same_cwd() {
    let fixture = Fixture::new().await;
    let project = fixture.project("shared", None).await;
    let a = WorkspaceId::random();
    let b = WorkspaceId::random();
    let first = fixture
        .workspaces
        .provision_workspace(Creator::Owner, a, local(project.id))
        .await
        .unwrap();
    let second = fixture
        .workspaces
        .provision_workspace(Creator::Owner, b, local(project.id))
        .await
        .unwrap();
    let (Provisioned::Ready(first), Provisioned::Ready(second)) = (first, second) else {
        panic!("local workspaces must be ready")
    };
    assert_ne!(first.id, second.id);
    assert_eq!(first.path, project.root);
    assert_eq!(second.path, project.root);
    assert_eq!(first.backing, WorkspaceBacking::Local);
    let snapshot = fixture.store.workspace_registry().await.unwrap();
    assert_eq!(snapshot.workspaces.len(), 2);
    assert!(snapshot.memberships.is_empty());
    assert!(
        fixture
            .workspaces
            .list_worktrees(Some(project.id), &[])
            .worktrees
            .is_empty()
    );
    fixture.stop.cancel();
}

#[tokio::test]
async fn repeating_a_reserved_workspace_never_duplicates_its_worktree() {
    let fixture = Fixture::new().await;
    let project = fixture.project("managed", None).await;
    let id = WorkspaceId::random();
    let intent = WorkspaceProvision {
        isolation: WorkspaceIsolation::Worktree {
            branch: WorktreeBranch::New {
                name: None,
                base: None,
            },
            setup: SetupChoice::Skip,
        },
        ..local(project.id)
    };
    let first = fixture
        .workspaces
        .provision_workspace(Creator::Owner, id, intent.clone())
        .await
        .unwrap();
    let second = fixture
        .workspaces
        .provision_workspace(Creator::Owner, id, intent)
        .await
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(
        fixture
            .workspaces
            .list_worktrees(Some(project.id), &[])
            .worktrees
            .len(),
        1
    );
    let reservation = fixture
        .store
        .registry_reservation(id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reservation.stage, ReservationStage::Ready);
    let Provisioned::Ready(workspace) = first else {
        panic!("worktree should be ready")
    };
    assert_eq!(
        workspace.backing,
        WorkspaceBacking::Managed {
            worktree: reservation.worktree.unwrap()
        }
    );
    assert!(workspace.path.is_dir());
    fixture.stop.cancel();
}

#[tokio::test]
async fn commands_needing_approval_are_returned_without_a_worktree_or_agent() {
    let fixture = Fixture::new().await;
    let project = fixture
        .project("approval", Some("[worktree]\nsetup = ['printf setup']\n"))
        .await;
    let id = WorkspaceId::random();
    let intent = WorkspaceProvision {
        isolation: WorkspaceIsolation::Worktree {
            branch: WorktreeBranch::New {
                name: None,
                base: None,
            },
            setup: SetupChoice::Run,
        },
        ..local(project.id)
    };
    let result = fixture
        .workspaces
        .provision_workspace(Creator::Owner, id, intent)
        .await
        .unwrap();
    assert!(matches!(result, Provisioned::NeedsApproval(_)));
    assert!(
        fixture
            .store
            .workspace_registry()
            .await
            .unwrap()
            .workspaces
            .is_empty()
    );
    assert!(
        fixture
            .workspaces
            .list_worktrees(Some(project.id), &[])
            .worktrees
            .is_empty()
    );
    assert_eq!(
        fixture
            .store
            .registry_reservation(id)
            .await
            .unwrap()
            .unwrap()
            .stage,
        ReservationStage::Reserved
    );
    fixture.stop.cancel();
}

#[tokio::test]
async fn a_workspace_id_cannot_be_reused_for_another_project() {
    let fixture = Fixture::new().await;
    let first = fixture.project("first", None).await;
    let second = fixture.project("second", None).await;
    let id = WorkspaceId::random();
    let _ready = fixture
        .workspaces
        .provision_workspace(Creator::Owner, id, local(first.id))
        .await
        .unwrap();
    let result = fixture
        .workspaces
        .provision_workspace(Creator::Owner, id, local(second.id))
        .await;
    assert!(matches!(result, Err(RegistryError::IntentConflict)));
    assert_eq!(
        fixture
            .store
            .workspace_registry()
            .await
            .unwrap()
            .workspaces
            .len(),
        1
    );
    fixture.stop.cancel();
}

#[tokio::test]
async fn blocked_external_source_skips_setup_and_retains_its_exact_identity() {
    use vessel_proto::forge::{ForgeKind, PullNumber, RepoRef};
    use vessel_proto::registry::{BlockedSource, WorkspaceAutomation};
    let fixture = Fixture::new().await;
    let project = fixture
        .project(
            "blocked",
            Some("[worktree]\nsetup = ['printf ran > blocked-marker']\n"),
        )
        .await;
    let source = BlockedSource::ChangeRequest {
        number: PullNumber::new(1).unwrap(),
        head_repository: RepoRef {
            kind: ForgeKind::Github,
            host: "github.com".parse().unwrap(),
            path: "fork/project".parse().unwrap(),
        },
    };
    let intent = WorkspaceProvision {
        isolation: WorkspaceIsolation::Worktree {
            branch: WorktreeBranch::New {
                name: None,
                base: None,
            },
            setup: SetupChoice::Run,
        },
        automation: WorkspaceAutomation::Blocked {
            source: source.clone(),
        },
        ..local(project.id)
    };
    let result = fixture
        .workspaces
        .provision_workspace(Creator::Owner, WorkspaceId::random(), intent)
        .await
        .unwrap();
    let Provisioned::Ready(workspace) = result else {
        panic!("blocked backing may still be provisioned without automation")
    };
    assert_eq!(
        workspace.automation,
        WorkspaceAutomation::Blocked { source }
    );
    let backing = fixture.workspaces.list_worktrees(Some(project.id), &[]);
    assert_eq!(backing.worktrees.len(), 1);
    assert!(matches!(
        backing.worktrees[0].setup,
        vessel_proto::workspace::SetupState::Skipped
    ));
    assert!(!workspace.path.join("blocked-marker").exists());
    fixture.stop.cancel();
}
