//! Public read entry points and the error cleanup boundary, with real git.
use tokio_util::sync::CancellationToken;
use vessel_proto::workspace::ChangeStatus;

use super::*;
use crate::git::GitError;
use crate::testing::Sandbox;

#[tokio::test]
async fn both_entry_points_finish_the_same_all_comparison_without_changing_the_real_index() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("all-entry-points");
    let git = sandbox.git().await;
    sandbox.run(&repo, &["checkout", "-q", "-b", "topic"]);
    sandbox.run(&repo, &["rm", "-q", "README.md"]);
    sandbox.run(&repo, &["commit", "-q", "-m", "remove"]);
    Sandbox::write(&repo, "README.md", "hello\nrecreated\n");
    let index = std::fs::read(repo.join(".git/index")).unwrap();
    let scratch = sandbox.path().join("scratch");
    std::fs::create_dir(&scratch).unwrap();
    let kind = GitComparison::All { base: None };
    let base = WorktreeBase::Ref {
        name: "main".parse().unwrap(),
    };
    let query = Comparison {
        directory: &repo,
        kind: &kind,
        base: Some(&base),
        whitespace: GitWhitespace::Show,
        scratch: &scratch,
    };
    let changes = git.comparison_changes(query).await.unwrap().unwrap();
    assert_eq!(changes.files.len(), 1);
    let file = git
        .comparison_file(
            query,
            &"README.md".parse().unwrap(),
            GitHunkOffset::default(),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(file.status, ChangeStatus::Modified);
    assert_eq!(file.status, changes.files[0].status);
    assert_eq!(std::fs::read_dir(&scratch).unwrap().count(), 0);
    assert_eq!(std::fs::read(repo.join(".git/index")).unwrap(), index);
}

#[tokio::test]
async fn cancellation_after_preparation_still_closes_the_actual_private_index() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("cancelled");
    let git = sandbox.git().await;
    sandbox.run(&repo, &["checkout", "-q", "-b", "topic"]);
    sandbox.run(&repo, &["rm", "-q", "README.md"]);
    sandbox.run(&repo, &["commit", "-q", "-m", "remove"]);
    Sandbox::write(&repo, "README.md", "hello\nrecreated\n");
    let index = std::fs::read(repo.join(".git/index")).unwrap();
    let kind = GitComparison::All { base: None };
    let base = WorktreeBase::Ref {
        name: "main".parse().unwrap(),
    };
    let scratch = sandbox.path().join("private-index");
    std::fs::create_dir(&scratch).unwrap();
    let prepared = git
        .prepare_comparison(&repo, &kind, Some(&base), &scratch)
        .await
        .unwrap()
        .unwrap();
    let private = std::fs::read_dir(&scratch)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let stop = CancellationToken::new();
    let cancelled = git.cancellable(stop.clone());
    stop.cancel();
    let read = prepared
        .changes(&cancelled, &repo, GitWhitespace::Show)
        .await;
    assert!(matches!(
        prepared.finish(read).await,
        Err(ComparisonError::Git(GitError::Cancelled))
    ));
    assert!(!private.exists());
    assert_eq!(std::fs::read(repo.join(".git/index")).unwrap(), index);
}
