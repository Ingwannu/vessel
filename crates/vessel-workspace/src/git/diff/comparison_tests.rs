//! ADR 0087 comparison cases, after checkout-git batching/worktree tests.
//! Real repositories prove HEAD, index and working files remain distinct.

use vessel_proto::workspace::{DiffContent, GitComparison, WorktreeBase};

use crate::testing::Sandbox;

#[tokio::test]
async fn committed_only_excludes_staged_unstaged_and_untracked_changes_and_preserves_index() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("comparisons");
    let git = sandbox.git().await;
    sandbox.run(&repo, &["checkout", "-q", "-b", "topic"]);
    Sandbox::write(&repo, "committed.txt", "committed\n");
    sandbox.run(&repo, &["add", "."]);
    sandbox.run(&repo, &["commit", "-q", "-m", "branch change"]);
    Sandbox::write(&repo, "README.md", "working\n");
    Sandbox::write(&repo, "staged.txt", "staged\n");
    sandbox.run(&repo, &["add", "staged.txt"]);
    Sandbox::write(&repo, "untracked.txt", "new\n");
    let index_before = std::fs::read(repo.join(".git/index")).unwrap();
    let base = WorktreeBase::Ref {
        name: "main".parse().unwrap(),
    };
    for (comparison, expected) in [
        (
            GitComparison::Committed { base: None },
            vec!["committed.txt"],
        ),
        (
            GitComparison::Uncommitted,
            vec!["README.md", "staged.txt", "untracked.txt"],
        ),
        (
            GitComparison::All { base: None },
            vec!["README.md", "committed.txt", "staged.txt", "untracked.txt"],
        ),
    ] {
        let resolved = git
            .resolve_comparison(&repo, &comparison, Some(&base))
            .await
            .unwrap()
            .unwrap();
        let summary = git.changes(&repo, &resolved, false).await.unwrap();
        assert_eq!(
            summary
                .files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }
    let resolved = git
        .resolve_comparison(&repo, &GitComparison::Committed { base: None }, Some(&base))
        .await
        .unwrap()
        .unwrap();
    let file = git
        .file_diff(
            &repo,
            &resolved,
            &"committed.txt".parse().unwrap(),
            false,
            0,
        )
        .await
        .unwrap();
    let DiffContent::Hunks { hunks } = file.content else {
        panic!("expected text hunks");
    };
    assert_eq!(
        hunks.first().unwrap().lines.first().unwrap().text,
        "committed"
    );
    assert_eq!(
        std::fs::read(repo.join(".git/index")).unwrap(),
        index_before
    );
}

#[tokio::test]
async fn no_base_is_explicit_and_an_invalid_requested_base_does_not_fall_back() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("base");
    let git = sandbox.git().await;
    for comparison in [
        GitComparison::Committed { base: None },
        GitComparison::All { base: None },
    ] {
        assert!(
            git.resolve_comparison(&repo, &comparison, None)
                .await
                .unwrap()
                .is_none()
        );
    }
    let good = WorktreeBase::Ref {
        name: "main".parse().unwrap(),
    };
    let asked = GitComparison::Committed {
        base: Some(WorktreeBase::Ref {
            name: "missing".parse().unwrap(),
        }),
    };
    assert!(matches!(
        git.resolve_comparison(&repo, &asked, Some(&good)).await,
        Err(super::ComparisonError::Git(
            crate::git::GitError::Unexpected { what: "base" }
        ))
    ));
}

#[tokio::test]
async fn committed_only_before_the_first_commit_is_empty_even_with_new_files() {
    let sandbox = Sandbox::new();
    let repo = sandbox.path().join("unborn");
    std::fs::create_dir(&repo).unwrap();
    sandbox.run(&repo, &["init", "-q"]);
    let other = sandbox.repo("base");
    sandbox.run(
        &repo,
        &[
            "fetch",
            "-q",
            other.to_str().unwrap(),
            "main:refs/heads/base",
        ],
    );
    Sandbox::write(&repo, "new.txt", "not committed\n");
    let git = sandbox.git().await;
    let base = WorktreeBase::Ref {
        name: "base".parse().unwrap(),
    };
    let resolved = git
        .resolve_comparison(&repo, &GitComparison::Committed { base: None }, Some(&base))
        .await
        .unwrap()
        .unwrap();
    let summary = git.changes(&repo, &resolved, false).await.unwrap();
    assert!(summary.files.is_empty());
    assert!(!summary.more);
    assert_eq!(summary.from, None);
}

#[tokio::test]
async fn comparison_prefers_origin_while_qualified_bases_keep_their_exact_stream() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("remote-base");
    let git = sandbox.git().await;
    sandbox.run(&repo, &["checkout", "-q", "-b", "remote-tip"]);
    Sandbox::write(&repo, "origin-base.txt", "already on origin\n");
    sandbox.run(&repo, &["add", "."]);
    sandbox.run(&repo, &["commit", "-q", "-m", "origin base"]);
    sandbox.run(&repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    sandbox.run(&repo, &["checkout", "-q", "main"]);
    Sandbox::write(&repo, "local-drift.txt", "local base drift\n");
    sandbox.run(&repo, &["add", "."]);
    sandbox.run(&repo, &["commit", "-q", "-m", "local drift"]);
    sandbox.run(
        &repo,
        &["checkout", "-q", "-b", "topic", "refs/remotes/origin/main"],
    );
    Sandbox::write(&repo, "working.txt", "not committed\n");
    let index = std::fs::read(repo.join(".git/index")).unwrap();
    for (name, committed, all) in [
        ("main", vec![], vec!["working.txt"]),
        ("origin/main", vec![], vec!["working.txt"]),
        ("refs/remotes/origin/main", vec![], vec!["working.txt"]),
        (
            "refs/heads/main",
            vec!["origin-base.txt"],
            vec!["origin-base.txt", "working.txt"],
        ),
    ] {
        let base = WorktreeBase::Ref {
            name: name.parse().unwrap(),
        };
        for (comparison, expected) in [
            (
                GitComparison::Committed {
                    base: Some(base.clone()),
                },
                committed,
            ),
            (GitComparison::All { base: Some(base) }, all),
        ] {
            let resolved = git
                .resolve_comparison(&repo, &comparison, None)
                .await
                .unwrap()
                .unwrap();
            let summary = git.changes(&repo, &resolved, false).await.unwrap();
            assert_eq!(
                summary
                    .files
                    .iter()
                    .map(|file| file.path.as_str())
                    .collect::<Vec<_>>(),
                expected,
                "{name}"
            );
        }
    }
    assert_eq!(std::fs::read(repo.join(".git/index")).unwrap(), index);
}

#[tokio::test]
async fn qualified_missing_or_noncommit_refs_never_fall_back_to_a_valid_local_branch() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("invalid-remote-base");
    let git = sandbox.git().await;
    for name in ["refs/remotes/origin/main", "refs/heads/missing"] {
        let query = GitComparison::Committed {
            base: Some(WorktreeBase::Ref {
                name: name.parse().unwrap(),
            }),
        };
        assert!(matches!(
            git.resolve_comparison(&repo, &query, None).await,
            Err(super::ComparisonError::Git(
                crate::git::GitError::Unexpected { what: "base" }
            ))
        ));
    }
    sandbox.run(
        &repo,
        &["update-ref", "refs/remotes/origin/main", "HEAD:README.md"],
    );
    let query = GitComparison::Committed {
        base: Some(WorktreeBase::Ref {
            name: "main".parse().unwrap(),
        }),
    };
    assert!(matches!(
        git.resolve_comparison(&repo, &query, None).await,
        Err(super::ComparisonError::Git(
            crate::git::GitError::Failed { .. }
        ))
    ));
}
