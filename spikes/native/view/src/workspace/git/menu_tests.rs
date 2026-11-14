//! Toolbar cases use the page's panel-model and git-bar order. The
//! existing `reference_tests` also retain Paseo/GOAT policy fixtures.

use vessel_proto::workspace::{FileState, GitStatus, StatusEntry};

use super::menu::*;
use super::{Reading, RequestKeys, Writing, fixtures};
use crate::workspace::git_quick::{Blocked, GitAction};

fn changed() -> GitStatus {
    let mut status = fixtures::status(vec![StatusEntry {
        path: "src/changed.rs".parse().unwrap(),
        old_path: None,
        index: FileState::Modified,
        worktree: FileState::Modified,
    }]);
    status.head = Some("1111111111111111111111111111111111111111".parse().unwrap());
    status
}

#[test]
fn menu_order_and_group_rules_keep_every_action_reachable() {
    let status = Reading::Ready(changed());
    let writing = Writing::default();
    let rows = rows(&status, &writing);
    assert_eq!(
        rows.iter().map(|row| row.action).collect::<Vec<_>>(),
        vec![
            Action::Quick(GitAction::Commit),
            Action::Quick(GitAction::Push),
            Action::Quick(GitAction::Pull),
            Action::Quick(GitAction::Fetch),
            Action::SwitchBranch,
            Action::CreateBranch,
            Action::RenameBranch,
            Action::SaveStash(StashScope::Tracked),
            Action::SaveStash(StashScope::WithUntracked),
            Action::Stashes,
        ]
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.starts_group)
            .map(|row| row.action)
            .collect::<Vec<_>>(),
        vec![Action::SwitchBranch, Action::SaveStash(StashScope::Tracked)]
    );
    assert_eq!(
        availability(Action::Quick(GitAction::Pull), &status, &writing),
        Some(Unavailable::Git(Blocked::NoUpstream))
    );
    assert_eq!(
        availability(Action::Quick(GitAction::Push), &status, &writing),
        None
    );
    assert_eq!(availability(Action::RenameBranch, &status, &writing), None);
}

#[test]
fn every_action_is_gated_while_a_write_or_refresh_owns_the_facts() {
    let mut keys = RequestKeys::default();
    let mut status = Reading::Ready(changed());
    let mut writing = Writing::default();
    assert!(writing.begin(keys.issue().unwrap()));
    assert!(
        rows(&status, &writing)
            .iter()
            .all(|row| row.unavailable == Some(Unavailable::Writing))
    );
    writing = Writing::default();
    status.start(keys.issue().unwrap());
    assert_eq!(primary(&status), Some(Action::Quick(GitAction::Commit)));
    assert!(
        rows(&status, &writing)
            .iter()
            .all(|row| row.unavailable == Some(Unavailable::Reading))
    );
    assert_eq!(primary(&Reading::Empty), None);
    assert_eq!(
        primary(&Reading::Failed(crate::workspace::problem::Problem::Older)),
        None
    );
}

#[test]
fn clean_and_detached_reasons_do_not_expose_invalid_rename_or_stash() {
    let mut status = fixtures::status(vec![]);
    status.branch = None;
    status.head = Some("1111111111111111111111111111111111111111".parse().unwrap());
    let status = Reading::Ready(status);
    let writing = Writing::default();
    assert_eq!(primary(&status), None);
    assert_eq!(
        availability(Action::RenameBranch, &status, &writing),
        Some(Unavailable::NoBranch)
    );
    for scope in [StashScope::Tracked, StashScope::WithUntracked] {
        assert_eq!(
            availability(Action::SaveStash(scope), &status, &writing),
            Some(Unavailable::NoChanges)
        );
    }
    assert_eq!(
        availability(Action::Quick(GitAction::Commit), &status, &writing),
        Some(Unavailable::Git(Blocked::Clean))
    );
}

#[test]
fn branch_filter_uses_the_page_trim_and_case_rules_without_a_second_limit() {
    let branch = vessel_proto::workspace::Branch {
        name: "origin/Feature/Rendering".parse().unwrap(),
        remote: Some("origin".parse().unwrap()),
        current: false,
        default: false,
        head: "1111111111111111111111111111111111111111".parse().unwrap(),
        time: "2026-11-12T09:00:00Z".parse().unwrap(),
        upstream: None,
        worktree: None,
    };
    let branches = vec![branch.clone(); 235];
    assert_eq!(filter_branches(&branches, " FEATURE/ "), branches);
    assert_eq!(filter_branches(&[branch], "other"), vec![]);
}

#[test]
fn paseo_clean_synced_base_fixture_uses_the_menu_only_shape_with_fetch_inside() {
    let mut status = fixtures::status(vec![]);
    status.branch = Some("main".parse().unwrap());
    status.head = Some("1111111111111111111111111111111111111111".parse().unwrap());
    status.upstream = Some(vessel_proto::workspace::Distance {
        name: "origin/main".parse().unwrap(),
        ahead: 0,
        behind: 0,
    });
    let status = Reading::Ready(status);
    assert_eq!(primary(&status), None);
    let actions = rows(&status, &Writing::default());
    assert_eq!(actions[3].action, Action::Quick(GitAction::Fetch));
    assert_eq!(actions[3].unavailable, None);
    assert_eq!(
        actions[0].unavailable,
        Some(Unavailable::Git(Blocked::Clean))
    );
    assert_eq!(
        actions[1].unavailable,
        Some(Unavailable::Git(Blocked::NothingToPush))
    );
}

#[test]
fn a_narrow_or_side_panel_gives_the_comparison_room_before_drawing_a_label() {
    assert_eq!(PanelWidth(390.0).density(), ControlDensity::IconOnly);
    assert_eq!(PanelWidth(440.0).density(), ControlDensity::IconOnly);
    assert_eq!(PanelWidth(479.0).density(), ControlDensity::IconOnly);
    assert_eq!(PanelWidth(480.0).density(), ControlDensity::Labelled);
    assert_eq!(PanelWidth(1280.0).density(), ControlDensity::Labelled);
}

#[test]
fn an_unborn_repository_names_the_first_commit_needed_before_sync_or_stash() {
    let status = Reading::Ready(fixtures::status(vec![]));
    assert_eq!(primary(&status), None);
    for action in [
        Action::Quick(GitAction::Push),
        Action::Quick(GitAction::Pull),
        Action::CreateBranch,
        Action::SaveStash(StashScope::Tracked),
        Action::SaveStash(StashScope::WithUntracked),
    ] {
        assert_eq!(
            availability(action, &status, &Writing::default()),
            Some(Unavailable::NoCommits)
        );
    }
    let mut changed = changed();
    changed.head = None;
    let status = Reading::Ready(changed);
    assert_eq!(primary(&status), Some(Action::Quick(GitAction::Commit)));
    assert_eq!(
        availability(
            Action::Quick(GitAction::Commit),
            &status,
            &Writing::default()
        ),
        None
    );
}

#[test]
fn the_primary_half_names_its_action_only_with_room_and_spins_only_for_its_own_write() {
    let status = Reading::Ready(changed());
    let idle = Writing::default();
    assert_eq!(
        primary_control(&status, &idle, PanelWidth(640.0)),
        PrimaryControl {
            action: Some(Action::Quick(GitAction::Commit)),
            running: false,
            labelled: true,
        }
    );
    assert_eq!(
        primary_control(&status, &idle, PanelWidth(440.0)),
        PrimaryControl {
            action: Some(Action::Quick(GitAction::Commit)),
            running: false,
            labelled: false,
        }
    );
    let mut keys = RequestKeys::default();
    let mut committing = Writing::default();
    assert!(committing.begin_action(
        keys.issue().unwrap(),
        super::WriteIntent::Quick(GitAction::Commit)
    ));
    assert!(primary_control(&status, &committing, PanelWidth(640.0)).running);
    let mut fetching = Writing::default();
    assert!(fetching.begin_action(
        keys.issue().unwrap(),
        super::WriteIntent::Quick(GitAction::Fetch)
    ));
    assert!(!primary_control(&status, &fetching, PanelWidth(640.0)).running);
}

#[test]
fn nothing_needed_leaves_the_primary_half_to_open_the_menu() {
    let mut clean = fixtures::status(vec![]);
    clean.head = Some("1111111111111111111111111111111111111111".parse().unwrap());
    clean.upstream = Some(vessel_proto::workspace::Distance {
        name: "origin/feature".parse().unwrap(),
        ahead: 0,
        behind: 0,
    });
    assert_eq!(
        primary_control(
            &Reading::Ready(clean),
            &Writing::default(),
            PanelWidth(640.0)
        ),
        PrimaryControl {
            action: None,
            running: false,
            labelled: false,
        }
    );
}
