//! Real repositories through the node's root authority and tracked entry points.
use std::collections::BTreeSet;

use vessel_proto::workspace::{
    CompareGit, CompareGitFile, GitCompared, GitComparison, GitHunkOffset, GitWhitespace,
};

use super::*;

fn paths(value: GitCompared<vessel_proto::workspace::DiffSummary>) -> BTreeSet<String> {
    match value {
        GitCompared::Ready { data } => data
            .files
            .iter()
            .map(|file| file.path.to_string())
            .collect(),
        other
        @ (GitCompared::NoBase | GitCompared::TooLarge | GitCompared::BaseMismatch { .. }) => {
            panic!("{other:?}")
        }
    }
}

#[tokio::test]
async fn node_comparisons_keep_committed_working_and_all_distinct_and_reject_invalid_bases() {
    let fixture = Fixture::new().await;
    let repo = fixture.repo("comparison-node", None);
    let project = fixture
        .workspaces
        .add_project(repo.clone(), None)
        .await
        .unwrap();
    let _checkout = fixture.git(&repo, &["checkout", "-q", "-b", "topic"]);
    std::fs::write(repo.join("committed"), "committed\n").unwrap();
    let _stage = fixture.git(&repo, &["add", "committed"]);
    let _commit = fixture.git(&repo, &["commit", "-q", "-m", "topic"]);
    std::fs::write(repo.join("working"), "working\n").unwrap();
    let index = std::fs::read(repo.join(".git/index")).unwrap();
    let root = WorkspaceRoot::Project {
        project: project.id,
    };
    let query = |comparison| CompareGit {
        root,
        comparison,
        whitespace: GitWhitespace::Show,
        reason: vessel_proto::workspace::GitReadReason::View,
    };
    assert_eq!(
        paths(
            fixture
                .workspaces
                .git_compare(query(GitComparison::Committed { base: None }))
                .await
                .unwrap()
        ),
        BTreeSet::from(["committed".to_owned()])
    );
    assert_eq!(
        paths(
            fixture
                .workspaces
                .git_compare(query(GitComparison::Uncommitted))
                .await
                .unwrap()
        ),
        BTreeSet::from(["working".to_owned()])
    );
    assert_eq!(
        paths(
            fixture
                .workspaces
                .git_compare(query(GitComparison::All { base: None }))
                .await
                .unwrap()
        ),
        BTreeSet::from(["committed".to_owned(), "working".to_owned()])
    );
    let file = fixture
        .workspaces
        .git_compare_file(CompareGitFile {
            query: query(GitComparison::Committed { base: None }),
            path: "committed".parse().unwrap(),
            from_hunk: GitHunkOffset::default(),
        })
        .await
        .unwrap();
    assert!(matches!(file, GitCompared::Ready { data } if data.path.as_str() == "committed"));
    let invalid = fixture
        .workspaces
        .git_compare(query(GitComparison::Committed {
            base: Some(vessel_proto::workspace::WorktreeBase::Ref {
                name: "missing-ref".parse().unwrap(),
            }),
        }))
        .await;
    assert_eq!(
        invalid.unwrap_err().wire().code,
        vessel_proto::ErrorCode::InvalidRequest
    );
    assert_eq!(std::fs::read(repo.join(".git/index")).unwrap(), index);
    assert_eq!(
        std::fs::read_dir(&fixture.workspaces.dirs.scratch)
            .unwrap()
            .count(),
        0
    );
    fixture.stop.cancel();
    assert!(fixture.tasks.close());
    fixture.tasks.wait().await;
}

#[tokio::test]
async fn an_unknown_root_fails_before_a_job_and_a_stopped_node_refuses_the_resolved_one() {
    let fixture = Fixture::new().await;
    let unknown = CompareGit {
        root: WorkspaceRoot::Project {
            project: ProjectId::random(),
        },
        comparison: GitComparison::Uncommitted,
        whitespace: GitWhitespace::Show,
        reason: vessel_proto::workspace::GitReadReason::View,
    };
    assert!(matches!(
        fixture.workspaces.git_compare(unknown).await,
        Err(WorkspaceError::UnknownProject(_))
    ));
    assert!(fixture.tasks.is_empty());
    let repo = fixture.repo("stopped-comparison", None);
    let project = fixture.workspaces.add_project(repo, None).await.unwrap();
    fixture.stop.cancel();
    let result = fixture
        .workspaces
        .git_compare(CompareGit {
            root: WorkspaceRoot::Project {
                project: project.id,
            },
            comparison: GitComparison::Uncommitted,
            whitespace: GitWhitespace::Show,
            reason: vessel_proto::workspace::GitReadReason::View,
        })
        .await;
    assert_eq!(
        result.unwrap_err().wire().code,
        vessel_proto::ErrorCode::Unavailable
    );
    assert!(fixture.tasks.close());
    fixture.tasks.wait().await;
}

#[tokio::test]
async fn a_folder_cannot_be_requested_as_a_file_diff() {
    let fixture = Fixture::new().await;
    let result = fixture
        .workspaces
        .git_compare_file(CompareGitFile {
            query: CompareGit {
                root: WorkspaceRoot::Project {
                    project: ProjectId::random(),
                },
                comparison: GitComparison::Uncommitted,
                whitespace: GitWhitespace::Show,
                reason: vessel_proto::workspace::GitReadReason::View,
            },
            path: "".parse().unwrap(),
            from_hunk: GitHunkOffset::default(),
        })
        .await;
    assert!(matches!(result, Err(WorkspaceError::Invalid(_))));
    assert!(fixture.tasks.is_empty());
    fixture.stop.cancel();
    assert!(fixture.tasks.close());
    fixture.tasks.wait().await;
}

#[tokio::test]
async fn the_same_resolved_snapshot_admits_only_the_recorded_worktree_stream() {
    use vessel_proto::workspace::{RefName, WorktreeBase};

    let fixture = Fixture::new().await;
    let repo = fixture.repo("pinned-comparison", None);
    let project = fixture
        .workspaces
        .add_project(repo.clone(), None)
        .await
        .unwrap();
    let _remote = fixture.git(&repo, &["update-ref", "refs/remotes/upstream/main", "HEAD"]);
    let stored: RefName = "refs/remotes/upstream/main".parse().unwrap();
    let outcome = fixture
        .workspaces
        .create_worktree(
            Creator::Owner,
            CreateWorktree {
                project: project.id,
                branch: WorktreeBranch::New {
                    name: Some("pinned-topic".parse().unwrap()),
                    base: Some(WorktreeBase::Ref {
                        name: stored.clone(),
                    }),
                },
                setup: SetupChoice::Skip,
            },
            None,
        )
        .await
        .unwrap();
    let worktree = match outcome {
        CreateWorktreeOutcome::Created { worktree } => worktree,
        other @ (CreateWorktreeOutcome::NeedsApproval { .. }
        | CreateWorktreeOutcome::Refused { .. }) => panic!("{other:?}"),
    };
    let root = WorkspaceRoot::Worktree {
        worktree: worktree.id,
    };
    let project_facts = fixture
        .workspaces
        .resolve_git(WorkspaceRoot::Project {
            project: project.id,
        })
        .await
        .unwrap();
    assert_eq!(project_facts.pinned_base, None);
    assert_eq!(project_facts.base, Some("main".parse().unwrap()));
    let facts = fixture.workspaces.resolve_git(root).await.unwrap();
    assert_eq!(facts.pinned_base, Some(stored.clone()));
    assert_eq!(facts.base, Some(stored.clone()));
    let query = |name: &str| CompareGit {
        root,
        comparison: GitComparison::All {
            base: Some(WorktreeBase::Ref {
                name: name.parse().unwrap(),
            }),
        },
        whitespace: GitWhitespace::Show,
        reason: vessel_proto::workspace::GitReadReason::View,
    };
    assert!(paths(fixture.workspaces.git_compare(query("main")).await.unwrap()).is_empty());
    // A stopped tracker would refuse any admitted work. A mismatch still returns
    // both facts, proving admission stops before the job and private index.
    fixture.stop.cancel();
    let requested = WorktreeBase::Ref {
        name: "refs/heads/main".parse().unwrap(),
    };
    assert_eq!(
        fixture
            .workspaces
            .git_compare(query("refs/heads/main"))
            .await
            .unwrap(),
        GitCompared::BaseMismatch {
            stored: stored.clone(),
            requested: requested.clone()
        }
    );
    assert_eq!(
        fixture
            .workspaces
            .git_compare_file(CompareGitFile {
                query: query("refs/heads/main"),
                path: "README.md".parse().unwrap(),
                from_hunk: GitHunkOffset::default(),
            })
            .await
            .unwrap(),
        GitCompared::BaseMismatch { stored, requested }
    );
    assert_eq!(
        std::fs::read_dir(&fixture.workspaces.dirs.scratch)
            .unwrap()
            .count(),
        0
    );
    assert!(fixture.tasks.close());
    fixture.tasks.wait().await;
}

#[tokio::test]
async fn explicit_refresh_bypasses_cached_working_changes_and_a_commit_invalidates_them() {
    use vessel_proto::workspace::{CommitStage, GitReadReason};
    let fixture = Fixture::new().await;
    let repo = fixture.repo("comparison-cache", None);
    let project = fixture
        .workspaces
        .add_project(repo.clone(), None)
        .await
        .unwrap();
    let mut query = CompareGit {
        root: WorkspaceRoot::Project {
            project: project.id,
        },
        comparison: GitComparison::Uncommitted,
        whitespace: GitWhitespace::Show,
        reason: GitReadReason::View,
    };
    std::fs::write(repo.join("first"), "one\n").unwrap();
    let first = BTreeSet::from(["first".to_owned()]);
    assert_eq!(
        paths(fixture.workspaces.git_compare(query.clone()).await.unwrap()),
        first
    );
    std::fs::write(repo.join("second"), "two\n").unwrap();
    assert_eq!(
        paths(fixture.workspaces.git_compare(query.clone()).await.unwrap()),
        first
    );
    query.reason = GitReadReason::Refresh;
    assert_eq!(
        paths(fixture.workspaces.git_compare(query.clone()).await.unwrap()),
        BTreeSet::from(["first".to_owned(), "second".to_owned()])
    );
    let _outcome = fixture
        .workspaces
        .git_commit(
            query.root,
            &"save fixture".parse().unwrap(),
            CommitStage::Everything,
        )
        .await
        .unwrap();
    query.reason = GitReadReason::View;
    assert!(paths(fixture.workspaces.git_compare(query).await.unwrap()).is_empty());
    fixture.stop.cancel();
    assert!(fixture.tasks.close());
    fixture.tasks.wait().await;
}

#[tokio::test]
async fn a_cached_payload_cannot_bypass_revalidation_of_the_recorded_folder() {
    let fixture = Fixture::new().await;
    let repo = fixture.repo("removed-cached-root", None);
    let project = fixture
        .workspaces
        .add_project(repo.clone(), None)
        .await
        .unwrap();
    let query = CompareGit {
        root: WorkspaceRoot::Project {
            project: project.id,
        },
        comparison: GitComparison::Uncommitted,
        whitespace: GitWhitespace::Show,
        reason: vessel_proto::workspace::GitReadReason::View,
    };
    let _cached = fixture.workspaces.git_compare(query.clone()).await.unwrap();
    std::fs::rename(&repo, repo.with_file_name("moved-cached-root")).unwrap();
    assert_eq!(
        fixture
            .workspaces
            .git_compare(query)
            .await
            .unwrap_err()
            .wire()
            .code,
        vessel_proto::ErrorCode::Conflict
    );
    fixture.stop.cancel();
    assert!(fixture.tasks.close());
    fixture.tasks.wait().await;
}

mod cached_root_probe;
