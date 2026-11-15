//! Blocking admission probes keep their bounded capacity after client loss.
use super::*;
use crate::workspace::metadata::ROOT_METADATA_WORKERS;
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_cancelled_start_probes_retain_capacity_until_real_metadata_io_finishes() {
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
}

#[tokio::test]
async fn checkpoint_root_lease_refuses_actual_setup_and_teardown_removal() {
    use crate::workspace::exclusion::{ADMISSION_TIMEOUT, RootRefusal};
    use vessel_workspace::physical::Directory;
    let fixture = Fixture::new().await;
    let project = fixture.project("project", Some(SETUP)).await;
    let created = fixture
        .create(project.id, Creator::Owner, SetupChoice::Skip)
        .await;
    let CreateWorktreeOutcome::Created { worktree } = created else {
        panic!("no worktree")
    };
    let ProjectCommands::Listed { digest, .. } =
        fixture.workspaces.commands(project.id, None).await.unwrap()
    else {
        panic!("no commands")
    };
    let _approved = fixture
        .workspaces
        .approve(project.id, digest, true)
        .await
        .unwrap();
    let physical = Directory::open(&worktree.path).unwrap();
    let deadline = tokio::time::Instant::now()
        .checked_add(ADMISSION_TIMEOUT)
        .unwrap();
    let lease = fixture
        .workspaces
        .exclusion
        .claim(physical.identity(), deadline)
        .await
        .unwrap();
    let _running = fixture.workspaces.run_setup(worktree.id).await.unwrap();
    let (setup, log) = fixture.setup_end(worktree.id).await;
    assert!(matches!(
        setup,
        SetupState::Failed {
            failure: vessel_proto::workspace::SetupFailure::Spawn,
            ..
        }
    ));
    assert!(!log.contains("prepared-by-setup"));
    assert!(matches!(
        fixture
            .workspaces
            .remove_worktree(worktree.id, IfLost::Discard, Vec::new)
            .await,
        Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
    ));
    assert!(worktree.path.exists());
    drop(lease);
    let _running = fixture.workspaces.run_setup(worktree.id).await.unwrap();
    let (setup, log) = fixture.setup_end(worktree.id).await;
    assert!(matches!(setup, SetupState::Succeeded { .. }));
    assert!(log.contains("prepared-by-setup"));
    let removed = fixture
        .workspaces
        .remove_worktree(worktree.id, IfLost::Discard, Vec::new)
        .await
        .unwrap();
    assert!(matches!(removed, RemoveWorktreeOutcome::Removed { .. }));
    assert!(!worktree.path.exists());
    fixture.stop.cancel();
    let _closed = fixture.tasks.close();
    fixture.tasks.wait().await;
    fixture.store.close().await;
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
}
