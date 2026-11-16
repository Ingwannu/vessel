//! Runtime workspace registration over the wire (ADR 0106): adding a project
//! registers nothing; a session started at the project's root registers one
//! `Local` workspace with the session as a derived member; node start
//! registers the roots of sessions stored before registration existed.

use std::path::{Path, PathBuf};
use std::time::Duration;

use vessel_client::NodeClient;
use vessel_proto::registry::{
    GetRegistry, RegistrySnapshot, WorkspaceBacking, WorkspaceDescriptor, WorkspaceLifecycle,
    WorkspaceResource,
};
use vessel_proto::workspace::{AddProject, ProjectInfo, RemoveProject, SessionPlace};
use vessel_proto::{SessionId, SessionInfo, WorkspaceId};

use crate::support::{Fixture, within};
use crate::workspace::{in_place, lookup, repository};

async fn registry(client: &NodeClient) -> RegistrySnapshot {
    client
        .call(GetRegistry { changed_from: None })
        .await
        .unwrap()
}

fn member(snapshot: &RegistrySnapshot, session: SessionId) -> Option<WorkspaceId> {
    snapshot.owner(WorkspaceResource::Session { session })
}

/// The only workspace in `snapshot`, which must be `project`'s root.
fn only_root(snapshot: &RegistrySnapshot, project: &ProjectInfo) -> WorkspaceDescriptor {
    let [workspace] = snapshot.workspaces.as_slice() else {
        panic!("one workspace per root, found {:?}", snapshot.workspaces);
    };
    assert_eq!(workspace.project, project.id);
    assert_eq!(workspace.path, project.root);
    assert_eq!(workspace.backing, WorkspaceBacking::Local);
    assert_eq!(workspace.lifecycle, WorkspaceLifecycle::Active);
    workspace.clone()
}

async fn add(client: &NodeClient, root: &Path) -> ProjectInfo {
    repository(root);
    client
        .call(AddProject {
            path: root.to_path_buf(),
            name: None,
        })
        .await
        .unwrap()
}

async fn started(fixture: &Fixture, client: &NodeClient, project: &ProjectInfo) -> SessionInfo {
    let place = SessionPlace::Project {
        project: project.id,
        dir: None,
    };
    client
        .create_session(in_place(fixture, place))
        .await
        .unwrap()
}

/// Kill `session` and wait until it has ended, so it no longer holds its
/// project (`session.kill` only signals).
async fn ended(client: &NodeClient, session: SessionId) {
    client.kill(session).await.unwrap();
    while client.session(session).await.unwrap().status.is_live() {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// The home as a node from before registration left it: placed sessions
/// and no registry record.
fn forget_registry(fixture: &Fixture) {
    let database = rusqlite::Connection::open(fixture.data_dir().database_path()).unwrap();
    database.busy_timeout(Duration::from_secs(10)).unwrap();
    let _deleted = database
        .execute("DELETE FROM workspace_registry", [])
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_at_a_project_root_registers_one_workspace_with_it_as_member() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.harness_lookup = lookup(&fixture))
            .await;
        let owner = fixture.connect(&node).await;
        let folder = tempfile::tempdir().unwrap();
        let root: PathBuf = std::fs::canonicalize(folder.path()).unwrap();
        let project = add(&owner, &root).await;
        // Adding a project registers no workspace (Paseo TRACKER 239).
        assert!(registry(&owner).await.workspaces.is_empty());

        let first = started(&fixture, &owner, &project).await;
        let second = started(&fixture, &owner, &project).await;
        let snapshot = registry(&owner).await;
        let workspace = only_root(&snapshot, &project);
        assert_eq!(member(&snapshot, first.id), Some(workspace.id));
        assert_eq!(member(&snapshot, second.id), Some(workspace.id));

        ended(&owner, first.id).await;
        ended(&owner, second.id).await;
        // Removal archives the workspace and keeps it; the snapshot still
        // reads with its project gone.
        let _ack = owner
            .call(RemoveProject {
                project: project.id,
            })
            .await
            .unwrap();
        let snapshot = registry(&owner).await;
        let [kept] = snapshot.workspaces.as_slice() else {
            panic!("the archived workspace is kept, found {:?}", snapshot.workspaces);
        };
        assert_eq!(kept.id, workspace.id);
        assert!(matches!(kept.lifecycle, WorkspaceLifecycle::Archived { .. }));
        assert_eq!(member(&snapshot, first.id), None);
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn node_start_registers_the_roots_of_stored_sessions_and_skips_missing_ones() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.harness_lookup = lookup(&fixture))
            .await;
        let owner = fixture.connect(&node).await;
        let folder = tempfile::tempdir().unwrap();
        let base: PathBuf = std::fs::canonicalize(folder.path()).unwrap();
        let (kept_root, gone_root) = (base.join("kept"), base.join("gone"));
        std::fs::create_dir(&kept_root).unwrap();
        std::fs::create_dir(&gone_root).unwrap();
        let kept = add(&owner, &kept_root).await;
        let gone = add(&owner, &gone_root).await;
        let earliest = started(&fixture, &owner, &kept).await;
        let later = started(&fixture, &owner, &kept).await;
        let orphan = started(&fixture, &owner, &gone).await;
        for session in [earliest.id, later.id, orphan.id] {
            ended(&owner, session).await;
        }
        node.stop().await.unwrap();

        forget_registry(&fixture);
        std::fs::remove_dir_all(&gone_root).unwrap();
        let node = fixture
            .start_with(|config| config.harness_lookup = lookup(&fixture))
            .await;
        let owner = fixture.connect(&node).await;
        let snapshot = registry(&owner).await;
        // Created at its earliest session's time, as Paseo's bootstrap does.
        let workspace = only_root(&snapshot, &kept);
        assert_eq!(workspace.created_at, earliest.created_at);
        assert_eq!(member(&snapshot, earliest.id), Some(workspace.id));
        assert_eq!(member(&snapshot, later.id), Some(workspace.id));
        assert_eq!(member(&snapshot, orphan.id), None);
        node.stop().await.unwrap();
    })
    .await;
}
