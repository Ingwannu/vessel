//! Selected roots remain independent and registration is durable before publication.

use super::Fixture;
use crate::workspace::WorkspaceError;
use vessel_proto::{
    ErrorCode,
    project_management::{
        CreateProjectDirectory, ProjectDirectoryCleanup, ProjectDirectoryOutcome,
    },
};

#[tokio::test]
async fn a_nested_selected_root_is_independent_while_legacy_add_still_uses_git_top() {
    let fixture = Fixture::new().await;
    let root = fixture.repo("repository", None);
    let nested = root.join("subproject");
    std::fs::create_dir(&nested).unwrap();
    let legacy = fixture
        .workspaces
        .add_project(nested.clone(), None)
        .await
        .unwrap();
    let selected = fixture
        .workspaces
        .register_project_directory(nested.clone(), None)
        .await
        .unwrap();
    assert_eq!(legacy.root, root);
    assert_eq!(selected.root, nested);
    assert_ne!(legacy.id, selected.id);
    assert!(selected.git);
    assert_eq!(
        fixture
            .workspaces
            .add_project(selected.root.clone(), None)
            .await
            .unwrap()
            .id,
        legacy.id
    );
    assert_eq!(fixture.store.workspace().await.unwrap().projects.len(), 2);
    fixture.stop.cancel();
}

#[tokio::test]
async fn concurrent_exact_root_registration_publishes_one_durable_identity() {
    let fixture = Fixture::new().await;
    let root = fixture.repo("concurrent", None);
    let (a, b) = tokio::join!(
        fixture
            .workspaces
            .register_project_directory(root.clone(), None),
        fixture.workspaces.register_project_directory(root, None)
    );
    let a = a.unwrap();
    let b = b.unwrap();
    assert_eq!(a.id, b.id);
    assert_eq!(fixture.workspaces.list_projects().await.projects.len(), 1);
    assert_eq!(fixture.store.workspace().await.unwrap().projects, [a]);
    fixture.stop.cancel();
}

#[tokio::test]
async fn failed_durable_registration_never_leaves_a_visible_ghost_project() {
    let fixture = Fixture::new().await;
    let root = fixture.repo("failed-store", None);
    fixture.store.close().await;
    let result = fixture
        .workspaces
        .register_project_directory(root, None)
        .await;
    assert!(
        matches!(result, Err(WorkspaceError::Store(_))),
        "{result:?}"
    );
    assert!(fixture.workspaces.list_projects().await.projects.is_empty());
    fixture.stop.cancel();
}

#[tokio::test]
async fn creating_a_plain_directory_registers_it_without_initializing_git_or_a_workspace() {
    let fixture = Fixture::new().await;
    let result = fixture
        .workspaces
        .create_project_directory(CreateProjectDirectory {
            parent: fixture.path(),
            name: "plain-project".to_owned().try_into().unwrap(),
        })
        .await
        .unwrap();
    let ProjectDirectoryOutcome::Created { project } = result else {
        panic!("{result:?}")
    };
    assert_eq!(project.root, fixture.path().join("plain-project"));
    assert!(!project.git);
    assert!(std::fs::read_dir(&project.root).unwrap().next().is_none());
    let stored = fixture.store.workspace().await.unwrap();
    assert_eq!(stored.projects, [project]);
    assert!(stored.worktrees.is_empty());
    fixture.stop.cancel();
}

#[tokio::test]
async fn new_directory_requires_an_existing_parent_and_never_adopts_an_existing_target() {
    let fixture = Fixture::new().await;
    let parent = fixture.path().join("absent");
    let result = fixture
        .workspaces
        .create_project_directory(CreateProjectDirectory {
            parent: parent.clone(),
            name: "child".to_owned().try_into().unwrap(),
        })
        .await;
    assert!(result.is_err());
    assert!(!parent.exists());
    let existing = fixture.path().join("existing");
    std::fs::create_dir(&existing).unwrap();
    std::fs::write(existing.join("keep"), "keep").unwrap();
    let result = fixture
        .workspaces
        .create_project_directory(CreateProjectDirectory {
            parent: fixture.path(),
            name: "existing".to_owned().try_into().unwrap(),
        })
        .await
        .unwrap_err();
    assert_eq!(result.code, ErrorCode::Conflict);
    assert_eq!(
        std::fs::read_to_string(existing.join("keep")).unwrap(),
        "keep"
    );
    assert!(fixture.workspaces.list_projects().await.projects.is_empty());
    fixture.stop.cancel();
}

#[tokio::test]
async fn registration_failure_removes_only_the_new_empty_directory() {
    let fixture = Fixture::new().await;
    fixture.store.close().await;
    let result = fixture
        .workspaces
        .create_project_directory(CreateProjectDirectory {
            parent: fixture.path(),
            name: "failed-registration".to_owned().try_into().unwrap(),
        })
        .await
        .unwrap();
    assert!(
        matches!(
            result,
            ProjectDirectoryOutcome::RegistrationFailed {
                cleanup: ProjectDirectoryCleanup::Removed,
                ..
            }
        ),
        "{result:?}"
    );
    assert!(!fixture.path().join("failed-registration").exists());
    assert!(fixture.workspaces.list_projects().await.projects.is_empty());
    fixture.stop.cancel();
}

#[tokio::test]
async fn config_requires_current_registered_identity_and_two_saves_cannot_both_win() {
    use vessel_proto::{
        ProjectId,
        registry::{ConfigRevision, ConfigText, WriteProjectConfig},
    };
    let fixture = Fixture::new().await;
    let project = fixture.project("config", None).await;
    let missing = fixture
        .workspaces
        .read_project_config(project.id)
        .await
        .unwrap();
    assert_eq!(missing.revision, ConfigRevision::Missing);
    assert_eq!(
        fixture
            .workspaces
            .read_project_config(ProjectId::random())
            .await
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    );
    let a = WriteProjectConfig {
        project: project.id,
        expected: missing.revision.clone(),
        text: ConfigText::parse("# first").unwrap(),
    };
    let b = WriteProjectConfig {
        project: project.id,
        expected: missing.revision,
        text: ConfigText::parse("# second").unwrap(),
    };
    let (a, b) = tokio::join!(
        fixture.workspaces.write_project_config(a),
        fixture.workspaces.write_project_config(b)
    );
    let error = match (a, b) {
        (Ok(_), Err(error)) | (Err(error), Ok(_)) => error,
        result => panic!("only one CAS save may win: {result:?}"),
    };
    assert_eq!(error.code, ErrorCode::Conflict);
    fixture
        .workspaces
        .remove_project(project.id, &[])
        .await
        .unwrap();
    assert_eq!(
        fixture
            .workspaces
            .read_project_config(project.id)
            .await
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    );
    assert!(project.root.join("vessel.toml").is_file());
    fixture.stop.cancel();
}

#[tokio::test]
async fn cancelled_config_admission_never_detaches_or_writes_after_release() {
    use std::{future::Future as _, task::Context};
    use vessel_proto::registry::{ConfigRevision, ConfigText, WriteProjectConfig};

    let fixture = Fixture::new().await;
    let project = fixture.project("cancelled-config", None).await;
    let admission = fixture.workspaces.admit_project(project.id).await.unwrap();
    let baseline = fixture.tasks.len();
    let request = WriteProjectConfig {
        project: project.id,
        expected: ConfigRevision::Missing,
        text: ConfigText::parse("# cancelled before admission").unwrap(),
    };
    let mut pending = Box::pin(fixture.workspaces.write_project_config(request));
    let mut context = Context::from_waker(std::task::Waker::noop());
    assert!(pending.as_mut().poll(&mut context).is_pending());
    assert_eq!(fixture.tasks.len(), baseline);
    drop(pending);
    drop(admission);

    let after = fixture
        .workspaces
        .read_project_config(project.id)
        .await
        .unwrap();
    assert_eq!(after.revision, ConfigRevision::Missing);
    let saved = fixture
        .workspaces
        .write_project_config(WriteProjectConfig {
            project: project.id,
            expected: ConfigRevision::Missing,
            text: ConfigText::parse("# admitted writer").unwrap(),
        })
        .await
        .unwrap();
    assert_ne!(saved.revision, ConfigRevision::Missing);
    assert_eq!(fixture.tasks.len(), baseline);
    fixture.stop.cancel();
}

#[tokio::test]
async fn directory_capacity_refuses_before_detaching_or_mkdir() {
    use crate::workspace::projects::admission::MAX_PROJECT_ADMISSIONS;
    use std::{future::Future as _, task::Context};

    let fixture = Fixture::new().await;
    let project = fixture.project("directory-capacity", None).await;
    let held = fixture.workspaces.admit_project(project.id).await.unwrap();
    let baseline = fixture.tasks.len();
    let mut pending = Vec::new();
    for _ in 0..MAX_PROJECT_ADMISSIONS.checked_sub(1).unwrap() {
        let mut call = Box::pin(fixture.workspaces.admit_project(project.id));
        let mut context = Context::from_waker(std::task::Waker::noop());
        assert!(call.as_mut().poll(&mut context).is_pending());
        pending.push(call);
    }
    let name = "not-created-at-capacity";
    let result = fixture
        .workspaces
        .create_project_directory(CreateProjectDirectory {
            parent: fixture.path(),
            name: name.to_owned().try_into().unwrap(),
        })
        .await;
    assert_eq!(result.unwrap_err().code, ErrorCode::Conflict);
    assert_eq!(fixture.tasks.len(), baseline);
    assert!(!fixture.path().join(name).exists());
    drop(pending);
    drop(held);
    let result = fixture
        .workspaces
        .create_project_directory(CreateProjectDirectory {
            parent: fixture.path(),
            name: name.to_owned().try_into().unwrap(),
        })
        .await
        .unwrap();
    assert!(matches!(result, ProjectDirectoryOutcome::Created { .. }));
    fixture.stop.cancel();
}
