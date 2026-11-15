//! The real-index regression from checkout-git tests, using production git
//! commands through the protected runner. No network or installed agent.

use vessel_proto::workspace::{
    ChangeStatus, DiffContent, GitComparison, GitHunkOffset, GitWhitespace, WorktreeBase,
};

use super::*;
use crate::testing::Sandbox;

fn base() -> WorktreeBase {
    WorktreeBase::Ref {
        name: "main".parse().unwrap(),
    }
}

#[tokio::test]
async fn recreated_base_file_is_compared_once_and_the_real_index_is_unchanged() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("all");
    let git = sandbox.git().await;
    sandbox.run(&repo, &["checkout", "-q", "-b", "topic"]);
    sandbox.run(&repo, &["rm", "-q", "README.md"]);
    sandbox.run(&repo, &["commit", "-q", "-m", "remove"]);
    Sandbox::write(&repo, "README.md", "hello\nchanged\n");
    let before = std::fs::read(repo.join(".git/index")).unwrap();
    let prepared = git
        .prepare_comparison(&repo, &GitComparison::All { base: None }, Some(&base()))
        .await
        .unwrap()
        .unwrap();
    let summary = prepared
        .changes(&git, &repo, GitWhitespace::Show)
        .await
        .unwrap();
    assert_eq!(summary.files.len(), 1);
    let changed = summary.files.first().unwrap();
    assert_eq!(changed.path.as_str(), "README.md");
    assert_eq!(changed.status, ChangeStatus::Modified);
    assert_eq!((changed.additions, changed.deletions), (Some(1), Some(0)));
    let file = prepared
        .file(
            &git,
            &repo,
            &"README.md".parse().unwrap(),
            GitWhitespace::Show,
            GitHunkOffset::default(),
        )
        .await
        .unwrap();
    assert_eq!(file.status, ChangeStatus::Modified);
    assert!(matches!(file.content, DiffContent::Hunks { .. }));
    let private = prepared.directory.as_ref().unwrap().path().to_owned();
    prepared.close().await.unwrap();
    assert!(!private.exists());
    assert_eq!(std::fs::read(repo.join(".git/index")).unwrap(), before);
}

#[tokio::test]
async fn identically_recreated_file_is_unchanged_in_summary_and_body() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("identical");
    let git = sandbox.git().await;
    sandbox.run(&repo, &["checkout", "-q", "-b", "topic"]);
    sandbox.run(&repo, &["rm", "-q", "README.md"]);
    sandbox.run(&repo, &["commit", "-q", "-m", "remove"]);
    Sandbox::write(&repo, "README.md", "hello\n");
    let prepared = git
        .prepare_comparison(&repo, &GitComparison::All { base: None }, Some(&base()))
        .await
        .unwrap()
        .unwrap();
    assert!(
        prepared
            .changes(&git, &repo, GitWhitespace::Show)
            .await
            .unwrap()
            .files
            .is_empty()
    );
    let file = prepared
        .file(
            &git,
            &repo,
            &"README.md".parse().unwrap(),
            GitWhitespace::Show,
            GitHunkOffset::default(),
        )
        .await
        .unwrap();
    assert_eq!(file.content, DiffContent::Unchanged);
    prepared.close().await.unwrap();
}

#[tokio::test]
async fn restored_paths_are_literal_and_preserve_other_staged_renames() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("literal");
    let git = sandbox.git().await;
    let name = "odd [name],*.txt";
    Sandbox::write(&repo, name, "original\n");
    sandbox.run(&repo, &["add", "--", name]);
    sandbox.run(&repo, &["commit", "-q", "-m", "base file"]);
    sandbox.run(&repo, &["checkout", "-q", "-b", "topic"]);
    sandbox.run(&repo, &["rm", "-q", "--", name]);
    sandbox.run(&repo, &["commit", "-q", "-m", "remove"]);
    Sandbox::write(&repo, name, "original\nmodified\n");
    sandbox.run(&repo, &["mv", "README.md", "renamed.md"]);
    let before = std::fs::read(repo.join(".git/index")).unwrap();
    let prepared = git
        .prepare_comparison(&repo, &GitComparison::All { base: None }, Some(&base()))
        .await
        .unwrap()
        .unwrap();
    let summary = prepared
        .changes(&git, &repo, GitWhitespace::Show)
        .await
        .unwrap();
    assert_eq!(summary.files.len(), 2);
    let literal = summary
        .files
        .iter()
        .find(|file| file.path.as_str() == name)
        .unwrap();
    assert_eq!(literal.status, ChangeStatus::Modified);
    let rename = summary
        .files
        .iter()
        .find(|file| file.path.as_str() == "renamed.md")
        .unwrap();
    assert_eq!(rename.status, ChangeStatus::Renamed);
    assert_eq!(rename.old_path.as_ref().unwrap().as_str(), "README.md");
    prepared.close().await.unwrap();
    assert_eq!(std::fs::read(repo.join(".git/index")).unwrap(), before);
}

#[test]
fn batches_bound_path_count_and_conservatively_escaped_argv() {
    let short: Vec<RelPath> = (0..1000)
        .map(|index| format!("file-{index}").parse().unwrap())
        .collect();
    let long: Vec<RelPath> = (0..8)
        .map(|index| {
            format!("{}/{index}", "a/".repeat(1900))
                .replace("//", "/")
                .parse()
                .unwrap()
        })
        .collect();
    for paths in [short, long] {
        let batches = batches(&paths);
        assert_eq!(
            batches.iter().map(|batch| batch.len()).sum::<usize>(),
            paths.len()
        );
        for batch in batches {
            assert!(batch.len() <= MAX_BATCH_PATHS);
            let cost = ARG_FIXED_BYTES
                + batch
                    .iter()
                    .map(|path| path.as_str().len() * 2 + CACHEINFO_ENTRY_BYTES)
                    .sum::<usize>();
            assert!(cost <= MAX_BATCH_ARG_BYTES);
        }
    }
}

#[test]
fn malformed_tree_entries_are_errors_instead_of_dropped_paths() {
    let id = "a".repeat(40);
    for record in [
        format!("100644 blob {id}\t../outside\0"),
        format!("100644 commit {id}\tfile\0"),
        format!("777777 blob {id}\tfile\0"),
        format!("100644 blob {id}\t\0"),
    ] {
        assert!(matches!(
            parse_entries(record.as_bytes()),
            Err(GitError::Unexpected {
                what: "base index entry"
            })
        ));
    }
}
