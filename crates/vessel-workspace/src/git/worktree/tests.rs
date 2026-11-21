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

/// A `git` that writes `start` and `end` around every `worktree add` to
/// `log`, and holds each between the two until `release` exists: what a
/// test needs to see whether two adds ran at once.
#[cfg(unix)]
fn gated_git(sandbox: &Sandbox) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let real = which::which("git").expect("the tests need git on PATH");
    let sleep = which::which("sleep").expect("the test needs sleep on PATH");
    let log = sandbox.path().join("adds.log");
    let release = sandbox.path().join("release");
    let program = sandbox.path().join("gated-git");
    let script = format!(
        "#!/bin/sh\n\
         case \"$*\" in\n\
         *'worktree add'*)\n\
         \techo start >> '{log}'\n\
         \twhile [ ! -e '{release}' ]; do '{sleep}' 0.02; done\n\
         \t'{real}' \"$@\"\n\
         \tcode=$?\n\
         \techo end >> '{log}'\n\
         \texit $code\n\
         \t;;\n\
         esac\n\
         exec '{real}' \"$@\"\n",
        log = log.display(),
        release = release.display(),
        sleep = sleep.display(),
        real = real.display(),
    );
    Sandbox::executable(&program, &script);
    (program, log, release)
}

/// The log's lines once it holds `starts` starts (a hang is a failure).
#[cfg(unix)]
async fn once_started(log: &std::path::Path, starts: usize) -> Vec<String> {
    let wait = async {
        loop {
            let text = std::fs::read_to_string(log).unwrap_or_default();
            if text.lines().filter(|line| *line == "start").count() >= starts {
                return text.lines().map(str::to_owned).collect();
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(60), wait)
        .await
        .expect("the adds never started")
}

/// Two creations in one repository take turns with their git runs (the
/// second would read the first's half-written entry: `repository_lock`);
/// two in different repositories run at once.
#[cfg(unix)]
#[tokio::test]
async fn adds_in_one_repository_take_turns_and_in_two_run_at_once() {
    let sandbox = Sandbox::new();
    let (program, log, release) = gated_git(&sandbox);
    let git = sandbox.git_through(program).await;
    let one = sandbox.repo("one");
    let two = sandbox.repo("two");
    let add = |repo: &std::path::Path, name: &str| {
        let git = git.clone();
        let repo = repo.to_owned();
        let path = sandbox.path().join(name);
        let branch: BranchName = format!("vessel/{name}").parse().unwrap();
        tokio::spawn(async move {
            let start = git.head(&repo).await.unwrap().unwrap();
            git.add_worktree(
                &repo,
                &path,
                &NewBranch::Create {
                    name: branch,
                    start,
                },
            )
            .await
        })
    };

    // Different repositories: both are in git before either is let go.
    let (a, b) = (add(&one, "a"), add(&two, "b"));
    let _both = once_started(&log, 2).await;
    std::fs::write(&release, "").unwrap();
    for added in [a, b] {
        assert!(matches!(added.await.unwrap(), Ok(Ok(()))));
    }

    // One repository: the second starts only once the first has ended.
    std::fs::remove_file(&release).unwrap();
    std::fs::remove_file(&log).unwrap();
    let (c, d) = (add(&one, "c"), add(&one, "d"));
    let _first = once_started(&log, 1).await;
    // Time for a second add to start, were it not waiting its turn.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let early = std::fs::read_to_string(&log).unwrap();
    assert_eq!(early.lines().collect::<Vec<_>>(), ["start"], "{early}");
    std::fs::write(&release, "").unwrap();
    for added in [c, d] {
        assert!(matches!(added.await.unwrap(), Ok(Ok(()))));
    }
    let all = std::fs::read_to_string(&log).unwrap();
    assert_eq!(
        all.lines().collect::<Vec<_>>(),
        ["start", "end", "start", "end"],
        "{all}"
    );
}

/// The created branch's commits counted only by a `rev-list` that ran: a
/// failure that is not the branch being gone keeps the worktree.
#[tokio::test]
async fn a_branch_git_cannot_read_is_never_counted_as_nothing() {
    let sandbox = Sandbox::new();
    let (repo, path, branch, git) = setup(&sandbox).await;
    sandbox.run(&path, &["commit", "-q", "--allow-empty", "-m", "doomed"]);
    let head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&path)
        .output()
        .unwrap();
    let head = String::from_utf8(head.stdout).unwrap();
    let head = head.trim();
    // The worktree moves to another branch; the created one keeps a commit
    // whose object is then lost.
    sandbox.run(&path, &["checkout", "-q", "-b", "other", "main"]);
    let (fan, rest) = head.split_at(2);
    std::fs::remove_file(repo.join(".git/objects").join(fan).join(rest)).unwrap();
    assert!(
        git.would_lose(&path, Some(&branch)).await.is_err(),
        "an unreadable branch is not one with nothing on it"
    );

    // Deleted by hand, it has nothing to lose.
    sandbox.run(&repo, &["update-ref", "-d", "refs/heads/vessel/test"]);
    assert_eq!(git.would_lose(&path, Some(&branch)).await.unwrap(), []);
}

/// A commit that lands on the created branch after the inspection keeps
/// the branch, though its folder goes.
#[tokio::test]
async fn a_commit_after_the_inspection_keeps_the_branch() {
    let sandbox = Sandbox::new();
    let (repo, path, branch, git) = setup(&sandbox).await;
    assert_eq!(git.would_lose(&path, Some(&branch)).await.unwrap(), []);
    sandbox.run(&path, &["commit", "-q", "--allow-empty", "-m", "late"]);
    git.remove_worktree(&repo, &path, Some(&branch), false)
        .await
        .unwrap();
    assert!(!path.exists());
    let branches = git.branches(&repo).await.unwrap();
    assert!(
        branches
            .branches
            .iter()
            .any(|found| found.name.as_str() == "vessel/test"),
        "the branch keeps the late commit"
    );
}
