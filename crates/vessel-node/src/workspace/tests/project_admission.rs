//! Project holds fence actual unregister state, not a path-derived owner cache.
use super::*;
use std::future::Future as _;
use std::task::Poll;

#[tokio::test]
async fn project_admission_blocks_unregister_until_the_final_holder_releases() {
    let fixture = Fixture::new().await;
    let project = fixture.project("held-project", None).await;
    let admission = fixture.workspaces.admit_project(project.id).await.unwrap();
    let retained = Arc::clone(&admission);
    assert_eq!(admission.root().path(), project.root);
    let remove = fixture.workspaces.remove_project(project.id, &[]);
    tokio::pin!(remove);
    assert!(matches!(
        std::future::poll_fn(|cx| Poll::Ready(remove.as_mut().poll(cx))).await,
        Poll::Pending
    ));
    assert!(fixture.workspaces.has_project(project.id));
    drop(admission);
    assert!(matches!(
        std::future::poll_fn(|cx| Poll::Ready(remove.as_mut().poll(cx))).await,
        Poll::Pending
    ));
    assert!(fixture.workspaces.has_project(project.id));
    drop(retained);
    remove.await.unwrap();
    assert!(!fixture.workspaces.has_project(project.id));
    assert!(project.root.is_dir(), "unregister retains the directory");
    fixture.stop.cancel();
}

#[tokio::test]
async fn admission_rejects_a_different_project_or_workspaces_allocation() {
    let fixture = Fixture::new().await;
    let other = Fixture::new().await;
    let project = fixture.project("admitted-project", None).await;
    let admission = fixture.workspaces.admit_project(project.id).await.unwrap();
    assert!(admission.require(&fixture.workspaces, project.id).is_ok());
    assert!(matches!(
        admission.require(&fixture.workspaces, ProjectId::random()),
        Err(WorkspaceError::Conflict(_))
    ));
    assert!(matches!(
        admission.require(&other.workspaces, project.id),
        Err(WorkspaceError::Conflict(_))
    ));
    fixture.stop.cancel();
    other.stop.cancel();
}

#[tokio::test]
async fn project_admission_capacity_refuses_before_mutation_and_recovers_on_release() {
    use crate::workspace::projects::admission::MAX_PROJECT_ADMISSIONS;
    let fixture = Fixture::new().await;
    let project = fixture.project("bounded-project", None).await;
    let capacity = fixture
        .workspaces
        .project_admissions
        .acquire_many(u32::try_from(MAX_PROJECT_ADMISSIONS).unwrap())
        .await
        .unwrap();
    assert!(matches!(
        fixture.workspaces.admit_project(project.id).await,
        Err(WorkspaceError::Busy(_))
    ));
    assert!(fixture.workspaces.has_project(project.id));
    drop(capacity);
    let admission = fixture.workspaces.admit_project(project.id).await.unwrap();
    assert_eq!(
        fixture.workspaces.project_admissions.available_permits(),
        MAX_PROJECT_ADMISSIONS - 1
    );
    drop(admission);
    assert_eq!(
        fixture.workspaces.project_admissions.available_permits(),
        MAX_PROJECT_ADMISSIONS
    );
    fixture.stop.cancel();
}

#[tokio::test]
async fn admitted_backing_refuses_legacy_bootstrap_before_any_worktree_effect() {
    let fixture = Fixture::new().await;
    let project = fixture.project("no-bootstrap", None).await;
    let admission = fixture.workspaces.admit_project(project.id).await.unwrap();
    let result = fixture
        .workspaces
        .create_worktree_admitted(
            Arc::clone(&admission),
            Creator::Owner,
            CreateWorktree {
                project: project.id,
                branch: WorktreeBranch::New {
                    name: None,
                    base: None,
                },
                setup: SetupChoice::Run,
            },
            None,
            vessel_proto::WorktreeId::random(),
        )
        .await;
    assert!(matches!(result, Err(WorkspaceError::Invalid(_))));
    assert!(
        fixture
            .workspaces
            .list_worktrees(Some(project.id), &[])
            .worktrees
            .is_empty()
    );
    fixture.stop.cancel();
}
