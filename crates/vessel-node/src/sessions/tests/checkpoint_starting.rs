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
    assert_eq!(fixture.runtime.list().len(), 1);
    drop(workspaces.admit_start(fixture.data.path()).await.unwrap());
    stop.cancel();
    fixture.runtime.shutdown().await;
    fixture.settle().await;
    fixture.stop().await;
}
