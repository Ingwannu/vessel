//! Exercise the real Store and Git producer while the wire row is pending.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use chrono::Utc;
use vessel_proto::registry::{
    WorkspaceAutomation, WorkspaceBacking, WorkspaceIsolation, WorkspaceLifecycle,
    WorkspaceResource, WorkspaceVisibility,
};
use vessel_proto::{SessionId, TerminalId, WorkspaceId};
use vessel_session::IdleStop;
use vessel_store::{
    ArchiveDefer, ProvisionRetirement, WorkspaceArchivePurpose, WorkspaceProvision,
};

use super::*;
use crate::workspace::{ArchiveResult, Occupant, WorkspaceActors, WorkspaceError};

struct Actors {
    scheduled: SessionId,
    foreign: WorkspaceResource,
    foreign_live: AtomicBool,
    stopped: AtomicUsize,
}

impl WorkspaceActors for Actors {
    fn live(&self, resource: WorkspaceResource) -> bool {
        resource == self.foreign && self.foreign_live.load(Ordering::SeqCst)
    }

    async fn joined(&self, resource: WorkspaceResource) -> Result<bool, WorkspaceError> {
        Ok(!self.live(resource))
    }

    fn occupied_folders(&self) -> Vec<(Occupant, PathBuf)> {
        Vec::new()
    }

    async fn stop_idle(&self, session: SessionId) -> Result<IdleStop, WorkspaceError> {
        assert_eq!(session, self.scheduled);
        let _previous = self.stopped.fetch_add(1, Ordering::SeqCst);
        Ok(IdleStop::Stopped)
    }

    async fn stop_member(&self, _: WorkspaceResource) -> Result<(), WorkspaceError> {
        Err(WorkspaceError::Conflict(
            "automatic archive must never call the explicit stop".to_owned(),
        ))
    }
}

#[tokio::test]
async fn a_managed_archive_defers_without_stopping_foreign_members_or_clearing_the_backing() {
    let fixture = Fixture::new().await;
    let project = fixture.project("isolated", None).await;
    let workspaces = &fixture.workspaces;
    let id = WorkspaceId::random();
    let intent = WorkspaceProvision {
        project: project.id,
        isolation: WorkspaceIsolation::Worktree {
            branch: WorktreeBranch::New {
                name: None,
                base: None,
            },
            setup: SetupChoice::Skip,
        },
        automation: WorkspaceAutomation::Allowed,
        title: None,
        visibility: WorkspaceVisibility::Foreground,
    };
    let admission = workspaces.admit_project(project.id).await.unwrap();
    let made = workspaces
        .provision_workspace(id, admission, intent.clone(), Creator::Owner)
        .await
        .unwrap();
    let WorkspaceBacking::Managed { worktree } = made.descriptor.backing else {
        panic!("the producer returned the wrong backing");
    };
    assert_eq!(workspaces.worktree(worktree).unwrap().created_for, None);
    let replay = workspaces
        .provision_workspace(
            id,
            workspaces.admit_project(project.id).await.unwrap(),
            intent,
            Creator::Owner,
        )
        .await
        .unwrap();
    assert_eq!(replay.descriptor, made.descriptor);

    let actors = Actors {
        scheduled: SessionId::random(),
        foreign: WorkspaceResource::Terminal {
            terminal: TerminalId::random(),
        },
        foreign_live: AtomicBool::new(true),
        stopped: AtomicUsize::new(0),
    };
    for resource in [
        WorkspaceResource::Session {
            session: actors.scheduled,
        },
        actors.foreign,
    ] {
        let lease = workspaces
            .admit_workspace_start(&made, resource)
            .await
            .unwrap();
        fixture.store.complete_registry_start(lease).await.unwrap();
    }
    let current = fixture.store.registry_workspace(id).await.unwrap().unwrap();
    let result = workspaces
        .archive_workspace(
            id,
            current.descriptor.revision,
            WorkspaceArchivePurpose::Automatic,
            Some(actors.scheduled),
            &actors,
        )
        .await
        .unwrap();
    assert_eq!(result, ArchiveResult::Deferred(ArchiveDefer::OtherMembers));
    assert_eq!(actors.stopped.load(Ordering::SeqCst), 0);
    assert!(made.descriptor.path.exists());
    let deferred = fixture.store.registry_workspace(id).await.unwrap().unwrap();
    assert_eq!(deferred.descriptor.lifecycle, WorkspaceLifecycle::Active);
    actors.foreign_live.store(false, Ordering::SeqCst);
    let result = workspaces
        .archive_workspace(
            id,
            deferred.descriptor.revision,
            WorkspaceArchivePurpose::Automatic,
            Some(actors.scheduled),
            &actors,
        )
        .await
        .unwrap();
    assert_eq!(result, ArchiveResult::Archived { kept: Vec::new() });
    assert_eq!(actors.stopped.load(Ordering::SeqCst), 1);
    assert!(!made.descriptor.path.exists());
    assert!(project.root.join("README.md").exists());
    fixture
        .store
        .retire_registry_workspace(id, ProvisionRetirement::CleanArchive, Utc::now())
        .await
        .unwrap();
    fixture.stop.cancel();
}

#[tokio::test]
async fn a_person_can_resolve_archived_dirty_debt_and_release_the_project_fence() {
    let fixture = Fixture::new().await;
    let project = fixture.project("dirty-debt", None).await;
    let workspaces = &fixture.workspaces;
    let id = WorkspaceId::random();
    let intent = WorkspaceProvision {
        project: project.id,
        isolation: WorkspaceIsolation::Worktree {
            branch: WorktreeBranch::New {
                name: None,
                base: None,
            },
            setup: SetupChoice::Skip,
        },
        automation: WorkspaceAutomation::Allowed,
        title: None,
        visibility: WorkspaceVisibility::Foreground,
    };
    let made = workspaces
        .provision_workspace(
            id,
            workspaces.admit_project(project.id).await.unwrap(),
            intent,
            Creator::Owner,
        )
        .await
        .unwrap();
    let WorkspaceBacking::Managed { worktree } = made.descriptor.backing else {
        panic!("fixture needs a managed backing");
    };
    std::fs::write(made.descriptor.path.join("dirty.txt"), "retain my work").unwrap();
    let actors = Actors {
        scheduled: SessionId::random(),
        foreign: WorkspaceResource::Terminal {
            terminal: TerminalId::random(),
        },
        foreign_live: AtomicBool::new(false),
        stopped: AtomicUsize::new(0),
    };
    let archived = workspaces
        .archive_workspace(
            id,
            made.descriptor.revision,
            WorkspaceArchivePurpose::Automatic,
            None,
            &actors,
        )
        .await
        .unwrap();
    assert!(matches!(archived, ArchiveResult::Archived { kept } if !kept.is_empty()));
    assert_eq!(
        fixture
            .store
            .archived_worktree_debt(worktree)
            .await
            .unwrap(),
        Some(id)
    );
    assert_eq!(
        fixture
            .store
            .registry_project_reference(project.id)
            .await
            .unwrap(),
        Some(id)
    );
    let kept = workspaces
        .remove_worktree(worktree, IfLost::Keep, Vec::new)
        .await
        .unwrap();
    assert!(matches!(kept, RemoveWorktreeOutcome::Kept { .. }));
    assert!(made.descriptor.path.join("dirty.txt").exists());
    let removed = workspaces
        .remove_worktree(worktree, IfLost::Discard, Vec::new)
        .await
        .unwrap();
    assert!(matches!(removed, RemoveWorktreeOutcome::Removed { .. }));
    assert!(!made.descriptor.path.exists());
    assert_eq!(
        fixture
            .store
            .registry_reservation(id)
            .await
            .unwrap()
            .unwrap()
            .stage,
        vessel_store::ReservationStage::Retired
    );
    assert_eq!(
        fixture
            .store
            .registry_project_reference(project.id)
            .await
            .unwrap(),
        None
    );
    workspaces.remove_project(project.id, &[]).await.unwrap();
    assert!(project.root.join("README.md").exists());
    fixture.stop.cancel();
}
