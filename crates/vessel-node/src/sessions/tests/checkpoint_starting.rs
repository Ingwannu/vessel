//! Physical admission is checked by the real retained session creation path.
use super::*;
use crate::workspace::{Opening, WorkspaceError, Workspaces, exclusion::RootRefusal};

async fn workspace(fixture: &Fixture) -> (Arc<Workspaces>, CancellationToken) {
    let data = DataDir::new(fixture.data.path());
    let stop = CancellationToken::new();
    let workspaces = Workspaces::start(
        Opening {
            data: data.root().to_path_buf(),
            workspace: data.prepare_workspace_dir().unwrap(),
            home: fixture.data.path().to_path_buf(),
            git: None,
            git_env: Vec::new(),
            removed: crate::TerminalsConfig::default().removed.into(),
        },
        fixture.store.clone(),
        fixture.tasks.clone(),
        stop.clone(),
    )
    .await
    .unwrap();
    (workspaces, stop)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_session_start_refuses_a_replaced_folder_before_spawning() {
    let fixture = Fixture::new().await;
    let (workspaces, stop) = workspace(&fixture).await;
    let folder = fixture.data.path().join("launch");
    std::fs::create_dir(&folder).unwrap();
    let root = workspaces.admit_start(&folder).await.unwrap();
    std::fs::rename(&folder, fixture.data.path().join("original")).unwrap();
    std::fs::create_dir(&folder).unwrap();
    let result = fixture
        .sessions
        .create(
            fixture.spec().with_cwd(folder),
            LaunchOptions::default(),
            None,
            super::super::Admission::new(root, None),
        )
        .await;
    assert!(
        matches!(result, Err(ServiceError::StartRoot(_))),
        "{result:?}"
    );
    assert!(fixture.runtime.list().is_empty());
    stop.cancel();
    fixture.settle().await;
    fixture.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_session_admission_releases_after_registration_or_failed_store_cleanup() {
    let fixture = Fixture::new().await;
    let (workspaces, stop) = workspace(&fixture).await;
    let root = workspaces.admit_start(fixture.data.path()).await.unwrap();
    assert!(matches!(
        workspaces.admit_start(fixture.data.path()).await,
        Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
    ));
    let info = fixture
        .sessions
        .create(
            fixture.spec(),
            LaunchOptions::default(),
            None,
            super::super::Admission::new(root, None),
        )
        .await
        .unwrap();
    assert!(matches!(
        fixture.sessions.find(info.id).await.unwrap(),
        Found::Live { .. }
    ));
    let mut ends = fixture.sessions.ends();
    fixture.runtime.get(info.id).unwrap().kill();
    assert_eq!(ends.recv().await.unwrap(), info.id);
    let root = workspaces.admit_start(fixture.data.path()).await.unwrap();
    fixture.store.close().await;
    let result = fixture
        .sessions
        .create(
            fixture.spec(),
            LaunchOptions::default(),
            None,
            super::super::Admission::new(root, None),
        )
        .await;
    assert!(matches!(result, Err(ServiceError::Store(_))), "{result:?}");
    // The failed creation has ended and been removed before its guard lets go.
    assert!(fixture.runtime.list().is_empty());
    drop(workspaces.admit_start(fixture.data.path()).await.unwrap());
    stop.cancel();
    fixture.runtime.shutdown().await;
    fixture.settle().await;
    fixture.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_capture_uses_launch_physical_identity_and_refuses_a_shared_live_root() {
    use vessel_proto::checkpoint::CheckpointError;
    let fixture = Fixture::new().await;
    let (workspaces, stop) = workspace(&fixture).await;
    let folder = fixture.data.path().join("capture-root");
    std::fs::create_dir(&folder).unwrap();
    let admitted = workspaces.admit_start(&folder).await.unwrap();
    let physical = admitted.physical();
    let first = fixture
        .sessions
        .create(
            fixture.structured().with_cwd(folder.clone()),
            LaunchOptions::default(),
            None,
            super::super::Admission::new(admitted, None),
        )
        .await
        .unwrap();
    assert_eq!(
        fixture.sessions.checkpoint_root(first.id).unwrap(),
        physical
    );
    let admitted = workspaces.admit_start(&folder).await.unwrap();
    let second = fixture
        .sessions
        .create(
            fixture.structured().with_cwd(folder.clone()),
            LaunchOptions::default(),
            None,
            super::super::Admission::new(admitted, None),
        )
        .await
        .unwrap();
    assert_eq!(
        fixture.sessions.checkpoint_root(first.id),
        Err(CheckpointError::UnsupportedScope)
    );
    let second = fixture.runtime.get(second.id).unwrap();
    second.kill();
    second.ended().await;
    assert_eq!(
        fixture.sessions.checkpoint_root(first.id).unwrap(),
        physical
    );
    std::fs::rename(&folder, fixture.data.path().join("old-capture-root")).unwrap();
    std::fs::create_dir(&folder).unwrap();
    let replacement = workspaces.admit_start(&folder).await.unwrap();
    assert_ne!(replacement.physical(), physical);
    assert_eq!(
        fixture.sessions.checkpoint_root(first.id).unwrap(),
        physical,
        "a reused cwd string cannot rebind the agent's actual launch root"
    );
    drop(replacement);
    let first = fixture.runtime.get(first.id).unwrap();
    first.kill();
    first.ended().await;
    stop.cancel();
    fixture.settle().await;
    fixture.stop().await;
}
