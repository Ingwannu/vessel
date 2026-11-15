//! Cases from the references' own test files: Paseo's
//! git/policy.test.ts (behind-origin and no-upstream worktree cases),
//! and GOAT's GitActionsControl.logic.test.ts (dirty and behind, no PR).
//! Their upstream counts and dirty state map to the node's `GitStatus`;
//! the expected English label and availability remain the same.
//! PR, archive and local-merge promotion are outside this slice.

use vessel_proto::workspace::{Distance, FileState, StatusEntry};

use super::fixtures::status;
use crate::Lang;
use crate::workspace::git_quick::{GitAction, action_blocked, quick_action};

#[test]
fn paseo_behind_origin_promotes_pull_with_its_english_label() {
    let mut input = status(vec![]);
    input.upstream = Some(Distance {
        name: "origin/feature".parse().unwrap(),
        ahead: 0,
        behind: 2,
    });
    assert_eq!(quick_action(&input), GitAction::Pull);
    assert_eq!(Lang::En.workspace().action(quick_action(&input)), "Pull");
}

#[test]
fn paseo_no_upstream_with_local_commits_keeps_push_available() {
    let mut input = status(vec![]);
    input.head = Some("0123456789abcdef0123456789abcdef01234567".parse().unwrap());
    input.base = Some(Distance {
        name: "main".parse().unwrap(),
        ahead: 1,
        behind: 0,
    });
    assert_eq!(input.upstream, None);
    assert_eq!(action_blocked(GitAction::Push, &input), None);
    assert_eq!(Lang::En.workspace().action(GitAction::Push), "Push");
}

#[test]
fn goat_local_changes_behind_upstream_can_commit_without_pushing() {
    let mut input = status(vec![StatusEntry {
        path: "src/a.ts".parse().unwrap(),
        old_path: None,
        index: FileState::Unmodified,
        worktree: FileState::Modified,
    }]);
    input.branch = Some("feature/test".parse().unwrap());
    input.upstream = Some(Distance {
        name: "origin/feature/test".parse().unwrap(),
        ahead: 0,
        behind: 2,
    });
    assert_eq!(quick_action(&input), GitAction::Commit);
    assert_eq!(action_blocked(GitAction::Commit, &input), None);
    assert_eq!(Lang::En.workspace().action(quick_action(&input)), "Commit");
}
