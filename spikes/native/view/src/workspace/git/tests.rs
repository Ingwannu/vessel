//! The page's commit-dialog, branch-dialog, stash-dialog and
//! discard-dialog inputs, plus adversarial wire-constructor boundaries.

use vessel_proto::workspace::{
    Branch, CommitStage, FileState, PathSet, StashAction, StashEntry, StatusEntry, WorkspaceRoot,
    WorktreeBase,
};

use super::fixtures::status;
use super::*;

fn root() -> WorkspaceRoot {
    WorkspaceRoot::Project {
        project: "00000000-0000-0000-0000-000000000001".parse().unwrap(),
    }
}

fn file(index: FileState, worktree: FileState) -> StatusEntry {
    StatusEntry {
        path: "src/a.ts".parse().unwrap(),
        old_path: None,
        index,
        worktree,
    }
}

#[test]
fn commit_defaults_to_staged_only_when_something_is_staged() {
    let working = file(FileState::Unmodified, FileState::Modified);
    let untracked = file(FileState::Untracked, FileState::Untracked);
    let first = CommitForm::of(&status(vec![working.clone(), untracked.clone()]));
    assert_eq!(first.stage, CommitStage::Everything);
    assert_eq!(first.changed.get(), 2);
    assert_eq!(first.staged.get(), 0);
    let mixed = CommitForm::of(&status(vec![
        working,
        untracked,
        file(FileState::Added, FileState::Modified),
    ]));
    assert_eq!(mixed.stage, CommitStage::Staged);
    assert_eq!(mixed.staged.get(), 1);
    assert_eq!(mixed.changed.get(), 3);
    assert_eq!(CommitForm::of(&status(vec![])).changed.get(), 0);
}

#[test]
fn index_actions_cover_both_halves_and_unknown_wire_states() {
    assert_eq!(
        file_actions(&file(FileState::Unmodified, FileState::Modified)),
        [IndexAction::Stage]
    );
    assert_eq!(
        file_actions(&file(FileState::Modified, FileState::Unmodified)),
        [IndexAction::Unstage]
    );
    assert_eq!(
        file_actions(&file(FileState::Modified, FileState::Modified)),
        [IndexAction::Stage, IndexAction::Unstage]
    );
    assert_eq!(
        file_actions(&file(FileState::Untracked, FileState::Untracked)),
        [IndexAction::Stage]
    );
    assert!(file_actions(&file(FileState::Unmodified, FileState::Unmodified)).is_empty());
    for state in [
        FileState::Added,
        FileState::Deleted,
        FileState::Renamed,
        FileState::Copied,
        FileState::TypeChanged,
        FileState::Conflicted,
        FileState::Unknown,
    ] {
        assert_eq!(
            file_actions(&file(state, state)),
            [IndexAction::Stage, IndexAction::Unstage]
        );
        assert_eq!(
            CommitForm::of(&status(vec![file(state, FileState::Unmodified)]))
                .staged
                .get(),
            1
        );
    }
}

#[test]
fn commit_trims_the_message_and_preserves_the_chosen_scope() {
    let root = root();
    for stage in [
        CommitStage::Staged,
        CommitStage::Tracked,
        CommitStage::Everything,
    ] {
        let request = commit_request(root, "  fix: one\n\nbody 한글  ", stage).unwrap();
        assert_eq!(request.root, root);
        assert_eq!(request.message.as_str(), "fix: one\n\nbody 한글");
        assert_eq!(request.stage, stage);
    }
    for bad in [
        String::new(),
        " \n\t ".to_owned(),
        "a\0b".to_owned(),
        "a".repeat(65_537),
    ] {
        assert!(commit_request(root, &bad, CommitStage::Staged).is_err());
    }
    assert!(commit_request(root, &"a".repeat(65_536), CommitStage::Staged).is_ok());
}

fn branch(name: &str, remote: Option<&str>) -> Branch {
    Branch {
        name: name.parse().unwrap(),
        remote: remote.map(|name| name.parse().unwrap()),
        current: false,
        default: false,
        head: "0123456789abcdef0123456789abcdef01234567".parse().unwrap(),
        time: "2026-11-12T10:00:00Z".parse().unwrap(),
        upstream: None,
        worktree: None,
    }
}

#[test]
fn switching_a_remote_ref_creates_the_local_branch_from_that_exact_ref() {
    let root = root();
    let local = switch_request(root, &branch("feature", None)).unwrap();
    assert_eq!(local.branch.as_str(), "feature");
    assert_eq!(local.create, None);
    let remote = switch_request(root, &branch("origin/team/feature", Some("origin"))).unwrap();
    assert_eq!(remote.branch.as_str(), "team/feature");
    assert_eq!(
        remote.create,
        Some(WorktreeBase::Ref {
            name: "origin/team/feature".parse().unwrap()
        })
    );
    assert!(matches!(
        switch_request(root, &branch("other/feature", Some("origin"))),
        Err(SwitchProblem::RemoteMismatch { .. })
    ));
    // A valid remote ref can name a local branch `HEAD`, which is invalid.
    assert!(matches!(
        switch_request(root, &branch("origin/HEAD", Some("origin"))),
        Err(SwitchProblem::InvalidName(_))
    ));
}

#[test]
fn create_and_rename_use_distinct_requests_and_wire_validation() {
    let root = root();
    let created = branch_request(root, BranchMode::Create, " team/새기능 ").unwrap();
    let BranchRequest::Create(created) = created else {
        panic!("create made a rename")
    };
    assert_eq!(created.branch.as_str(), "team/새기능");
    assert_eq!(
        created.create,
        Some(WorktreeBase::Ref {
            name: "HEAD".parse().unwrap()
        })
    );
    let BranchRequest::Rename(renamed) = branch_request(root, BranchMode::Rename, "next").unwrap()
    else {
        panic!("rename made a create")
    };
    assert_eq!(renamed.from, None);
    assert_eq!(renamed.to.as_str(), "next");
    for bad in ["", "HEAD", "-delete", "a..b", "a.lock", "with space"] {
        for mode in [BranchMode::Create, BranchMode::Rename] {
            assert!(branch_request(root, mode, bad).is_err(), "{bad}");
        }
    }
}

#[test]
fn discard_explains_restoration_and_both_kinds_of_deletion() {
    let root = root();
    for (file, effect, untracked) in [
        (
            file(FileState::Modified, FileState::Modified),
            DiscardEffect::Restore,
            false,
        ),
        (
            file(FileState::Added, FileState::Modified),
            DiscardEffect::DeleteAdded,
            false,
        ),
        (
            file(FileState::Untracked, FileState::Untracked),
            DiscardEffect::DeleteUntracked,
            true,
        ),
    ] {
        let decision = DiscardDecision::of(root, &file);
        assert_eq!(decision.path(), &file.path);
        assert_eq!(decision.effect(), effect);
        let request = decision.confirm();
        assert_eq!(request.root, root);
        assert_eq!(
            request.paths,
            PathSet::Paths {
                paths: vec![file.path]
            }
        );
        assert_eq!(request.untracked, untracked);
    }
}

#[test]
fn stash_choices_pin_the_listed_commit_and_index_in_every_request() {
    let root = root();
    let entry = StashEntry {
        index: 7,
        commit: "0123456789abcdef0123456789abcdef01234567".parse().unwrap(),
        message: "WIP on feature".to_owned(),
        time: "2026-11-12T10:00:00Z".parse().unwrap(),
    };
    for choice in [StashChoice::Apply, StashChoice::Pop, StashChoice::Drop] {
        let request = stash_request(root, &entry, choice);
        assert_eq!(request.root, root);
        let expected = match choice {
            StashChoice::Apply => StashAction::Apply {
                index: entry.index,
                commit: entry.commit.clone(),
            },
            StashChoice::Pop => StashAction::Pop {
                index: entry.index,
                commit: entry.commit.clone(),
            },
            StashChoice::Drop => StashAction::Drop {
                index: entry.index,
                commit: entry.commit.clone(),
            },
        };
        assert_eq!(request.action, expected);
    }
}
