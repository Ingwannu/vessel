//! The page's use-git outcomes: a success refreshes, a typed refusal
//! remains in place, and an index write uses its returned status.

use vessel_proto::workspace::{
    CommitOutcome, FetchOutcome, GitRefusal, PullOutcome, PushOutcome, RenameOutcome, StashList,
    StashOutcome, SwitchOutcome,
};

use super::super::problem::Problem;
use super::*;

#[test]
fn every_git_write_keeps_a_typed_refusal() {
    let refusal = GitRefusal::WouldOverwrite {
        paths: vec!["src/a.ts".parse().unwrap()],
    };
    let outcomes = [
        Written::from(CommitOutcome::Refused {
            refusal: refusal.clone(),
        }),
        Written::from(FetchOutcome::Refused {
            refusal: refusal.clone(),
        }),
        Written::from(PushOutcome::Refused {
            refusal: refusal.clone(),
        }),
        Written::from(PullOutcome::Refused {
            refusal: refusal.clone(),
        }),
        Written::from(RenameOutcome::Refused {
            refusal: refusal.clone(),
        }),
        Written::from(SwitchOutcome::Refused {
            refusal: refusal.clone(),
        }),
        Written::from(StashOutcome::Refused {
            refusal: refusal.clone(),
        }),
    ];
    for outcome in outcomes {
        assert_eq!(outcome, Written::Refused(refusal.clone()));
    }
}

#[test]
fn commit_success_keeps_its_id_and_subject_for_the_success_text() {
    let commit = vessel_proto::workspace::CommitInfo {
        id: "0123456789abcdef0123456789abcdef01234567".parse().unwrap(),
        parents: vec![],
        author: "Test".to_owned(),
        time: "2026-11-12T10:00:00Z".parse().unwrap(),
        subject: "A change".to_owned(),
    };
    assert_eq!(
        Written::from(CommitOutcome::Committed {
            commit: commit.clone()
        }),
        Written::Said(Success::Committed(commit))
    );
}

#[test]
fn fetch_and_pull_successes_distinguish_a_fast_forward_from_no_change() {
    assert_eq!(
        Written::from(FetchOutcome::Fetched),
        Written::Said(Success::Fetched)
    );
    assert_eq!(
        Written::from(PullOutcome::UpToDate),
        Written::Said(Success::UpToDate)
    );
    let from: vessel_proto::workspace::CommitId =
        "abcdef0123456789abcdef0123456789abcdef01".parse().unwrap();
    let to: vessel_proto::workspace::CommitId =
        "0123456789abcdef0123456789abcdef01234567".parse().unwrap();
    assert_eq!(
        Written::from(PullOutcome::Pulled {
            from: from.clone(),
            to: to.clone()
        }),
        Written::Said(Success::Pulled { from, to })
    );
}

#[test]
fn push_success_keeps_the_branch_and_whether_it_set_an_upstream() {
    for (upstream_set, upstream) in [(false, UpstreamChange::Kept), (true, UpstreamChange::Set)] {
        let pushed = PushOutcome::Pushed {
            remote: "origin".parse().unwrap(),
            branch: "feature".parse().unwrap(),
            upstream_set,
        };
        assert_eq!(
            Written::from(pushed),
            Written::Said(Success::Pushed {
                remote: "origin".parse().unwrap(),
                branch: "feature".parse().unwrap(),
                upstream
            })
        );
    }
}

#[test]
fn branch_and_stash_successes_keep_the_new_names_and_list() {
    assert_eq!(
        Written::from(SwitchOutcome::Switched {
            branch: "feature".parse().unwrap()
        }),
        Written::Said(Success::Switched("feature".parse().unwrap()))
    );
    assert_eq!(
        Written::from(RenameOutcome::Renamed {
            branch: "new-name".parse().unwrap()
        }),
        Written::Said(Success::Renamed("new-name".parse().unwrap()))
    );
    let stashes = StashList {
        entries: vec![],
        more: false,
    };
    assert_eq!(
        Written::from(StashOutcome::Done {
            stashes: stashes.clone()
        }),
        Written::Said(Success::Stashed(stashes))
    );
    let status = fixtures::status(vec![]);
    assert_eq!(Written::from(status.clone()), Written::Status(status));
}

#[test]
fn only_one_write_can_run_and_only_its_own_reply_can_end_it() {
    let mut keys = RequestKeys::default();
    let (first, other) = (keys.issue().unwrap(), keys.issue().unwrap());
    let mut writing = Writing::default();
    assert!(writing.begin(first));
    assert!(!writing.begin(other));
    assert!(writing.busy());
    assert_eq!(
        writing.finish(other, Ok(Written::Said(Success::Fetched))),
        None
    );
    assert!(writing.busy());
    assert_eq!(
        writing.finish(first, Err(Problem::Older)),
        Some(Err(Problem::Older))
    );
    assert!(!writing.busy());
    assert_eq!(
        writing.finish(first, Ok(Written::Said(Success::Fetched))),
        None
    );
    assert!(writing.begin(other));
    assert_eq!(
        writing.finish(first, Ok(Written::Refused(GitRefusal::NoIdentity))),
        None
    );
    assert!(writing.busy());
}

#[test]
fn a_late_reply_cannot_clear_or_replace_another_requests_pending_label() {
    use super::WriteIntent;
    use crate::workspace::git_quick::GitAction;
    let mut keys = RequestKeys::default();
    let first = keys.issue().unwrap();
    let other = keys.issue().unwrap();
    let mut writing = Writing::default();
    assert!(writing.begin_action(first, WriteIntent::Quick(GitAction::Push)));
    assert!(!writing.begin_action(other, WriteIntent::Quick(GitAction::Pull)));
    assert_eq!(
        writing.finish(other, Ok(Written::Said(Success::Fetched))),
        None
    );
    assert_eq!(writing.intent(), Some(&WriteIntent::Quick(GitAction::Push)));
    assert_eq!(
        writing.finish(first, Err(Problem::Older)),
        Some(Err(Problem::Older))
    );
    assert_eq!(writing.intent(), None);
}
