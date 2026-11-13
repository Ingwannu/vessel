//! ADR 0030 in the node: git's own folders refused as projects, and no
//! removal between a session's place being checked and the session being
//! live (wave 1's review, findings 7 and 17); and, protocol 11, none
//! between a terminal's start in a worktree and its registration (the
//! window track C left open, closed in the join of wave 2's first half).

use vessel_proto::workspace::{IfLost, KeptReason, RemoveWorktreeOutcome, SessionPlace};

use super::{Fixture, WorkspaceError};

#[tokio::test]
async fn git_s_own_folders_are_never_projects() {
    let fixture = Fixture::new().await;
    let root = fixture.repo("app", None);
    // The `.git` folder, a folder inside it, and a bare repository.
    let bare = fixture.temp.path().join("bare.git");
    let _out = fixture.git(fixture.temp.path(), &["init", "-q", "--bare", "bare.git"]);
    let named = fixture.temp.path().join("plain-name");
    let _out = fixture.git(fixture.temp.path(), &["init", "-q", "--bare", "plain-name"]);
    for folder in [
        root.join(".git"),
        root.join(".git").join("hooks"),
        bare,
        named,
    ] {
        let refused = fixture.workspaces.add_project(folder.clone(), None).await;
        assert!(
            matches!(refused, Err(WorkspaceError::Invalid(ref why)) if why.contains("git's own folder") || why.contains(".git")),
            "{}: {refused:?}",
            folder.display()
        );
    }
    // `project.init` at a `.git` path is refused before anything is made.
    let inside = root.join(".git").join("new");
    let refused = fixture.workspaces.init_project(inside.clone(), None).await;
    assert!(
        matches!(refused, Err(WorkspaceError::Invalid(_))),
        "{refused:?}"
    );
    assert!(!inside.exists());
    // The working tree itself is still a project.
    assert!(fixture.workspaces.add_project(root, None).await.is_ok());
}

#[tokio::test]
async fn a_session_starting_in_a_worktree_keeps_it_from_removal() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", None).await;
    let worktree = fixture.created(project.id).await;
    let place = SessionPlace::Worktree {
        worktree: worktree.id,
    };
    let first = fixture.workspaces.place(&place).await.unwrap();
    let second = fixture.workspaces.place(&place).await.unwrap();
    let resumed = fixture.workspaces.start_in(worktree.id).unwrap();
    let mut held = vec![first.starting, second.starting, Some(resumed)];
    // Checked, not live yet: no live session names the folder, and still
    // the removal keeps it, until the last one is given back.
    while !held.is_empty() {
        let kept = fixture
            .workspaces
            .remove_worktree(worktree.id, IfLost::Discard, Vec::new)
            .await
            .unwrap();
        assert_eq!(
            kept,
            RemoveWorktreeOutcome::Kept {
                reasons: vec![KeptReason::InUse]
            },
            "with {} still starting",
            held.len()
        );
        drop(held.pop());
    }
    let removed = fixture
        .workspaces
        .remove_worktree(worktree.id, IfLost::Discard, Vec::new)
        .await
        .unwrap();
    assert!(
        matches!(removed, RemoveWorktreeOutcome::Removed { .. }),
        "{removed:?}"
    );
}

/// The live sessions are read once the removal holds the worktree: from
/// then on nothing starts in it, so a session cannot slip in between the
/// reading and the removal.
#[tokio::test]
async fn nothing_starts_once_a_removal_reads_the_live_sessions() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", None).await;
    let worktree = fixture.created(project.id).await;
    let workspaces = std::sync::Arc::clone(&fixture.workspaces);
    let removed = fixture
        .workspaces
        .remove_worktree(worktree.id, IfLost::Discard, || {
            assert!(
                workspaces.start_in(worktree.id).is_none(),
                "a start again is refused while the removal holds it"
            );
            Vec::new()
        })
        .await
        .unwrap();
    assert!(
        matches!(removed, RemoveWorktreeOutcome::Removed { .. }),
        "{removed:?}"
    );
}

/// A terminal or a script's run starting in a worktree holds it from
/// removal until it is registered (then the removal reads it among what
/// runs there): a removal meanwhile keeps the worktree.
#[tokio::test]
async fn a_terminal_starting_in_a_worktree_keeps_it_from_removal() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", None).await;
    let worktree = fixture.created(project.id).await;
    let inside = worktree.path.join("src");
    let held = fixture.workspaces.hold_folder(&inside).unwrap();
    assert!(held.is_some(), "a folder in a worktree is held");
    let kept = fixture
        .workspaces
        .remove_worktree(worktree.id, IfLost::Discard, Vec::new)
        .await
        .unwrap();
    assert_eq!(
        kept,
        RemoveWorktreeOutcome::Kept {
            reasons: vec![KeptReason::InUse]
        }
    );
    drop(held);
    assert!(
        fixture
            .workspaces
            .hold_folder(fixture.temp.path())
            .unwrap()
            .is_none(),
        "a folder in no worktree holds nothing"
    );
    let removed = fixture
        .workspaces
        .remove_worktree(worktree.id, IfLost::Discard, Vec::new)
        .await
        .unwrap();
    assert!(
        matches!(removed, RemoveWorktreeOutcome::Removed { .. }),
        "{removed:?}"
    );
}

/// Once a removal reads what runs in the worktree it holds the worktree,
/// so a terminal starting then is refused before anything runs: either
/// the removal sees a start, or the start sees the removal.
#[tokio::test]
async fn no_terminal_starts_once_a_removal_reads_what_runs() {
    let fixture = Fixture::new().await;
    let project = fixture.project("app", None).await;
    let worktree = fixture.created(project.id).await;
    let workspaces = std::sync::Arc::clone(&fixture.workspaces);
    let path = worktree.path.clone();
    let removed = fixture
        .workspaces
        .remove_worktree(worktree.id, IfLost::Discard, || {
            assert!(
                matches!(
                    workspaces.hold_folder(&path),
                    Err(WorkspaceError::Conflict(_))
                ),
                "a terminal is refused while the removal holds the worktree"
            );
            Vec::new()
        })
        .await
        .unwrap();
    assert!(
        matches!(removed, RemoveWorktreeOutcome::Removed { .. }),
        "{removed:?}"
    );
}
