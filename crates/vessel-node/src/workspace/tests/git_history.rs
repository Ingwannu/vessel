//! History enters through the same registered roots and owned job budget.
use super::*;
use vessel_proto::workspace::{GitCommitLocation, GitHistoryRead, ReadGitHistory, WorktreeBase};

#[tokio::test]
async fn history_uses_recorded_and_default_refs_from_one_root_snapshot() {
    let fixture = Fixture::new().await;
    let repo = fixture.repo("history-node", None);
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
                    name: Some("history-topic".parse().unwrap()),
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
    let CreateWorktreeOutcome::Created { worktree } = outcome else {
        panic!("fixture worktree was not created");
    };
    let root = WorkspaceRoot::Worktree {
        worktree: worktree.id,
    };
    let facts = fixture.workspaces.resolve_git(root).await.unwrap();
    assert_eq!(facts.pinned_base, Some(stored.clone()));
    assert_eq!(facts.base, Some(stored.clone()));
    assert_eq!(facts.project_default, Some("main".parse().unwrap()));
    let _commit = fixture.git(
        &worktree.path,
        &["commit", "-q", "--allow-empty", "-m", "Topic work"],
    );
    let GitHistoryRead::Ready { data } = fixture
        .workspaces
        .git_history(ReadGitHistory { root })
        .await
        .unwrap()
    else {
        panic!("expected complete history");
    };
    assert_eq!(data.base, Some(stored.clone()));
    assert_eq!(
        data.commits
            .iter()
            .map(|item| (item.commit.subject.as_str(), item.location))
            .collect::<Vec<_>>(),
        [
            ("Topic work", GitCommitLocation::Workspace),
            ("first", GitCommitLocation::Base),
        ]
    );
    let _deleted = fixture.git(&repo, &["update-ref", "-d", stored.as_str()]);
    assert_eq!(
        fixture
            .workspaces
            .git_history(ReadGitHistory { root })
            .await
            .unwrap(),
        GitHistoryRead::BaseUnavailable { base: stored }
    );
    fixture.stop.cancel();
    assert!(fixture.tasks.close());
    fixture.tasks.wait().await;
}

#[tokio::test]
async fn a_live_history_read_revalidates_its_root_and_shutdown_refuses_before_disk_validation() {
    let fixture = Fixture::new().await;
    let repo = fixture.repo("history-admission", None);
    let project = fixture
        .workspaces
        .add_project(repo.clone(), None)
        .await
        .unwrap();
    let root = WorkspaceRoot::Project {
        project: project.id,
    };
    let facts = fixture.workspaces.resolve_git(root).await.unwrap();
    assert_eq!(facts.pinned_base, None);
    assert_eq!(facts.project_default, Some("main".parse().unwrap()));
    std::fs::rename(&repo, fixture.path().join("history-moved")).unwrap();
    let moved = fixture
        .workspaces
        .git_history(ReadGitHistory { root })
        .await;
    assert!(
        matches!(
            moved,
            Err(WorkspaceError::Conflict(ref detail))
            if detail == "the folder is gone from this machine"
        ),
        "{moved:?}"
    );
    // Shutdown admission is intentionally before queued disk resolution. A
    // stopped node must not start I/O merely to rediscover that same bad root.
    fixture.stop.cancel();
    let stopped = fixture
        .workspaces
        .git_history(ReadGitHistory { root })
        .await;
    assert!(
        matches!(
            stopped,
            Err(WorkspaceError::ComparisonJob(
                super::super::comparison::JobError::Closed
            ))
        ),
        "{stopped:?}"
    );
    assert!(fixture.tasks.close());
    fixture.tasks.wait().await;
}
