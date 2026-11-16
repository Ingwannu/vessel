//! Real file-writer and metadata barriers, with retained failure cleanup.
use super::root_case::{phase, run};
use super::*;
use crate::workspace::{exclusion::RootRefusal, metadata::ROOT_METADATA_WORKERS};
use vessel_proto::workspace::WriteExpect;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_cancelled_start_probes_retain_capacity_until_real_metadata_io_finishes() {
    run(async |fixture, held| {
        for _worker in 0..ROOT_METADATA_WORKERS {
            let workspaces = Arc::clone(&fixture.workspaces);
            let (entered, ready) = tokio::sync::oneshot::channel();
            let (release, blocked) = std::sync::mpsc::channel();
            let caller = tokio::spawn(async move {
                workspaces.root_metadata(move || {
                    let _gone = entered.send(());
                    let _released = blocked.recv();
                    Ok(())
                }).await
            });
            held.retain(release, caller);
            phase("metadata worker entry", ready).await.unwrap();
        }
        held.abort_callers().await;
        // This must refuse before canonicalization or any metadata permit. The
        // saturated pool makes an accidental ambient-cwd lookup observable.
        assert!(matches!(phase("relative input", fixture.workspaces.admit_start(Path::new("."))).await,
            Err(WorkspaceError::Directory(error)) if error.kind() == std::io::ErrorKind::InvalidInput));
        assert!(matches!(phase("metadata capacity", fixture.workspaces.root_metadata(|| Ok(()))).await, Err(WorkspaceError::MetadataBusy)));
        fixture.stop.cancel();
        assert!(matches!(phase("stopped metadata", fixture.workspaces.root_metadata(|| Ok(()))).await, Err(WorkspaceError::Stopping)));
    }).await;
}

fn opened_root(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    std::fs::read_dir("/proc/self/fd")
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|entry| std::fs::metadata(entry.path()).ok())
        .any(|opened| opened.dev() == metadata.dev() && opened.ino() == metadata.ino())
}

async fn replaced_root(fixture: &Fixture, root: &Path) {
    let nested = root.join("nested");
    std::fs::create_dir(&nested).unwrap();
    let original = std::fs::metadata(root).unwrap();
    assert!(
        !opened_root(&original),
        "fixture must not already pin this inode"
    );
    let alias = fixture.path().join("selected-alias");
    std::os::unix::fs::symlink(root, &alias).unwrap();
    let guard = phase(
        "nested parent alias admission",
        fixture.workspaces.admit_start(&alias.join("nested")),
    )
    .await
    .unwrap();
    assert!(
        opened_root(&original),
        "nested cwd must pin the selected ancestor"
    );
    // Remove only private fixture backing. Keep the exact ancestor FD pinned
    // until the guard releases, even after its name is replaced.
    std::fs::remove_dir_all(root).unwrap();
    assert!(opened_root(&original));
    std::fs::create_dir(root).unwrap();
    assert!(
        phase("replacement refusal", guard.revalidate())
            .await
            .is_err()
    );
    drop(guard);
    assert!(!opened_root(&original));
}

#[tokio::test]
async fn checkpoint_root_lease_pins_the_managed_ancestor_fd_for_a_nested_start() {
    run(async |fixture, _held| {
        let project = phase("project setup", fixture.project("project", None)).await;
        let CreateWorktreeOutcome::Created { worktree } = phase(
            "worktree setup",
            fixture.create(project.id, Creator::Owner, SetupChoice::Skip),
        )
        .await
        else {
            panic!("no managed worktree")
        };
        replaced_root(fixture, &worktree.path).await;
    })
    .await;
}

#[tokio::test]
async fn checkpoint_root_lease_pins_the_project_ancestor_fd_for_a_nested_start() {
    run(async |fixture, _held| {
        let project = phase("project setup", fixture.project("project", None)).await;
        replaced_root(fixture, &project.root).await;
    })
    .await;
}

#[tokio::test]
async fn checkpoint_project_nested_and_parent_alias_starts_refuse_the_real_project_writer() {
    run(async |fixture, _held| {
        let project = phase("project setup", fixture.project("project", None)).await;
        let root = WorkspaceRoot::Project {
            project: project.id,
        };
        let nested = project.root.join("subdir");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(nested.join("file"), b"original").unwrap();
        let alias = fixture.path().join("alias");
        std::os::unix::fs::symlink(&project.root, &alias).unwrap();
        let path = "subdir/file".parse().unwrap();
        for cwd in [project.root.clone(), nested.clone(), alias.join("subdir")] {
            let guard = phase(
                "nested or alias admission",
                fixture.workspaces.admit_start(&cwd),
            )
            .await
            .unwrap();
            assert!(matches!(
                phase(
                    "project writer refusal",
                    fixture
                        .workspaces
                        .write_file(root, &path, "new".to_owned(), WriteExpect::Any)
                )
                .await,
                Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
            ));
            assert_eq!(std::fs::read(nested.join("file")).unwrap(), b"original");
            drop(guard);
        }
        let _written = phase(
            "released project writer",
            fixture
                .workspaces
                .write_file(root, &path, "new".to_owned(), WriteExpect::Any),
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(nested.join("file")).unwrap(), b"new");
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_root_admission_rpc_loss_retains_actual_file_io_and_postconditions() {
    run(async |fixture, held| {
        let project = phase("project setup", fixture.project("project", None)).await;
        let root = WorkspaceRoot::Project {
            project: project.id,
        };
        let workspaces = Arc::clone(&fixture.workspaces);
        let (entered, ready) = tokio::sync::oneshot::channel();
        let (release, blocked) = std::sync::mpsc::channel();
        let caller = tokio::spawn(async move {
            workspaces
                .run_file_write(root, move |resolved| async move {
                    tokio::task::spawn_blocking(move || {
                        let _gone = entered.send(());
                        let _released = blocked.recv();
                        std::fs::write(resolved.root.path().join("README.md"), b"retained")
                            .unwrap();
                    })
                    .await
                    .map_err(WorkspaceError::project_task)?;
                    Ok(())
                })
                .await
        });
        held.retain(release, caller);
        phase("file worker entry", ready).await.unwrap();
        held.abort_callers().await;
        assert!(matches!(
            phase(
                "retained writer refusal",
                fixture.workspaces.admit_start(&project.root)
            )
            .await,
            Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
        ));
        // Verify the actual file postcondition after the retained task drains,
        // even if its original reply receiver was cancelled.
        held.release();
        fixture.stop.cancel();
        let _closed = fixture.tasks.close();
        phase("retained writer drain", fixture.tasks.wait()).await;
        assert_eq!(
            std::fs::read(project.root.join("README.md")).unwrap(),
            b"retained"
        );
    })
    .await;
}

#[tokio::test]
async fn checkpoint_nested_registration_cannot_split_an_existing_physical_lease() {
    run(async |fixture, _held| {
        let parent = fixture.path().join("parent");
        let child = parent.join("child");
        std::fs::create_dir_all(child.join("deeper")).unwrap();
        std::fs::write(child.join("file"), b"original").unwrap();
        let child_project = phase(
            "child project setup",
            fixture.workspaces.add_project(child.clone(), None),
        )
        .await
        .unwrap();
        let guard = phase(
            "child admission",
            fixture.workspaces.admit_start(&child.join("deeper")),
        )
        .await
        .unwrap();
        let parent_project = phase(
            "parent project setup",
            fixture.workspaces.add_project(parent.clone(), None),
        )
        .await
        .unwrap();
        let root = WorkspaceRoot::Project {
            project: parent_project.id,
        };
        let path = "child/file".parse().unwrap();
        assert!(matches!(
            phase(
                "new parent writer refusal",
                fixture
                    .workspaces
                    .write_file(root, &path, "new".to_owned(), WriteExpect::Any)
            )
            .await,
            Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
        ));
        assert_eq!(std::fs::read(child.join("file")).unwrap(), b"original");
        drop(guard);
        let outer = phase("parent admission", fixture.workspaces.admit_start(&parent))
            .await
            .unwrap();
        let child_root = WorkspaceRoot::Project {
            project: child_project.id,
        };
        assert!(matches!(
            phase(
                "nested project writer refusal",
                fixture.workspaces.write_file(
                    child_root,
                    &"file".parse().unwrap(),
                    "new".to_owned(),
                    WriteExpect::Any
                )
            )
            .await,
            Err(WorkspaceError::RootAdmission(RootRefusal::Busy))
        ));
        drop(outer);
        let _written = phase(
            "released nested writer",
            fixture
                .workspaces
                .write_file(root, &path, "new".to_owned(), WriteExpect::Any),
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(child.join("file")).unwrap(), b"new");
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_failed_assertion_still_releases_and_joins_actual_metadata_worker() {
    use futures_util::FutureExt as _;
    let (finished, completed) = tokio::sync::oneshot::channel();
    let outcome = std::panic::AssertUnwindSafe(run(async |fixture, held| {
        let workspaces = Arc::clone(&fixture.workspaces);
        let (entered, ready) = tokio::sync::oneshot::channel();
        let (release, blocked) = std::sync::mpsc::channel();
        let caller = tokio::spawn(async move {
            workspaces
                .root_metadata(move || {
                    let _gone = entered.send(());
                    blocked.recv().unwrap();
                    finished.send(()).unwrap();
                    Ok(())
                })
                .await
        });
        held.retain(release, caller);
        phase("injected-failure worker entry", ready).await.unwrap();
        panic!("original injected assertion failure");
    }))
    .catch_unwind()
    .await;
    assert!(outcome.is_err(), "the original failure must be propagated");
    phase("actual worker completion receipt", completed)
        .await
        .unwrap();
}
