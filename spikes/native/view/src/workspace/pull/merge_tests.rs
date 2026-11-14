//! Paseo's `canMergePr`, with what the node sends: the rows show only
//! when a merge would be taken, in Paseo's order, and the request pins
//! what was seen.

use vessel_proto::forge::{MergeMethod, MergeOutcome, MergeRefusal, Mergeable, PullState};
use vessel_proto::workspace::{Distance, GitStatus, StatusEntry};

use super::{Merged, merge_methods, merge_outcome, merge_request};
use crate::workspace::git::fixtures::status;
use crate::workspace::pull::fixtures::{HEAD, whole};

/// A clean folder at the pull request's head, in step with its remote.
fn ready() -> GitStatus {
    let mut facts = status(vec![]);
    facts.head = Some(HEAD.parse().unwrap());
    facts.upstream = Some(Distance {
        name: "origin/feature".parse().unwrap(),
        ahead: 0,
        behind: 0,
    });
    facts
}

#[test]
fn the_allowed_methods_come_in_paseo_s_order() {
    assert_eq!(
        merge_methods(&whole(), &ready()),
        vec![MergeMethod::Squash, MergeMethod::Merge]
    );
}

#[test]
fn a_draft_a_closed_one_and_a_conflict_offer_no_merge() {
    let mut draft = whole();
    draft.summary.draft = true;
    assert!(merge_methods(&draft, &ready()).is_empty());
    let mut merged = whole();
    merged.summary.state = PullState::Merged;
    assert!(merge_methods(&merged, &ready()).is_empty());
    for blocked in [
        Mergeable::Conflicting,
        Mergeable::Blocked,
        Mergeable::Behind,
        Mergeable::Unknown,
    ] {
        let mut pull = whole();
        pull.summary.mergeable = blocked;
        assert!(merge_methods(&pull, &ready()).is_empty(), "{blocked:?}");
    }
    let mut unstable = whole();
    unstable.summary.mergeable = Mergeable::Unstable;
    assert!(!merge_methods(&unstable, &ready()).is_empty());
}

#[test]
fn local_changes_unpushed_commits_or_another_head_offer_no_merge() {
    let mut dirty = ready();
    dirty.files.push(StatusEntry {
        path: "src/a.rs".parse().unwrap(),
        old_path: None,
        index: vessel_proto::workspace::FileState::Unmodified,
        worktree: vessel_proto::workspace::FileState::Modified,
    });
    assert!(merge_methods(&whole(), &dirty).is_empty());
    let mut ahead = ready();
    ahead.upstream = Some(Distance {
        name: "origin/feature".parse().unwrap(),
        ahead: 1,
        behind: 0,
    });
    assert!(merge_methods(&whole(), &ahead).is_empty());
    let mut unpushed = ready();
    unpushed.upstream = None;
    assert!(merge_methods(&whole(), &unpushed).is_empty());
    let mut moved = ready();
    moved.head = Some("89abcdef0123456789abcdef0123456789abcdef".parse().unwrap());
    assert!(merge_methods(&whole(), &moved).is_empty());
}

#[test]
fn a_worktree_with_nothing_over_its_base_offers_no_merge() {
    let mut level = ready();
    level.base = Some(Distance {
        name: "main".parse().unwrap(),
        ahead: 0,
        behind: 2,
    });
    assert!(merge_methods(&whole(), &level).is_empty());
    let mut ahead = level.clone();
    ahead.base = Some(Distance {
        name: "main".parse().unwrap(),
        ahead: 1,
        behind: 2,
    });
    assert!(!merge_methods(&whole(), &ahead).is_empty());
}

#[test]
fn the_request_pins_the_number_and_the_head_seen() {
    let session = "00000000-0000-4000-8000-000000000042".parse().unwrap();
    let request = merge_request(session, &whole(), MergeMethod::Squash);
    assert_eq!(request.number.get(), 42);
    assert_eq!(request.head.to_string(), HEAD);
    assert_eq!(request.method, MergeMethod::Squash);
}

#[test]
fn an_unknown_outcome_is_a_refusal_shown() {
    assert_eq!(
        merge_outcome(MergeOutcome::Unknown),
        Merged::Refused(MergeRefusal::Unknown)
    );
    assert_eq!(
        merge_outcome(MergeOutcome::Refused {
            refusal: MergeRefusal::Draft
        }),
        Merged::Refused(MergeRefusal::Draft)
    );
}
