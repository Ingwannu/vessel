//! Choosing an action: the page's `git-bar.tsx` requests and Paseo's
//! explain-instead-of-send rule (`git/use-actions.tsx` 41–58).

use vessel_proto::workspace::{
    CommitId, Distance, Fetch, FileState, Pull, Push, StashAction, StashChanges, StatusEntry,
    WorkspaceRoot,
};

use super::super::fixtures::status;
use super::super::{Action, BranchMode, Reading, RequestKeys, StashScope, Unavailable, Writing};
use super::{DirectWrite, Step, step};
use crate::workspace::git::WriteIntent;
use crate::workspace::git_quick::{Blocked, GitAction};

fn root() -> WorkspaceRoot {
    WorkspaceRoot::Project {
        project: "00000000-0000-0000-0000-000000000001".parse().unwrap(),
    }
}

fn head() -> CommitId {
    "0123456789abcdef0123456789abcdef01234567".parse().unwrap()
}

fn changed() -> StatusEntry {
    StatusEntry {
        path: "src/a.ts".parse().unwrap(),
        old_path: None,
        index: FileState::Unmodified,
        worktree: FileState::Modified,
    }
}

fn upstream(ahead: u32, behind: u32) -> Distance {
    Distance {
        name: "origin/feature".parse().unwrap(),
        ahead,
        behind,
    }
}

fn ready(
    files: Vec<StatusEntry>,
    upstream: Option<Distance>,
) -> Reading<vessel_proto::workspace::GitStatus> {
    let mut facts = status(files);
    facts.head = Some(head());
    facts.upstream = upstream;
    Reading::Ready(facts)
}

#[test]
fn a_branch_without_an_upstream_is_pushed_and_set_as_its_upstream() {
    let read = ready(vec![], None);
    assert_eq!(
        step(
            Action::Quick(GitAction::Push),
            root(),
            &read,
            &Writing::default()
        ),
        Step::Write(
            DirectWrite::Push(Push {
                root: root(),
                remote: None,
                set_upstream: true,
            }),
            WriteIntent::Quick(GitAction::Push),
        )
    );
}

#[test]
fn a_branch_with_an_upstream_is_pushed_without_changing_it() {
    let read = ready(vec![], Some(upstream(2, 0)));
    assert_eq!(
        step(
            Action::Quick(GitAction::Push),
            root(),
            &read,
            &Writing::default()
        ),
        Step::Write(
            DirectWrite::Push(Push {
                root: root(),
                remote: None,
                set_upstream: false,
            }),
            WriteIntent::Quick(GitAction::Push),
        )
    );
}

#[test]
fn pull_and_fetch_send_their_requests_for_the_shown_root() {
    let behind = ready(vec![], Some(upstream(0, 3)));
    assert_eq!(
        step(
            Action::Quick(GitAction::Pull),
            root(),
            &behind,
            &Writing::default()
        ),
        Step::Write(
            DirectWrite::Pull(Pull { root: root() }),
            WriteIntent::Quick(GitAction::Pull),
        )
    );
    assert_eq!(
        step(
            Action::Quick(GitAction::Fetch),
            root(),
            &behind,
            &Writing::default()
        ),
        Step::Write(
            DirectWrite::Fetch(Fetch {
                root: root(),
                remote: None,
            }),
            WriteIntent::Quick(GitAction::Fetch),
        )
    );
}

#[test]
fn forms_open_instead_of_writing() {
    let read = ready(vec![changed()], Some(upstream(0, 0)));
    let writing = Writing::default();
    assert_eq!(
        step(Action::Quick(GitAction::Commit), root(), &read, &writing),
        Step::Commit
    );
    assert_eq!(
        step(Action::SwitchBranch, root(), &read, &writing),
        Step::Switch
    );
    assert_eq!(
        step(Action::CreateBranch, root(), &read, &writing),
        Step::Branch(BranchMode::Create)
    );
    assert_eq!(
        step(Action::RenameBranch, root(), &read, &writing),
        Step::Branch(BranchMode::Rename)
    );
    assert_eq!(
        step(Action::Stashes, root(), &read, &writing),
        Step::Stashes
    );
}

#[test]
fn stashing_saves_with_or_without_new_files() {
    let read = ready(vec![changed()], Some(upstream(0, 0)));
    for (scope, untracked) in [
        (StashScope::Tracked, false),
        (StashScope::WithUntracked, true),
    ] {
        assert_eq!(
            step(Action::SaveStash(scope), root(), &read, &Writing::default()),
            Step::Write(
                DirectWrite::Stash(StashChanges {
                    root: root(),
                    action: StashAction::Save {
                        message: None,
                        untracked,
                    },
                }),
                WriteIntent::SaveStash(scope),
            )
        );
    }
}

#[test]
fn an_unavailable_action_is_explained_and_nothing_is_sent() {
    let clean = ready(vec![], Some(upstream(0, 0)));
    let writing = Writing::default();
    assert_eq!(
        step(Action::Quick(GitAction::Commit), root(), &clean, &writing),
        Step::Explain(Unavailable::Git(Blocked::Clean))
    );
    assert_eq!(
        step(Action::Quick(GitAction::Push), root(), &clean, &writing),
        Step::Explain(Unavailable::Git(Blocked::NothingToPush))
    );
    assert_eq!(
        step(
            Action::SaveStash(StashScope::Tracked),
            root(),
            &clean,
            &writing
        ),
        Step::Explain(Unavailable::NoChanges)
    );
}

#[test]
fn nothing_runs_from_facts_that_are_missing_refreshing_failed_or_busy() {
    let writing = Writing::default();
    assert_eq!(
        step(
            Action::Quick(GitAction::Fetch),
            root(),
            &Reading::Empty,
            &writing
        ),
        Step::Explain(Unavailable::StatusMissing)
    );
    assert_eq!(
        step(
            Action::Quick(GitAction::Fetch),
            root(),
            &Reading::Failed(crate::workspace::problem::Problem::Older),
            &writing
        ),
        Step::Explain(Unavailable::StatusFailed)
    );
    let mut keys = RequestKeys::default();
    let mut refreshing = ready(vec![], Some(upstream(0, 0)));
    refreshing.start(keys.issue().unwrap());
    assert_eq!(
        step(
            Action::Quick(GitAction::Fetch),
            root(),
            &refreshing,
            &writing
        ),
        Step::Explain(Unavailable::Reading)
    );
    let mut busy = Writing::default();
    assert!(busy.begin(keys.issue().unwrap()));
    assert_eq!(
        step(
            Action::Quick(GitAction::Fetch),
            root(),
            &ready(vec![], Some(upstream(0, 0))),
            &busy
        ),
        Step::Explain(Unavailable::Writing)
    );
}

#[test]
fn a_form_opens_on_the_shown_facts_while_a_refresh_is_out_but_a_write_waits() {
    let mut keys = RequestKeys::default();
    let writing = Writing::default();
    let mut changed_refreshing = ready(vec![changed()], Some(upstream(0, 0)));
    changed_refreshing.start(keys.issue().unwrap());
    assert_eq!(
        step(
            Action::Quick(GitAction::Commit),
            root(),
            &changed_refreshing,
            &writing
        ),
        Step::Commit
    );
    assert_eq!(
        step(Action::SwitchBranch, root(), &changed_refreshing, &writing),
        Step::Switch
    );
    assert_eq!(
        step(
            Action::SaveStash(StashScope::Tracked),
            root(),
            &changed_refreshing,
            &writing
        ),
        Step::Explain(Unavailable::Reading)
    );
    let mut clean_refreshing = ready(vec![], Some(upstream(0, 0)));
    clean_refreshing.start(keys.issue().unwrap());
    assert_eq!(
        step(
            Action::Quick(GitAction::Commit),
            root(),
            &clean_refreshing,
            &writing
        ),
        Step::Explain(Unavailable::Git(Blocked::Clean))
    );
}
