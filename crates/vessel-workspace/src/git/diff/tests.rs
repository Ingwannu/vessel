//! Every scope, a file's hunks, untracked, binary and mode changes,
//! against real repositories.

use vessel_proto::workspace::{
    ChangeStatus, DiffContent, DiffScope, DiffSummary, LineKind, ModeChange, WorktreeBase,
};

use crate::testing::Sandbox;

#[tokio::test]
async fn every_scope_lists_what_it_compares() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("r");
    let git = sandbox.git().await;
    sandbox.run(&repo, &["checkout", "-q", "-b", "topic"]);
    Sandbox::write(&repo, "committed.txt", "c\n");
    sandbox.run(&repo, &["add", "."]);
    sandbox.run(&repo, &["commit", "-q", "-m", "c"]);
    Sandbox::write(&repo, "README.md", "hello\nworld\n");
    Sandbox::write(&repo, "staged.txt", "s\n");
    sandbox.run(&repo, &["add", "staged.txt"]);
    Sandbox::write(&repo, "untracked.txt", "u1\nu2\n");
    let main = WorktreeBase::Ref {
        name: "main".parse().unwrap(),
    };
    let paths = |summary: &DiffSummary| {
        summary
            .files
            .iter()
            .map(|file| (file.path.as_str().to_owned(), file.status, file.additions))
            .collect::<Vec<_>>()
    };
    let (git, repo, main) = (&git, &repo, &main);
    let scope = |scope| async move {
        let resolved = git.resolve_scope(repo, &scope, Some(main)).await.unwrap();
        git.changes(repo, &resolved, false).await.unwrap()
    };
    let unstaged = scope(DiffScope::Unstaged).await;
    assert_eq!(
        paths(&unstaged),
        [
            ("README.md".to_owned(), ChangeStatus::Modified, Some(1)),
            ("untracked.txt".to_owned(), ChangeStatus::Added, Some(2)),
        ]
    );
    let staged = scope(DiffScope::Staged).await;
    assert_eq!(
        paths(&staged),
        [("staged.txt".to_owned(), ChangeStatus::Added, Some(1))]
    );
    let uncommitted = scope(DiffScope::Uncommitted).await;
    assert_eq!(uncommitted.files.len(), 3);
    let since_base = scope(DiffScope::Base { base: None }).await;
    assert_eq!(
        since_base
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        ["README.md", "committed.txt", "staged.txt", "untracked.txt"],
        "committed and uncommitted together"
    );
    let head = git.head(repo).await.unwrap().unwrap();
    let one = scope(DiffScope::Commit { id: head }).await;
    assert_eq!(
        paths(&one),
        [("committed.txt".to_owned(), ChangeStatus::Added, Some(1))]
    );
}

#[tokio::test]
async fn a_file_s_diff_is_typed_hunks_and_untracked_files_are_all_added() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("r");
    let git = sandbox.git().await;
    Sandbox::write(&repo, "README.md", "hello\nworld\n");
    Sandbox::write(&repo, "new.txt", "a\nb");
    let resolved = git
        .resolve_scope(&repo, &DiffScope::Uncommitted, None)
        .await
        .unwrap();
    let readme = git
        .file_diff(&repo, &resolved, &"README.md".parse().unwrap(), false, 0)
        .await
        .unwrap();
    let DiffContent::Hunks { hunks } = &readme.content else {
        panic!("{readme:?}");
    };
    let lines: Vec<_> = hunks
        .first()
        .unwrap()
        .lines
        .iter()
        .map(|line| (line.kind, line.text.as_str()))
        .collect();
    assert_eq!(
        lines,
        [(LineKind::Context, "hello"), (LineKind::Added, "world")]
    );
    let new = git
        .file_diff(&repo, &resolved, &"new.txt".parse().unwrap(), false, 0)
        .await
        .unwrap();
    assert_eq!(new.status, ChangeStatus::Added);
    let DiffContent::Hunks { hunks } = &new.content else {
        panic!("{new:?}");
    };
    assert_eq!(hunks.first().unwrap().lines.len(), 2);
    let same = git
        .file_diff(&repo, &resolved, &"nothing.txt".parse().unwrap(), false, 0)
        .await
        .unwrap();
    assert_eq!(same.content, DiffContent::Unchanged);
}

#[cfg(unix)]
#[tokio::test]
async fn a_mode_change_and_a_binary_file_say_so() {
    use std::os::unix::fs::PermissionsExt;
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("r");
    let git = sandbox.git().await;
    Sandbox::write(&repo, "bin.dat", "a\0b");
    sandbox.run(&repo, &["add", "bin.dat"]);
    sandbox.run(&repo, &["commit", "-q", "-m", "bin"]);
    std::fs::write(repo.join("bin.dat"), b"c\0d").unwrap();
    std::fs::set_permissions(
        repo.join("README.md"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let resolved = git
        .resolve_scope(&repo, &DiffScope::Uncommitted, None)
        .await
        .unwrap();
    let summary = git.changes(&repo, &resolved, false).await.unwrap();
    let binary = summary
        .files
        .iter()
        .find(|file| file.path.as_str() == "bin.dat")
        .unwrap();
    assert!(binary.binary);
    assert_eq!(binary.additions, None);
    let bin = git
        .file_diff(&repo, &resolved, &"bin.dat".parse().unwrap(), false, 0)
        .await
        .unwrap();
    assert_eq!(bin.content, DiffContent::Binary);
    let mode = git
        .file_diff(&repo, &resolved, &"README.md".parse().unwrap(), false, 0)
        .await
        .unwrap();
    assert_eq!(
        mode.mode,
        Some(ModeChange {
            old: 0o100_644,
            new: 0o100_755
        })
    );
}
