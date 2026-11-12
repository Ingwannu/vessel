//! Each side of the tidying rule, snapshots and their restore, against real
//! repositories.

use vessel_proto::WorktreeId;
use vessel_proto::workspace::{BranchName, KeptReason};

use super::{AddRefusal, NewBranch, SnapshotRefusal};
use crate::testing::Sandbox;

/// A repository and a worktree of it on a new branch `vessel/test`.
async fn setup(
    sandbox: &Sandbox,
) -> (
    std::path::PathBuf,
    std::path::PathBuf,
    BranchName,
    crate::Git,
) {
    let repo = sandbox.repo("r");
    let git = sandbox.git().await;
    let head = git.head(&repo).await.unwrap().unwrap();
    let branch: BranchName = "vessel/test".parse().unwrap();
    let path = sandbox.path().join("wt");
    git.add_worktree(
        &repo,
        &path,
        &NewBranch::Create {
            name: branch.clone(),
            start: head,
        },
    )
    .await
    .unwrap()
    .unwrap();
    (repo, path, branch, git)
}

#[tokio::test]
async fn a_clean_worktree_loses_nothing_and_is_removed_with_its_branch() {
    let sandbox = Sandbox::new();
    let (repo, path, branch, git) = setup(&sandbox).await;
    // Ignored files are what setup makes again: not counted.
    Sandbox::write(&repo, ".gitignore", "node_modules/\n");
    sandbox.run(&repo, &["add", ".gitignore"]);
    sandbox.run(&repo, &["commit", "-q", "-m", "ignore"]);
    sandbox.run(&path, &["merge", "-q", "--ff-only", "main"]);
    Sandbox::write(&path, "node_modules/x.js", "x");
    assert_eq!(git.would_lose(&path, Some(&branch)).await.unwrap(), []);
    git.remove_worktree(&repo, &path, Some(&branch), false)
        .await
        .unwrap();
    assert!(!path.exists());
    let branches = git.branches(&repo).await.unwrap();
    assert!(
        branches
            .branches
            .iter()
            .all(|found| found.name.as_str() != "vessel/test"),
        "the branch it created goes with it"
    );
}

#[tokio::test]
async fn changes_commits_flags_and_operations_each_keep_a_worktree() {
    let sandbox = Sandbox::new();
    let (repo, path, branch, git) = setup(&sandbox).await;

    Sandbox::write(&path, "untracked.txt", "u");
    assert_eq!(
        git.would_lose(&path, Some(&branch)).await.unwrap(),
        [KeptReason::Uncommitted { files: 1 }]
    );
    sandbox.run(&path, &["add", "untracked.txt"]);
    sandbox.run(&path, &["commit", "-q", "-m", "only here"]);
    assert_eq!(
        git.would_lose(&path, Some(&branch)).await.unwrap(),
        [KeptReason::UniqueCommits { commits: 1 }],
        "a commit on the branch the node would delete"
    );
    // Once another branch has it, nothing is lost.
    sandbox.run(&repo, &["branch", "keep", "vessel/test"]);
    assert_eq!(git.would_lose(&path, Some(&branch)).await.unwrap(), []);
    // A branch the node did not create keeps its own commits.
    assert_eq!(git.would_lose(&path, None).await.unwrap(), []);

    sandbox.run(&path, &["update-index", "--assume-unchanged", "README.md"]);
    assert_eq!(
        git.would_lose(&path, Some(&branch)).await.unwrap(),
        [KeptReason::FlaggedFiles]
    );
    sandbox.run(
        &path,
        &["update-index", "--no-assume-unchanged", "README.md"],
    );

    sandbox.run(&path, &["update-ref", "refs/worktree/mine", "HEAD"]);
    assert_eq!(
        git.would_lose(&path, Some(&branch)).await.unwrap(),
        [KeptReason::WorktreeRefs]
    );
    sandbox.run(&path, &["update-ref", "-d", "refs/worktree/mine"]);

    // A detached HEAD with a commit no branch has.
    sandbox.run(&path, &["checkout", "-q", "--detach"]);
    sandbox.run(&path, &["commit", "-q", "--allow-empty", "-m", "detached"]);
    assert_eq!(
        git.would_lose(&path, None).await.unwrap(),
        [KeptReason::UniqueCommits { commits: 1 }]
    );
}

#[tokio::test]
async fn a_snapshot_keeps_the_work_and_restores_it_uncommitted() {
    let sandbox = Sandbox::new();
    let (repo, path, branch, git) = setup(&sandbox).await;
    Sandbox::write(&path, "README.md", "edited\n");
    Sandbox::write(&path, "new/file.txt", "new\n");
    sandbox.run(&path, &["add", "new/file.txt"]);
    Sandbox::write(&path, "loose.txt", "loose\n");
    let id = WorktreeId::random();
    let snapshot = git
        .snapshot(&path, id, sandbox.path())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.branch.as_ref(), Some(&branch));
    // The worktree's own index is untouched: `loose.txt` is still untracked.
    let status = git.status(&path).await.unwrap();
    assert!(
        status
            .files
            .iter()
            .any(|entry| entry.path.as_str() == "loose.txt"
                && entry.worktree == vessel_proto::workspace::FileState::Untracked)
    );
    git.remove_worktree(&repo, &path, Some(&branch), true)
        .await
        .unwrap();

    let listed = git.snapshots(&repo).await.unwrap();
    assert_eq!(listed.snapshots.len(), 1);
    let kept = listed.snapshots.first().unwrap();
    assert_eq!(kept.worktree, id);
    assert_eq!(kept.commit, snapshot.commit);

    let back = sandbox.path().join("back");
    git.add_worktree(
        &repo,
        &back,
        &NewBranch::Create {
            name: "restored".parse().unwrap(),
            start: kept.head.clone(),
        },
    )
    .await
    .unwrap()
    .unwrap();
    git.restore_snapshot(&back, &kept.commit).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(back.join("README.md")).unwrap(),
        "edited\n"
    );
    assert_eq!(
        std::fs::read_to_string(back.join("loose.txt")).unwrap(),
        "loose\n"
    );
    let restored = git.status(&back).await.unwrap();
    assert!(
        restored
            .files
            .iter()
            .all(|entry| entry.index == vessel_proto::workspace::FileState::Unmodified),
        "put back as uncommitted, nothing staged: {restored:?}"
    );
    assert!(git.drop_snapshot(&repo, id).await.unwrap());
    assert!(!git.drop_snapshot(&repo, id).await.unwrap());
}

#[tokio::test]
async fn a_nested_repository_is_never_kept_as_a_bare_link() {
    let sandbox = Sandbox::new();
    let (_repo, path, _branch, git) = setup(&sandbox).await;
    let nested = path.join("vendor");
    std::fs::create_dir(&nested).unwrap();
    sandbox.run(&nested, &["init", "-q"]);
    Sandbox::write(&nested, "lib.txt", "l");
    sandbox.run(&nested, &["add", "."]);
    sandbox.run(&nested, &["commit", "-q", "-m", "nested"]);
    let refused = git
        .snapshot(&path, WorktreeId::random(), sandbox.path())
        .await
        .unwrap();
    assert_eq!(refused, Err(SnapshotRefusal::NestedRepository));
}

#[tokio::test]
async fn adding_refuses_an_existing_branch_and_one_checked_out_elsewhere() {
    let sandbox = Sandbox::new();
    let (repo, _path, branch, git) = setup(&sandbox).await;
    let head = git.head(&repo).await.unwrap().unwrap();
    let again = git
        .add_worktree(
            &repo,
            &sandbox.path().join("again"),
            &NewBranch::Create {
                name: branch.clone(),
                start: head,
            },
        )
        .await
        .unwrap();
    assert_eq!(again, Err(AddRefusal::BranchExists));
    let elsewhere = git
        .add_worktree(
            &repo,
            &sandbox.path().join("elsewhere"),
            &NewBranch::Existing { name: branch },
        )
        .await
        .unwrap();
    assert!(
        matches!(elsewhere, Err(AddRefusal::CheckedOutElsewhere(_))),
        "{elsewhere:?}"
    );
    let missing = git
        .add_worktree(
            &repo,
            &sandbox.path().join("missing"),
            &NewBranch::Existing {
                name: "nope".parse().unwrap(),
            },
        )
        .await
        .unwrap();
    assert_eq!(missing, Err(AddRefusal::NoSuchBranch));
}

/// A worktree's `.git` names its repository's record of it as `git
/// worktree add` wrote it; rewritten to name a folder of someone else's
/// (whose configuration git would read, filter programs included), or
/// replaced by a link, it is not the repository's any more.
#[tokio::test]
async fn a_worktree_whose_gitfile_was_rewritten_is_not_its_repositorys() {
    use crate::git::gitfile::registered;

    let sandbox = Sandbox::new();
    let (repo, path, _branch, _git) = setup(&sandbox).await;
    assert!(registered(&path, &repo));

    let gitfile = path.join(".git");
    let original = std::fs::read_to_string(&gitfile).unwrap();
    let fake = path.join("fake");
    std::fs::create_dir(&fake).unwrap();
    std::fs::write(&gitfile, "gitdir: ./fake\n").unwrap();
    assert!(!registered(&path, &repo), "a folder of its own");
    // Another worktree's record does not name this one back.
    let elsewhere = sandbox.path().join("elsewhere");
    sandbox.run(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "other",
            &elsewhere.display().to_string(),
        ],
    );
    let others = std::fs::read_to_string(elsewhere.join(".git")).unwrap();
    std::fs::write(&gitfile, others).unwrap();
    assert!(!registered(&path, &repo), "another worktree's record");
    #[cfg(unix)]
    {
        let kept = sandbox.path().join("gitfile");
        std::fs::write(&kept, &original).unwrap();
        std::fs::remove_file(&gitfile).unwrap();
        std::os::unix::fs::symlink(&kept, &gitfile).unwrap();
        assert!(!registered(&path, &repo), "a link, not the file");
        std::fs::remove_file(&gitfile).unwrap();
    }
    std::fs::write(&gitfile, original).unwrap();
    assert!(registered(&path, &repo), "as git wrote it");
}
