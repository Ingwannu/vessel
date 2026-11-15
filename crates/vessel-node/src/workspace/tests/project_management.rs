//! Selected roots remain independent and registration is durable before publication.

use super::Fixture;
use crate::workspace::WorkspaceError;

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
