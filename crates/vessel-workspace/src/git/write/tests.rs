//! Every change against real repositories: a working one and a bare one
//! standing in for a remote.

use vessel_proto::workspace::{
    CommitOutcome, CommitStage, FetchOutcome, GitRefusal, PathSet, PullOutcome, PushOutcome,
    RenameOutcome, StashAction, StashOutcome, SwitchOutcome, WorktreeBase,
};

use crate::testing::Sandbox;

fn paths(list: &[&str]) -> PathSet {
    PathSet::Paths {
        paths: list.iter().map(|path| path.parse().unwrap()).collect(),
    }
}

#[tokio::test]
async fn stage_unstage_discard_and_commit() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("r");
    let git = sandbox.git().await;
    Sandbox::write(&repo, "README.md", "changed\n");
    Sandbox::write(&repo, "new.txt", "n\n");
    Sandbox::write(&repo, "*", "a file named star\n");
    // A literal path: `*` is that one file, not a glob of everything.
    git.stage(&repo, &paths(&["*"])).await.unwrap();
    let status = git.status(&repo).await.unwrap();
    let staged: Vec<_> = status
        .files
        .iter()
        .filter(|entry| entry.index != vessel_proto::workspace::FileState::Unmodified)
        .map(|entry| entry.path.as_str())
        .collect();
    assert_eq!(staged, ["*"]);
    git.unstage(&repo, &PathSet::All).await.unwrap();
    git.stage(&repo, &paths(&["new.txt"])).await.unwrap();

    let nothing = git
        .commit(&repo, &"empty".parse().unwrap(), CommitStage::Staged)
        .await
        .unwrap();
    assert!(
        matches!(nothing, CommitOutcome::Committed { .. }),
        "new.txt is staged"
    );
    let again = git
        .commit(&repo, &"again".parse().unwrap(), CommitStage::Staged)
        .await
        .unwrap();
    assert_eq!(
        again,
        CommitOutcome::Refused {
            refusal: GitRefusal::NothingToCommit
        }
    );
    let tracked = git
        .commit(&repo, &"tracked".parse().unwrap(), CommitStage::Tracked)
        .await
        .unwrap();
    let CommitOutcome::Committed { commit } = tracked else {
        panic!("{tracked:?}");
    };
    assert_eq!(commit.subject, "tracked");
    assert!(
        repo.join("*").exists(),
        "untracked files stay out of `tracked`"
    );

    Sandbox::write(&repo, "README.md", "dirty\n");
    Sandbox::write(&repo, "junk.txt", "j");
    Sandbox::write(&repo, ".gitignore", "ignored.txt\n");
    Sandbox::write(&repo, "ignored.txt", "keep me");
    git.discard(&repo, &PathSet::All, false).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.join("README.md")).unwrap(),
        "changed\n"
    );
    assert!(
        repo.join("junk.txt").exists(),
        "untracked stays without `untracked`"
    );
    git.discard(&repo, &paths(&["junk.txt"]), true)
        .await
        .unwrap();
    assert!(!repo.join("junk.txt").exists());
    assert!(
        repo.join("ignored.txt").exists(),
        "ignored files are never touched"
    );
}

#[tokio::test]
async fn a_commit_without_an_identity_is_refused() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("r");
    let git = sandbox.git().await;
    // The sandbox's global file names a user; this repository unsets it.
    sandbox.run(&repo, &["config", "user.name", ""]);
    sandbox.run(&repo, &["config", "user.email", ""]);
    let outcome = git
        .commit(&repo, &"x".parse().unwrap(), CommitStage::Everything)
        .await
        .unwrap();
    assert_eq!(
        outcome,
        CommitOutcome::Refused {
            refusal: GitRefusal::NoIdentity
        }
    );
}

#[tokio::test]
async fn push_fetch_and_pull_against_a_remote() {
    let sandbox = Sandbox::new();
    let remote = sandbox.path().join("remote.git");
    std::fs::create_dir(&remote).unwrap();
    sandbox.run(&remote, &["init", "-q", "--bare"]);
    let one = sandbox.repo("one");
    let git = sandbox.git().await;
    sandbox.run(&one, &["remote", "add", "origin", remote.to_str().unwrap()]);
    let no_upstream = git.push(&one, None, false).await.unwrap();
    assert_eq!(
        no_upstream,
        PushOutcome::Refused {
            refusal: GitRefusal::NoUpstream
        }
    );
    let pushed = git.push(&one, None, true).await.unwrap();
    assert!(
        matches!(&pushed, PushOutcome::Pushed { upstream_set: true, branch, .. } if branch.as_str() == "main"),
        "{pushed:?}"
    );

    let two = sandbox.path().join("two");
    sandbox.run(
        sandbox.path(),
        &["clone", "-q", remote.to_str().unwrap(), "two"],
    );
    Sandbox::write(&one, "README.md", "from one\n");
    sandbox.run(&one, &["commit", "-q", "-am", "one"]);
    assert!(matches!(
        git.push(&one, None, false).await.unwrap(),
        PushOutcome::Pushed { .. }
    ));
    assert_eq!(git.fetch(&two, None).await.unwrap(), FetchOutcome::Fetched);
    let pulled = git.pull(&two).await.unwrap();
    assert!(matches!(pulled, PullOutcome::Pulled { .. }), "{pulled:?}");
    assert_eq!(git.pull(&two).await.unwrap(), PullOutcome::UpToDate);

    // Both sides commit: the push is not a fast-forward, the pull diverges.
    Sandbox::write(&one, "a.txt", "a");
    sandbox.run(&one, &["add", "a.txt"]);
    sandbox.run(&one, &["commit", "-q", "-m", "a"]);
    assert!(matches!(
        git.push(&one, None, false).await.unwrap(),
        PushOutcome::Pushed { .. }
    ));
    Sandbox::write(&two, "b.txt", "b");
    sandbox.run(&two, &["add", "b.txt"]);
    sandbox.run(&two, &["commit", "-q", "-m", "b"]);
    assert_eq!(
        git.push(&two, None, false).await.unwrap(),
        PushOutcome::Refused {
            refusal: GitRefusal::NonFastForward
        }
    );
    assert_eq!(
        git.pull(&two).await.unwrap(),
        PullOutcome::Refused {
            refusal: GitRefusal::Diverged
        }
    );
}

#[tokio::test]
async fn switch_rename_and_the_stash() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("r");
    let git = sandbox.git().await;
    let topic = "topic".parse().unwrap();
    let main = WorktreeBase::Ref {
        name: "main".parse().unwrap(),
    };
    assert_eq!(
        git.switch(&repo, &topic, Some(&main)).await.unwrap(),
        SwitchOutcome::Switched {
            branch: topic.clone()
        }
    );
    assert_eq!(
        git.switch(&repo, &topic, Some(&main)).await.unwrap(),
        SwitchOutcome::Refused {
            refusal: GitRefusal::BranchExists
        }
    );
    assert_eq!(
        git.switch(&repo, &"nope".parse().unwrap(), None)
            .await
            .unwrap(),
        SwitchOutcome::Refused {
            refusal: GitRefusal::NoSuchRef
        }
    );
    let renamed = git
        .rename_branch(&repo, None, &"feature".parse().unwrap())
        .await
        .unwrap();
    assert!(matches!(renamed, RenameOutcome::Renamed { .. }));

    let empty = git
        .stash(
            &repo,
            &StashAction::Save {
                message: None,
                untracked: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        empty,
        StashOutcome::Refused {
            refusal: GitRefusal::NothingToStash
        }
    );
    Sandbox::write(&repo, "README.md", "stash me\n");
    let saved = git
        .stash(
            &repo,
            &StashAction::Save {
                message: Some("wip".parse().unwrap()),
                untracked: false,
            },
        )
        .await
        .unwrap();
    let StashOutcome::Done { stashes } = saved else {
        panic!("{saved:?}");
    };
    let entry = stashes.entries.first().unwrap().clone();
    // Another run stashes in between: index 0 is no longer this entry.
    Sandbox::write(&repo, "README.md", "second\n");
    sandbox.run(&repo, &["stash", "-q"]);
    let moved = git
        .stash(
            &repo,
            &StashAction::Pop {
                index: 0,
                commit: entry.commit.clone(),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        moved,
        StashOutcome::Refused {
            refusal: GitRefusal::StashMoved
        }
    );
    let popped = git
        .stash(
            &repo,
            &StashAction::Pop {
                index: 1,
                commit: entry.commit,
            },
        )
        .await
        .unwrap();
    assert!(matches!(popped, StashOutcome::Done { .. }), "{popped:?}");
    assert_eq!(
        std::fs::read_to_string(repo.join("README.md")).unwrap(),
        "stash me\n"
    );
}
