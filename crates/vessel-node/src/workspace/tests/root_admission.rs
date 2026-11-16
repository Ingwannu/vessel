//! Blocking admission probes keep their bounded capacity after client loss.
use super::*;
const ROOT_TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
use crate::workspace::metadata::ROOT_METADATA_WORKERS;
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_cancelled_start_probes_retain_capacity_until_real_metadata_io_finishes() {
    vessel_testkit::within(ROOT_TEST_TIMEOUT, async {
        let fixture = Fixture::new().await;
        let mut callers = Vec::new();
        let mut releases = Vec::new();
        for _worker in 0..ROOT_METADATA_WORKERS {
            let workspaces = Arc::clone(&fixture.workspaces);
            let (entered, ready) = tokio::sync::oneshot::channel();
            let (release, held) = std::sync::mpsc::channel();
            callers.push(tokio::spawn(async move {
                workspaces
                    .root_metadata(move || {
                        entered.send(()).unwrap();
                        held.recv().unwrap();
                        Ok(())
                    })
                    .await
            }));
            ready.await.unwrap();
            releases.push(release);
        }
        for caller in callers {
            caller.abort();
            assert!(caller.await.unwrap_err().is_cancelled());
        }
        assert!(matches!(
            fixture.workspaces.root_metadata(|| Ok(())).await,
            Err(WorkspaceError::MetadataBusy)
        ));
        fixture.stop.cancel();
        assert!(matches!(
            fixture.workspaces.root_metadata(|| Ok(())).await,
            Err(WorkspaceError::Stopping)
        ));
        for release in releases {
            release.send(()).unwrap();
        }
        let _closed = fixture.tasks.close();
        fixture.tasks.wait().await;
        fixture.store.close().await;
    })
    .await
    .unwrap();
}

fn opened_root(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    std::fs::read_dir("/proc/self/fd")
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|entry| std::fs::metadata(entry.path()).ok())
        .any(|opened| opened.dev() == metadata.dev() && opened.ino() == metadata.ino())
}

#[tokio::test]
async fn checkpoint_root_lease_pins_the_managed_ancestor_fd_for_a_nested_start() {
    vessel_testkit::within(ROOT_TEST_TIMEOUT, async {
        let fixture = Fixture::new().await;
        let project = fixture.project("project", None).await;
        let CreateWorktreeOutcome::Created { worktree } = fixture
            .create(project.id, Creator::Owner, SetupChoice::Skip)
            .await
        else {
            panic!("no managed worktree")
        };
        let nested = worktree.path.join("nested");
        std::fs::create_dir(&nested).unwrap();
        let original = std::fs::metadata(&worktree.path).unwrap();
        assert!(
            !opened_root(&original),
            "fixture must not already pin this inode"
        );
        let guard = fixture.workspaces.admit_start(&nested).await.unwrap();
        assert!(
            opened_root(&original),
            "nested cwd alone does not pin its managed ancestor"
        );
        // Delete only this private fixture's backing. The opened root must remain
        // pinned even with no name, preventing inode reuse until the lease releases.
        std::fs::remove_dir_all(&worktree.path).unwrap();
        assert!(opened_root(&original));
        std::fs::create_dir(&worktree.path).unwrap();
        assert!(
            guard.revalidate().await.is_err(),
            "a replacement cannot inherit the old lease"
        );
        drop(guard);
        assert!(!opened_root(&original));
        fixture.stop.cancel();
        let _closed = fixture.tasks.close();
        fixture.tasks.wait().await;
        fixture.store.close().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn checkpoint_root_admission_refuses_the_real_file_writer_then_releases_it() {
    vessel_testkit::within(ROOT_TEST_TIMEOUT, async {
        use crate::workspace::exclusion::RootRefusal;
        use vessel_proto::workspace::WriteExpect;
        let fixture = Fixture::new().await;
        let project = fixture.project("project", None).await;
        let root = WorkspaceRoot::Project {
            project: project.id,
        };
        let guard = fixture.workspaces.admit_start(&project.root).await.unwrap();
        let path = "README.md".parse().unwrap();
        assert!(matches!(
            fixture
                .workspaces
                .write_file(root, &path, "new".to_owned(), WriteExpect::Any)
                .await,
            Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
        ));
        assert_eq!(
            std::fs::read(project.root.join("README.md")).unwrap(),
            b"hello\n"
        );
        drop(guard);
        let _written = fixture
            .workspaces
            .write_file(root, &path, "new".to_owned(), WriteExpect::Any)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(project.root.join("README.md")).unwrap(),
            b"new"
        );
        fixture.stop.cancel();
        let _closed = fixture.tasks.close();
        fixture.tasks.wait().await;
        fixture.store.close().await;
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_root_admission_rpc_loss_retains_actual_file_io_and_postconditions() {
    vessel_testkit::within(ROOT_TEST_TIMEOUT, async {
        use crate::workspace::exclusion::RootRefusal;
        let fixture = Fixture::new().await;
        let project = fixture.project("project", None).await;
        let root = WorkspaceRoot::Project {
            project: project.id,
        };
        let workspaces = Arc::clone(&fixture.workspaces);
        let (entered, ready) = tokio::sync::oneshot::channel();
        let (release, held) = std::sync::mpsc::channel();
        let caller = tokio::spawn(async move {
            workspaces
                .run_file_write(root, move |resolved| async move {
                    tokio::task::spawn_blocking(move || {
                        entered.send(()).unwrap();
                        held.recv().unwrap();
                        std::fs::write(resolved.root.path().join("README.md"), b"retained")
                            .unwrap();
                    })
                    .await
                    .map_err(WorkspaceError::project_task)?;
                    Ok(())
                })
                .await
        });
        ready.await.unwrap();
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
        assert!(matches!(
            fixture.workspaces.admit_start(&project.root).await,
            Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
        ));
        release.send(()).unwrap();
        fixture.stop.cancel();
        let _closed = fixture.tasks.close();
        fixture.tasks.wait().await;
        fixture.store.close().await;
        assert_eq!(
            std::fs::read(project.root.join("README.md")).unwrap(),
            b"retained"
        );
    })
    .await
    .unwrap();
}
