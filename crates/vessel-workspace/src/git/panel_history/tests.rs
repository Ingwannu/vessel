//! checkout-git.commits.test.ts fixtures, plus strict missing-ref admission.
//! File stats are exercised by immutable comparison tests, not duplicated here.
use super::*;
use crate::testing::Sandbox;
use std::path::Path;
use vessel_proto::workspace::{GitCommitLocation as Location, GitCommitPublication as Publication};

fn commit(sandbox: &Sandbox, repo: &Path, subject: &str) {
    sandbox.run(repo, &["commit", "-q", "--allow-empty", "-m", subject]);
}

async fn history(git: &Git, repo: &Path, base: &str) -> GitHistory {
    git.workspace_history(HistoryQuery {
        directory: repo,
        base: Some(&base.parse().unwrap()),
        project_default: Some(&"main".parse().unwrap()),
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn all_workspace_commits_precede_ten_base_commits_without_a_legacy_page_cap() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("history");
    let git = sandbox.git().await;
    for index in 1..=14 {
        commit(&sandbox, &repo, &format!("Base {index}"));
    }
    sandbox.run(&repo, &["checkout", "-q", "-b", "feature"]);
    for index in 1..=205 {
        commit(&sandbox, &repo, &format!("Workspace {index}"));
    }
    let result = history(&git, &repo, "main").await;
    assert_eq!(result.base, Some("main".parse().unwrap()));
    assert_eq!(result.commits.len(), 215);
    let expected = (1..=205)
        .rev()
        .map(|index| (format!("Workspace {index}"), Location::Workspace))
        .chain(
            (5..=14)
                .rev()
                .map(|index| (format!("Base {index}"), Location::Base)),
        )
        .collect::<Vec<_>>();
    assert_eq!(
        result
            .commits
            .iter()
            .map(|item| (item.commit.subject.clone(), item.location))
            .collect::<Vec<_>>(),
        expected
    );
    assert!(
        result
            .commits
            .iter()
            .all(|item| item.publication == Publication::Local && item.commit.author == "Test")
    );
    assert!(
        result
            .commits
            .iter()
            .all(|item| item.commit.id.as_str().starts_with(item.short.as_str()))
    );
}

#[tokio::test]
async fn recent_base_history_is_capped_and_a_short_saved_missing_base_uses_the_default() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("base");
    let git = sandbox.git().await;
    for index in 1..=14 {
        commit(&sandbox, &repo, &format!("Base {index}"));
    }
    let on_base = history(&git, &repo, "main").await;
    assert_eq!(on_base.base, None);
    assert_eq!(
        on_base
            .commits
            .iter()
            .map(|item| item.commit.subject.as_str())
            .collect::<Vec<_>>(),
        (5..=14)
            .rev()
            .map(|index| format!("Base {index}"))
            .collect::<Vec<_>>()
    );
    assert!(
        on_base
            .commits
            .iter()
            .all(|item| item.location == Location::Base)
    );
    sandbox.run(&repo, &["checkout", "-q", "-b", "feature"]);
    commit(&sandbox, &repo, "Feature work");
    let fallback = history(&git, &repo, "deleted-base").await;
    assert_eq!(fallback.base, Some("main".parse().unwrap()));
    assert_eq!(
        fallback.commits.first().unwrap().location,
        Location::Workspace
    );
    assert_eq!(
        fallback.commits.first().unwrap().commit.subject,
        "Feature work"
    );
    let qualified = "refs/heads/deleted-base".parse().unwrap();
    assert!(
        matches!(git.workspace_history(HistoryQuery { directory: &repo, base: Some(&qualified), project_default: Some(&"main".parse().unwrap()) }).await,
        Err(HistoryError::MissingBase(found)) if found == qualified)
    );
}

#[tokio::test]
async fn context_starts_at_the_fork_and_publication_uses_any_remote_not_just_upstream() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("fork");
    let git = sandbox.git().await;
    commit(&sandbox, &repo, "Shared base");
    sandbox.run(&repo, &["update-ref", "refs/remotes/other/main", "HEAD"]);
    sandbox.run(&repo, &["checkout", "-q", "-b", "feature"]);
    commit(&sandbox, &repo, "Published feature");
    sandbox.run(&repo, &["update-ref", "refs/remotes/other/feature", "HEAD"]);
    commit(&sandbox, &repo, "Local feature");
    sandbox.run(&repo, &["checkout", "-q", "main"]);
    commit(&sandbox, &repo, "Newer base");
    sandbox.run(&repo, &["checkout", "-q", "feature"]);
    let result = history(&git, &repo, "main").await;
    assert_eq!(
        result
            .commits
            .iter()
            .map(|item| (
                item.commit.subject.as_str(),
                item.location,
                item.publication
            ))
            .collect::<Vec<_>>(),
        [
            ("Local feature", Location::Workspace, Publication::Local),
            (
                "Published feature",
                Location::Workspace,
                Publication::Published
            ),
            ("Shared base", Location::Base, Publication::Published),
            ("first", Location::Base, Publication::Published),
        ]
    );
}

#[tokio::test]
async fn a_local_ahead_base_wins_ties_but_a_more_advanced_origin_wins() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("streams");
    let git = sandbox.git().await;
    sandbox.run(&repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    commit(&sandbox, &repo, "Local base work");
    sandbox.run(&repo, &["checkout", "-q", "-b", "feature"]);
    commit(&sandbox, &repo, "Feature work");
    let local = history(&git, &repo, "origin/main").await;
    assert_eq!(local.base, Some("main".parse().unwrap()));
    assert_eq!(
        local
            .commits
            .iter()
            .map(|item| (item.commit.subject.as_str(), item.location))
            .collect::<Vec<_>>(),
        [
            ("Feature work", Location::Workspace),
            ("Local base work", Location::Base),
            ("first", Location::Base),
        ]
    );
    sandbox.run(
        &repo,
        &[
            "checkout",
            "-q",
            "-b",
            "remote-base",
            "refs/remotes/origin/main",
        ],
    );
    commit(&sandbox, &repo, "Remote one");
    sandbox.run(&repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    sandbox.run(&repo, &["checkout", "-q", "feature"]);
    assert_eq!(
        history(&git, &repo, "main").await.base,
        Some("main".parse().unwrap()),
        "a tie keeps local"
    );
    sandbox.run(&repo, &["checkout", "-q", "remote-base"]);
    commit(&sandbox, &repo, "Remote two");
    sandbox.run(&repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    sandbox.run(&repo, &["checkout", "-q", "feature"]);
    assert_eq!(
        history(&git, &repo, "main").await.base,
        Some("origin/main".parse().unwrap())
    );
    assert_eq!(
        history(&git, &repo, "refs/heads/main").await.base,
        Some("refs/heads/main".parse().unwrap())
    );
}

#[tokio::test]
async fn detached_and_unborn_checkouts_have_no_branch_history_and_abbreviations_stay_observed() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("checkout");
    let git = sandbox.git().await;
    sandbox.run(&repo, &["config", "core.abbrev", "4"]);
    let result = history(&git, &repo, "main").await;
    assert_eq!(result.commits[0].short.as_str().len(), 4);
    sandbox.run(&repo, &["checkout", "-q", "--detach"]);
    assert_eq!(
        history(&git, &repo, "main").await,
        GitHistory {
            base: None,
            commits: Vec::new()
        }
    );
    sandbox.run(&repo, &["checkout", "-q", "--orphan", "new"]);
    assert_eq!(
        history(&git, &repo, "main").await,
        GitHistory {
            base: None,
            commits: Vec::new()
        }
    );
}
