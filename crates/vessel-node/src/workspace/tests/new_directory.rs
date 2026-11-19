//! ADR 0113 in the node: `project.create_directory` makes one empty
//! folder a project without Git, as Paseo's New directory, and a leading
//! `~` in its parent, `project.add`'s path and `project.clone`'s parent
//! is the node user's home.

use vessel_proto::registry::DirectoryName;
use vessel_proto::workspace::ProjectState;

use super::{Fixture, WorkspaceError};

fn name(text: &str) -> DirectoryName {
    DirectoryName::parse(text).unwrap()
}

#[tokio::test]
async fn a_new_directory_is_one_empty_folder_and_a_project_without_git() {
    let fixture = Fixture::new().await;
    let parent = fixture.path().join("code");
    std::fs::create_dir(&parent).unwrap();
    let made = fixture
        .workspaces
        .create_project_directory(parent.clone(), name("새 프로젝트"))
        .await
        .unwrap();
    let folder = parent.join("새 프로젝트");
    assert_eq!(made.root, folder);
    assert!(!made.git);
    assert!(
        matches!(made.state, ProjectState::Ready),
        "{:?}",
        made.state
    );
    assert_eq!(made.name.as_str(), "새 프로젝트");
    assert!(folder.is_dir());
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 0);
    assert_eq!(fixture.workspaces.list_projects().await.projects.len(), 1);

    // The same name again: it exists.
    let again = fixture
        .workspaces
        .create_project_directory(parent.clone(), name("새 프로젝트"))
        .await
        .unwrap_err();
    assert!(matches!(again, WorkspaceError::Conflict(_)), "{again:?}");
    assert_eq!(fixture.workspaces.list_projects().await.projects.len(), 1);
}

#[tokio::test]
async fn a_new_directory_needs_an_existing_parent_and_an_allowed_root() {
    let fixture = Fixture::new().await;
    // No parent: nothing is made, parents included.
    let missing = fixture.path().join("absent");
    let error = fixture
        .workspaces
        .create_project_directory(missing.clone(), name("app"))
        .await
        .unwrap_err();
    assert!(matches!(error, WorkspaceError::Invalid(_)), "{error:?}");
    assert!(!missing.exists());
    // A relative parent, and `~user`, are refused.
    for relative in ["code", "~user"] {
        let error = fixture
            .workspaces
            .create_project_directory(relative.into(), name("app"))
            .await
            .unwrap_err();
        assert!(
            matches!(error, WorkspaceError::Invalid(_)),
            "{relative}: {error:?}"
        );
    }
    // Inside the node's data folder: refused before anything is made.
    let data = fixture.path().join("home").join(".vessel");
    let error = fixture
        .workspaces
        .create_project_directory(data.clone(), name("app"))
        .await
        .unwrap_err();
    assert!(matches!(error, WorkspaceError::Invalid(_)), "{error:?}");
    assert!(!data.join("app").exists());
    assert!(fixture.workspaces.list_projects().await.projects.is_empty());
}

#[tokio::test]
async fn a_new_directory_s_parent_may_be_the_home_as_tilde() {
    let fixture = Fixture::new().await;
    let made = fixture
        .workspaces
        .create_project_directory("~".into(), name("notes"))
        .await
        .unwrap();
    let home = std::fs::canonicalize(fixture.path().join("home")).unwrap();
    assert_eq!(made.root, home.join("notes"));
    std::fs::create_dir(home.join("dev")).unwrap();
    let nested = fixture
        .workspaces
        .create_project_directory("~/dev".into(), name("app"))
        .await
        .unwrap();
    assert_eq!(nested.root, home.join("dev").join("app"));
}

#[tokio::test]
async fn an_added_path_may_start_with_tilde_but_not_tilde_user() {
    let fixture = Fixture::new().await;
    let home = std::fs::canonicalize(fixture.path().join("home")).unwrap();
    std::fs::create_dir(home.join("shop")).unwrap();
    let added = fixture
        .workspaces
        .add_project("~/shop".into(), None)
        .await
        .unwrap();
    assert_eq!(added.root, home.join("shop"));
    let error = fixture
        .workspaces
        .add_project("~user/shop".into(), None)
        .await
        .unwrap_err();
    assert!(matches!(error, WorkspaceError::Invalid(_)), "{error:?}");
}

#[tokio::test]
async fn a_clone_s_parent_may_be_the_home_as_tilde() {
    let fixture = Fixture::new().await;
    let home = std::fs::canonicalize(fixture.path().join("home")).unwrap();
    let url = "https://127.0.0.1:1/nothing.git".parse().unwrap();
    let cloning = fixture
        .workspaces
        .clone_project(url, "~".into(), None, None)
        .await
        .unwrap();
    assert_eq!(cloning.root, home.join("nothing"));
    // Let the clone end (it cannot connect) so nothing outlives the test.
    let mut info = cloning;
    for _round in 0..30 {
        if !matches!(info.state, ProjectState::Cloning { .. }) {
            break;
        }
        info = fixture
            .workspaces
            .get_project(info.id, Some(info.revision), true)
            .await
            .unwrap();
    }
    assert!(
        matches!(info.state, ProjectState::CloneFailed { .. }),
        "{:?}",
        info.state
    );
}
