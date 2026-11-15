//! Actual disk calls inside the one retained file owner, with no TTL sleeps.
use super::*;
use std::collections::BTreeSet;
use vessel_proto::workspace::{
    CompareGit, GitCompared, GitComparison, GitReadReason, GitWhitespace, RelPath, WriteExpect,
};
use vessel_workspace::files;

const DEADLINE: std::time::Duration = std::time::Duration::from_secs(60);
fn query(project: ProjectId) -> CompareGit {
    CompareGit {
        root: WorkspaceRoot::Project { project },
        comparison: GitComparison::Uncommitted,
        whitespace: GitWhitespace::Show,
        reason: GitReadReason::View,
    }
}
async fn changed(fixture: &Fixture, query: CompareGit) -> BTreeSet<RelPath> {
    let GitCompared::Ready { data } = fixture.workspaces.git_compare(query).await.unwrap() else {
        panic!("fixture comparison must be ready");
    };
    data.files.into_iter().map(|file| file.path).collect()
}
async fn drain(fixture: &Fixture) {
    fixture.stop.cancel();
    assert!(fixture.tasks.close());
    fixture.tasks.wait().await;
}

#[tokio::test]
async fn legacy_file_mutations_invalidate_the_actual_cached_working_tree() {
    vessel_testkit::within(DEADLINE, async {
        let fixture = Fixture::new().await;
        let project = fixture.project("file-cache", None).await;
        let query = query(project.id);
        assert!(changed(&fixture, query.clone()).await.is_empty());
        assert_eq!(fixture.workspaces.comparisons.cached_payloads_for_test(), 1);
        fixture
            .workspaces
            .create_dir(query.root, &"folder".parse().unwrap())
            .await
            .unwrap();
        assert_eq!(fixture.workspaces.comparisons.cached_payloads_for_test(), 0);
        assert!(changed(&fixture, query.clone()).await.is_empty());
        let old: RelPath = "before.txt".parse().unwrap();
        let new: RelPath = "after.txt".parse().unwrap();
        fixture
            .workspaces
            .write_file(
                query.root,
                &old,
                "new data\n".to_owned(),
                WriteExpect::Absent,
            )
            .await
            .unwrap();
        assert_eq!(fixture.workspaces.comparisons.cached_payloads_for_test(), 0);
        assert_eq!(
            changed(&fixture, query.clone()).await,
            BTreeSet::from([old.clone()])
        );
        fixture
            .workspaces
            .rename_path(query.root, &old, &new)
            .await
            .unwrap();
        assert_eq!(fixture.workspaces.comparisons.cached_payloads_for_test(), 0);
        assert_eq!(
            changed(&fixture, query.clone()).await,
            BTreeSet::from([new.clone()])
        );
        fixture
            .workspaces
            .delete_path(query.root, &new, false)
            .await
            .unwrap();
        assert_eq!(fixture.workspaces.comparisons.cached_payloads_for_test(), 0);
        assert!(changed(&fixture, query).await.is_empty());
        drain(&fixture).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn an_owned_file_task_keeps_invalidation_until_physical_completion_after_waiter_loss() {
    vessel_testkit::within(DEADLINE, async {
        let fixture = Fixture::new().await;
        let project = fixture.project("owned-file-cache", None).await;
        let query = query(project.id);
        assert!(changed(&fixture, query.clone()).await.is_empty());
        let (entered, start) = tokio::sync::oneshot::channel();
        let (release, waiting) = tokio::sync::oneshot::channel::<()>();
        let path: RelPath = "physical.txt".parse().unwrap();
        let written = path.clone();
        let work = fixture
            .workspaces
            .mutating_file(query.root, move |opened| async move {
                entered.send(()).unwrap();
                waiting.await.unwrap();
                files::write(
                    &opened,
                    &written,
                    "physical data\n".to_owned(),
                    WriteExpect::Absent,
                )
                .await
            });
        let waiter = fixture.workspaces.spawn_file_work(work);
        start.await.unwrap();
        assert_eq!(fixture.workspaces.comparisons.cached_payloads_for_test(), 0);
        // A genuine comparison during the held operation can cache pre-edit data.
        assert!(changed(&fixture, query.clone()).await.is_empty());
        assert_eq!(fixture.workspaces.comparisons.cached_payloads_for_test(), 1);
        drop(waiter);
        assert!(!fixture.tasks.is_empty());
        assert!(!project.root.join(path.as_str()).exists());
        release.send(()).unwrap();
        // Close/drain/reopen is a test barrier, not a product shutdown or a sleep.
        assert!(fixture.tasks.close());
        fixture.tasks.wait().await;
        assert!(fixture.tasks.reopen());
        assert_eq!(
            std::fs::read_to_string(project.root.join(path.as_str())).unwrap(),
            "physical data\n"
        );
        assert_eq!(fixture.workspaces.comparisons.cached_payloads_for_test(), 0);
        assert_eq!(changed(&fixture, query).await, BTreeSet::from([path]));
        drain(&fixture).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn the_future_captures_the_original_root_before_its_tracked_task_is_scheduled() {
    vessel_testkit::within(DEADLINE, async {
        let fixture = Fixture::new().await;
        let project = fixture.project("before-file-queue", None).await;
        let replacement = fixture.repo("after-file-queue", None);
        let root = WorkspaceRoot::Project {
            project: project.id,
        };
        let path: RelPath = "must-not-exist.txt".parse().unwrap();
        let work = fixture.workspaces.write_file(
            root,
            &path,
            "wrong destination".to_owned(),
            WriteExpect::Absent,
        );
        fixture
            .workspaces
            .state()
            .projects
            .get_mut(&project.id)
            .unwrap()
            .info
            .root = replacement.clone();
        assert!(matches!(
            fixture.workspaces.spawn_file_work(work).await.unwrap(),
            Err(WorkspaceError::TargetChanged)
        ));
        assert!(!project.root.join(path.as_str()).exists());
        assert!(!replacement.join(path.as_str()).exists());
        drain(&fixture).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn partial_physical_work_still_invalidates_when_its_following_disk_step_fails() {
    vessel_testkit::within(DEADLINE, async {
        let fixture = Fixture::new().await;
        let project = fixture.project("partial-file-cache", None).await;
        let query = query(project.id);
        assert!(changed(&fixture, query.clone()).await.is_empty());
        let path: RelPath = "retained.txt".parse().unwrap();
        let keep = path.clone();
        let missing: RelPath = "missing.txt".parse().unwrap();
        let work = fixture
            .workspaces
            .mutating_file(query.root, move |opened| async move {
                let _written = files::write(
                    &opened,
                    &keep,
                    "retained data\n".to_owned(),
                    WriteExpect::Absent,
                )
                .await?;
                // Exercise a real disk error after a real successful write, not a fake Ok.
                files::delete(&opened, &missing, false).await
            });
        let result = fixture.workspaces.spawn_file_work(work).await.unwrap();
        assert!(matches!(result, Err(WorkspaceError::NoSuchPath(_))));
        assert_eq!(fixture.workspaces.comparisons.cached_payloads_for_test(), 0);
        assert_eq!(
            std::fs::read_to_string(project.root.join(path.as_str())).unwrap(),
            "retained data\n"
        );
        assert_eq!(changed(&fixture, query).await, BTreeSet::from([path]));
        drain(&fixture).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn ordinary_non_git_files_remain_writable_while_the_git_wrapper_still_refuses() {
    vessel_testkit::within(DEADLINE, async {
        let fixture = Fixture::new().await;
        let directory = fixture.path().join("non-git-files");
        std::fs::create_dir(&directory).unwrap();
        let project = fixture
            .workspaces
            .add_project(directory.clone(), None)
            .await
            .unwrap();
        assert!(!project.git);
        let root = WorkspaceRoot::Project {
            project: project.id,
        };
        let folder: RelPath = "folder".parse().unwrap();
        let path: RelPath = "folder/file.txt".parse().unwrap();
        fixture.workspaces.create_dir(root, &folder).await.unwrap();
        fixture
            .workspaces
            .write_file(root, &path, "plain file".to_owned(), WriteExpect::Absent)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(directory.join(path.as_str())).unwrap(),
            "plain file"
        );
        assert!(matches!(
            fixture.workspaces.git_status(root).await,
            Err(WorkspaceError::NotGit)
        ));
        drain(&fixture).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn duplication_uses_the_same_cache_guard_and_actual_returned_sibling_path() {
    vessel_testkit::within(DEADLINE, async {
        let fixture = Fixture::new().await;
        let project = fixture.project("duplicate-file-cache", None).await;
        let query = query(project.id);
        assert!(changed(&fixture, query.clone()).await.is_empty());
        let work = fixture.workspaces.duplicate_file_entry(
            vessel_proto::workspace::panel_files::DuplicateFileEntry {
                root: query.root,
                path: "README.md".parse().unwrap(),
            },
        );
        let copied = fixture
            .workspaces
            .spawn_file_work(work)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fixture.workspaces.comparisons.cached_payloads_for_test(), 0);
        assert_eq!(
            std::fs::read_to_string(project.root.join(copied.path.as_str())).unwrap(),
            "hello\n"
        );
        assert_eq!(
            changed(&fixture, query).await,
            BTreeSet::from([copied.path])
        );
        drain(&fixture).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn shutdown_refuses_a_prepared_file_future_before_unknown_root_or_any_disk_step() {
    vessel_testkit::within(DEADLINE, async {
        let fixture = Fixture::new().await;
        let root = WorkspaceRoot::Project {
            project: ProjectId::random(),
        };
        let work = fixture.workspaces.mutating_file(root, |_| {
            std::future::poll_fn(|_| -> std::task::Poll<Result<(), files::FileError>> {
                panic!("shutdown must not poll the physical operation")
            })
        });
        fixture.stop.cancel();
        assert!(matches!(
            fixture.workspaces.spawn_file_work(work).await.unwrap(),
            Err(WorkspaceError::ComparisonJob(
                super::super::comparison::JobError::Closed
            ))
        ));
        assert!(fixture.tasks.close());
        fixture.tasks.wait().await;
    })
    .await
    .unwrap();
}
