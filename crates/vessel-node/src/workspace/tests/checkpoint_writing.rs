//! Real workspace mutations consume the same gate as checkpoint restore.
use super::*;
use crate::workspace::{
    exclusion::{ADMISSION_TIMEOUT, RootRefusal},
    writing::RequiredRoot,
};
use tokio::sync::oneshot;
use vessel_proto::workspace::{RelPath, WriteExpect};
use vessel_workspace::physical::Directory;
fn deadline() -> tokio::time::Instant {
    tokio::time::Instant::now()
        .checked_add(ADMISSION_TIMEOUT)
        .unwrap()
}
async fn finish(fixture: &Fixture) {
    fixture.stop.cancel();
    let _closed = fixture.tasks.close();
    fixture.tasks.wait().await;
    fixture.store.close().await;
}
#[tokio::test]
async fn checkpoint_root_lease_refuses_actual_file_mutations_until_the_owner_releases() {
    let fixture = Fixture::new().await;
    let project = fixture.project("project", None).await;
    let root = WorkspaceRoot::Project {
        project: project.id,
    };
    let physical = Directory::open(&project.root).unwrap();
    let lease = fixture
        .workspaces
        .exclusion
        .claim(physical.identity(), deadline())
        .await
        .unwrap();
    let path = "README.md".parse::<RelPath>().unwrap();
    assert!(matches!(
        fixture
            .workspaces
            .write_file(root, &path, "new".to_owned(), WriteExpect::Any)
            .await,
        Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
    ));
    assert!(matches!(
        fixture
            .workspaces
            .create_dir(root, &"new-dir".parse().unwrap())
            .await,
        Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
    ));
    assert!(matches!(
        fixture
            .workspaces
            .rename_path(root, &path, &"moved".parse().unwrap())
            .await,
        Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
    ));
    assert!(matches!(
        fixture.workspaces.delete_path(root, &path, false).await,
        Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
    ));
    assert_eq!(
        std::fs::read(project.root.join("README.md")).unwrap(),
        b"hello\n"
    );
    drop(lease);
    let _written = fixture
        .workspaces
        .write_file(root, &path, "new".to_owned(), WriteExpect::Any)
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(project.root.join("README.md")).unwrap(),
        b"new"
    );
    finish(&fixture).await;
}
#[tokio::test]
async fn checkpoint_caller_cancellation_keeps_the_admitted_writer_until_real_io_and_postcondition()
{
    let fixture = Fixture::new().await;
    let project = fixture.project("project", None).await;
    let root = WorkspaceRoot::Project {
        project: project.id,
    };
    let physical = Directory::open(&project.root).unwrap();
    let (entered, ready) = oneshot::channel();
    let (release, held) = std::sync::mpsc::channel();
    let workspaces = Arc::clone(&fixture.workspaces);
    let caller = tokio::spawn(async move {
        workspaces
            .run_write(
                root,
                RequiredRoot::Directory,
                move |resolved, _cancellation| async move {
                    tokio::task::spawn_blocking(move || {
                        entered.send(()).unwrap();
                        held.recv().unwrap();
                        std::fs::write(resolved.root.path().join("README.md"), b"retained write")
                            .unwrap();
                    })
                    .await?;
                    Ok(())
                },
            )
            .await
    });
    ready.await.unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert!(matches!(
        fixture
            .workspaces
            .exclusion
            .claim(physical.identity(), deadline())
            .await,
        Err(RootRefusal::Busy)
    ));
    release.send(()).unwrap();
    finish(&fixture).await;
    assert_eq!(
        std::fs::read(project.root.join("README.md")).unwrap(),
        b"retained write"
    );
}

#[tokio::test]
async fn checkpoint_root_lease_refuses_git_index_commit_switch_and_stash_mutations() {
    use crate::workspace::IndexChange;
    use vessel_proto::workspace::{CommitStage, PathSet, StashAction};
    let fixture = Fixture::new().await;
    let project = fixture.project("project", None).await;
    let root = WorkspaceRoot::Project {
        project: project.id,
    };
    let physical = Directory::open(&project.root).unwrap();
    let lease = fixture
        .workspaces
        .exclusion
        .claim(physical.identity(), deadline())
        .await
        .unwrap();
    let paths = PathSet::All;
    assert!(matches!(
        fixture
            .workspaces
            .git_index(root, &paths, IndexChange::Discard { untracked: false })
            .await,
        Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
    ));
    assert!(matches!(
        fixture
            .workspaces
            .git_commit(
                root,
                &"checkpoint refusal".parse().unwrap(),
                CommitStage::Everything
            )
            .await,
        Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
    ));
    assert!(matches!(
        fixture
            .workspaces
            .git_switch(root, &"main".parse().unwrap(), None)
            .await,
        Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
    ));
    assert!(matches!(
        fixture
            .workspaces
            .git_stash(
                root,
                &StashAction::Save {
                    message: None,
                    untracked: false
                }
            )
            .await,
        Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
    ));
    drop(lease);
    let _status = fixture
        .workspaces
        .git_index(root, &paths, IndexChange::Stage)
        .await
        .unwrap();
    finish(&fixture).await;
}
#[tokio::test]
async fn checkpoint_root_lease_refuses_git_network_and_reference_mutations_before_a_process_starts()
{
    let fixture = Fixture::new().await;
    let project = fixture.project("project", None).await;
    let root = WorkspaceRoot::Project {
        project: project.id,
    };
    let physical = Directory::open(&project.root).unwrap();
    let lease = fixture
        .workspaces
        .exclusion
        .claim(physical.identity(), deadline())
        .await
        .unwrap();
    // This fixture has no remotes. Even a broken guard cannot reach a network.
    assert!(matches!(
        fixture.workspaces.git_fetch(root, None).await,
        Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
    ));
    assert!(matches!(
        fixture.workspaces.git_push(root, None, false).await,
        Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
    ));
    assert!(matches!(
        fixture.workspaces.git_pull(root).await,
        Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
    ));
    assert!(matches!(
        fixture
            .workspaces
            .git_rename_branch(root, None, &"other".parse().unwrap())
            .await,
        Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
    ));
    drop(lease);
    finish(&fixture).await;
}
#[tokio::test]
async fn checkpoint_retained_operation_receives_disconnect_cancellation_before_releasing_its_root()
{
    let fixture = Fixture::new().await;
    let project = fixture.project("project", None).await;
    let root = WorkspaceRoot::Project {
        project: project.id,
    };
    let (entered, ready) = oneshot::channel();
    let (cancelled, noticed) = oneshot::channel();
    let workspaces = Arc::clone(&fixture.workspaces);
    let caller = tokio::spawn(async move {
        workspaces
            .run_write(
                root,
                RequiredRoot::Directory,
                move |_resolved, cancellation| async move {
                    entered.send(()).unwrap();
                    cancellation.cancelled().await;
                    cancelled.send(()).unwrap();
                    Ok(())
                },
            )
            .await
    });
    ready.await.unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    noticed.await.unwrap();
    finish(&fixture).await;
}
