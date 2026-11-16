//! Actual legacy cleanup cannot cross independent registry metadata admission.
use super::*;
use std::time::Duration;
use vessel_proto::WorkspaceId;
use vessel_proto::registry::{
    RegistryRevision, WorkspaceAutomation, WorkspaceBacking, WorkspaceDescriptor,
    WorkspaceIsolation, WorkspaceLifecycle, WorkspacePin, WorkspaceVisibility,
};
use vessel_store::{RegistryRecord, RegistryReservation, ReservationStage, WorkspaceProvision};

const REGISTRY_FENCE_DEADLINE: Duration = Duration::from_secs(60);

fn descriptor(
    project: &ProjectInfo,
    backing: WorkspaceBacking,
    path: PathBuf,
) -> WorkspaceDescriptor {
    WorkspaceDescriptor {
        id: WorkspaceId::random(),
        project: project.id,
        path,
        automation: WorkspaceAutomation::Allowed,
        title: None,
        pin: WorkspacePin::Unpinned,
        backing,
        lifecycle: WorkspaceLifecycle::Active,
        visibility: WorkspaceVisibility::Foreground,
        created_at: chrono::DateTime::UNIX_EPOCH,
        revision: RegistryRevision::ZERO,
    }
}

async fn retain(fixture: &Fixture, item: WorkspaceDescriptor) {
    fixture
        .store
        .put_registry_workspace(
            RegistryRecord {
                descriptor: item,
                recovery: None,
            },
            None,
        )
        .await
        .unwrap();
}

// ADR 0106 deliberately reverses W11's pin that every retained lifecycle
// blocks: a `Local` workspace owns no directory, so legacy unregister archives
// the Active ones in its own write and keeps the records (Paseo TRACKER 258).
#[tokio::test]
async fn registry_fence_local_workspaces_are_archived_by_legacy_project_unregister() {
    vessel_testkit::within(REGISTRY_FENCE_DEADLINE, async {
        let fixture = Fixture::new().await;
        let archived_at = chrono::DateTime::UNIX_EPOCH;
        for archived in [false, true] {
            let project = fixture
                .project(if archived { "archived" } else { "active" }, None)
                .await;
            let mut item = descriptor(&project, WorkspaceBacking::Local, project.root.clone());
            if archived {
                item.lifecycle = WorkspaceLifecycle::Archived { at: archived_at };
            }
            let id = item.id;
            retain(&fixture, item).await;
            fixture
                .workspaces
                .remove_project(project.id, &[])
                .await
                .unwrap();
            assert!(!fixture.workspaces.has_project(project.id));
            assert!(project.root.join("README.md").is_file());
            let kept = fixture.store.registry_workspace(id).await.unwrap().unwrap();
            let WorkspaceLifecycle::Archived { at } = kept.descriptor.lifecycle else {
                panic!(
                    "the record must be archived and kept, found {:?}",
                    kept.descriptor.lifecycle
                );
            };
            if archived {
                assert_eq!(at, archived_at);
            } else {
                assert!(at > archived_at);
            }
        }
        fixture.stop.cancel();
    })
    .await
    .expect("registry cleanup fence did not settle within its safety deadline");
}

#[tokio::test]
async fn registry_fence_local_workspace_in_transition_still_blocks_legacy_project_unregister() {
    vessel_testkit::within(REGISTRY_FENCE_DEADLINE, async {
        let fixture = Fixture::new().await;
        let at = chrono::DateTime::UNIX_EPOCH;
        for (name, lifecycle) in [
            ("archiving", WorkspaceLifecycle::Archiving { at }),
            ("restoring", WorkspaceLifecycle::Restoring { at }),
        ] {
            let project = fixture.project(name, None).await;
            let mut item = descriptor(&project, WorkspaceBacking::Local, project.root.clone());
            item.lifecycle = lifecycle;
            retain(&fixture, item).await;
            let result = fixture.workspaces.remove_project(project.id, &[]).await;
            assert!(
                matches!(result, Err(WorkspaceError::RegistryProjectOwned(id)) if id==project.id)
            );
            assert!(fixture.workspaces.has_project(project.id));
        }
        fixture.stop.cancel();
    })
    .await
    .expect("registry cleanup fence did not settle within its safety deadline");
}

#[tokio::test]
async fn registry_fence_managed_backing_blocks_discard_before_teardown_and_keeps_dirty_files() {
    vessel_testkit::within(REGISTRY_FENCE_DEADLINE, async {
        let fixture = Fixture::new().await;
        let project = fixture
            .project(
                "managed",
                Some("[worktree]\nteardown = [\"printf ran > ../teardown-marker\"]\n"),
            )
            .await;
        let CreateWorktreeOutcome::Created { worktree } = fixture
            .create(project.id, Creator::Owner, SetupChoice::Skip)
            .await
        else {
            panic!("fixture backing must be created without bootstrap");
        };
        let ProjectCommands::Listed { digest, .. } =
            fixture.workspaces.commands(project.id, None).await.unwrap()
        else {
            panic!("fixture teardown must be declared");
        };
        let _approved = fixture
            .workspaces
            .approve(project.id, digest, true)
            .await
            .unwrap();
        let marker = worktree.path.parent().unwrap().join("teardown-marker");
        std::fs::write(worktree.path.join("uncommitted"), "keep this work").unwrap();
        let item = descriptor(
            &project,
            WorkspaceBacking::Managed {
                worktree: worktree.id,
            },
            worktree.path.clone(),
        );
        retain(&fixture, item).await;
        let result = fixture
            .workspaces
            .remove_worktree(worktree.id, IfLost::Discard, Vec::new)
            .await;
        assert!(matches!(result, Err(WorkspaceError::RegistryProjectOwned(id)) if id==project.id));
        assert_eq!(
            std::fs::read_to_string(worktree.path.join("uncommitted")).unwrap(),
            "keep this work"
        );
        assert!(
            !marker.exists(),
            "approved teardown must not execute before the registry fence"
        );
        assert!(
            fixture
                .workspaces
                .state()
                .worktrees
                .get(&worktree.id)
                .is_some_and(|entry| entry.running.is_none())
        );
        fixture.stop.cancel();
    })
    .await
    .expect("registry cleanup fence did not settle within its safety deadline");
}

#[tokio::test]
async fn registry_fence_uncertain_reservation_without_descriptor_blocks_legacy_deletion() {
    vessel_testkit::within(REGISTRY_FENCE_DEADLINE, async {
        let fixture = Fixture::new().await;
        let project = fixture.project("reserved", None).await;
        let worktree = fixture.created(project.id).await;
        fixture
            .store
            .reserve_registry_workspace(RegistryReservation {
                workspace: WorkspaceId::random(),
                worktree: Some(worktree.id),
                stage: ReservationStage::Uncertain,
                intent: WorkspaceProvision {
                    project: project.id,
                    isolation: WorkspaceIsolation::Local,
                    automation: WorkspaceAutomation::Allowed,
                    title: None,
                    visibility: WorkspaceVisibility::Foreground,
                },
            })
            .await
            .unwrap();
        assert!(
            fixture
                .store
                .workspace_registry()
                .await
                .unwrap()
                .workspaces
                .is_empty()
        );
        let result = fixture
            .workspaces
            .remove_worktree(worktree.id, IfLost::Discard, Vec::new)
            .await;
        assert!(matches!(result, Err(WorkspaceError::RegistryProjectOwned(id)) if id==project.id));
        assert!(worktree.path.join("README.md").is_file());
        fixture.stop.cancel();
    })
    .await
    .expect("registry cleanup fence did not settle within its safety deadline");
}

#[tokio::test]
async fn registry_fence_project_admission_serializes_publication_against_legacy_remove() {
    vessel_testkit::within(REGISTRY_FENCE_DEADLINE, async {
        use std::future::Future as _;
        let fixture = Fixture::new().await;
        let project = fixture.project("publication", None).await;
        let worktree = fixture.created(project.id).await;
        let admission = fixture.workspaces.admit_project(project.id).await.unwrap();
        let mut remove = Box::pin(fixture.workspaces.remove_worktree(
            worktree.id,
            IfLost::Discard,
            Vec::new,
        ));
        {
            let mut context = std::task::Context::from_waker(std::task::Waker::noop());
            assert!(remove.as_mut().poll(&mut context).is_pending());
        }
        retain(
            &fixture,
            descriptor(
                &project,
                WorkspaceBacking::Managed {
                    worktree: worktree.id,
                },
                worktree.path.clone(),
            ),
        )
        .await;
        drop(admission);
        assert!(
            matches!(remove.await, Err(WorkspaceError::RegistryProjectOwned(id)) if id==project.id)
        );
        assert!(worktree.path.exists());
        fixture.stop.cancel();
    })
    .await
    .expect("registry cleanup fence did not settle within its safety deadline");
}

#[tokio::test]
async fn registry_fence_automatic_retire_keeps_registry_backing_and_unrelated_legacy_still_removes()
{
    vessel_testkit::within(REGISTRY_FENCE_DEADLINE, async {
        let fixture = Fixture::new().await;
        let project = fixture.project("retired-managed", None).await;
        let session = SessionId::random();
        let result = fixture
            .workspaces
            .create_worktree(
                Creator::Owner,
                CreateWorktree {
                    project: project.id,
                    branch: WorktreeBranch::New {
                        name: None,
                        base: None,
                    },
                    setup: SetupChoice::Skip,
                },
                Some(session),
            )
            .await
            .unwrap();
        let CreateWorktreeOutcome::Created { worktree } = result else {
            panic!("fixture backing")
        };
        retain(
            &fixture,
            descriptor(
                &project,
                WorkspaceBacking::Managed {
                    worktree: worktree.id,
                },
                worktree.path.clone(),
            ),
        )
        .await;
        let tidied = fixture.workspaces.retire(session, Vec::new).await.unwrap();
        assert!(matches!(tidied.outcome, RemoveWorktreeOutcome::Kept { .. }));
        assert!(worktree.path.join("README.md").is_file());
        let unowned = fixture.project("unowned", None).await;
        let other = fixture.created(unowned.id).await;
        let removed = fixture
            .workspaces
            .remove_worktree(other.id, IfLost::Discard, Vec::new)
            .await
            .unwrap();
        assert!(matches!(removed, RemoveWorktreeOutcome::Removed { .. }));
        assert!(!other.path.exists());
        fixture.stop.cancel();
    })
    .await
    .expect("registry cleanup fence did not settle within its safety deadline");
}

#[tokio::test]
async fn registry_fence_saturation_refuses_before_detach_and_releases_the_same_budget() {
    vessel_testkit::within(REGISTRY_FENCE_DEADLINE, async {
        use crate::workspace::projects::admission::MAX_PROJECT_ADMISSIONS;
        use std::future::Future as _;
        let fixture = Fixture::new().await;
        let project = fixture.project("saturated-removal", None).await;
        let worktree = fixture.created(project.id).await;
        let admission = fixture.workspaces.admit_project(project.id).await.unwrap();
        let baseline = fixture.tasks.len();
        let mut waiters = Vec::new();
        {
            let mut context = std::task::Context::from_waker(std::task::Waker::noop());
            for _ in 1..MAX_PROJECT_ADMISSIONS {
                let mut waiter = Box::pin(fixture.workspaces.admit_project(project.id));
                assert!(waiter.as_mut().poll(&mut context).is_pending());
                waiters.push(waiter);
            }
        }
        let result = fixture
            .workspaces
            .remove_worktree(worktree.id, IfLost::Discard, Vec::new)
            .await;
        assert!(matches!(result, Err(WorkspaceError::Busy(_))));
        assert_eq!(fixture.tasks.len(), baseline);
        assert!(worktree.path.exists());
        drop(waiters);
        drop(admission);
        let removed = fixture
            .workspaces
            .remove_worktree(worktree.id, IfLost::Discard, Vec::new)
            .await
            .unwrap();
        assert!(matches!(removed, RemoveWorktreeOutcome::Removed { .. }));
        fixture.stop.cancel();
    })
    .await
    .expect("registry cleanup fence did not settle within its safety deadline");
}
