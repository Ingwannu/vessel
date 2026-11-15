//! The facts as Paseo's pane reads them: the state under the title, the
//! checks' outcome and counts in its order, the verdict and the pills.

use vessel_proto::forge::{Checks, ChecksState, Mergeable, PullState, ReviewDecision, Reviews};

use super::{CheckCount, ChecksOutcome, Merge, PullPhase, ReviewPills, Verdict, check_counts};
use crate::workspace::pull::fixtures::summary;

#[test]
fn an_open_draft_reads_as_a_draft_and_a_closed_one_never_does() {
    assert_eq!(
        PullPhase::of(&summary(PullState::Open, false)),
        PullPhase::Open
    );
    assert_eq!(
        PullPhase::of(&summary(PullState::Open, true)),
        PullPhase::Draft
    );
    assert_eq!(
        PullPhase::of(&summary(PullState::Merged, true)),
        PullPhase::Merged
    );
    assert_eq!(
        PullPhase::of(&summary(PullState::Closed, true)),
        PullPhase::Closed
    );
    assert_eq!(
        PullPhase::of(&summary(PullState::Unknown, false)),
        PullPhase::Unknown
    );
}

#[test]
fn a_cancelled_check_is_a_failure_and_unknown_is_never_success() {
    assert_eq!(
        ChecksOutcome::of(ChecksState::Cancelled),
        ChecksOutcome::Failure
    );
    assert_eq!(
        ChecksOutcome::of(ChecksState::Failing),
        ChecksOutcome::Failure
    );
    assert_eq!(
        ChecksOutcome::of(ChecksState::Pending),
        ChecksOutcome::Pending
    );
    assert_eq!(
        ChecksOutcome::of(ChecksState::Passing),
        ChecksOutcome::Success
    );
    assert_eq!(ChecksOutcome::of(ChecksState::None), ChecksOutcome::None);
    assert_eq!(
        ChecksOutcome::of(ChecksState::Unknown),
        ChecksOutcome::Unknown
    );
}

#[test]
fn counts_come_failing_then_running_then_passed_and_only_above_zero() {
    let checks = Checks {
        state: ChecksState::Failing,
        passed: 5,
        failed: 2,
        pending: 0,
        truncated: false,
        failing: vec![],
    };
    assert_eq!(
        check_counts(&checks),
        vec![CheckCount::Failing(2), CheckCount::Passed(5)]
    );
    let none = Checks {
        state: ChecksState::None,
        passed: 0,
        failed: 0,
        pending: 0,
        truncated: false,
        failing: vec![],
    };
    assert!(check_counts(&none).is_empty());
}

#[test]
fn the_verdict_mergeability_and_pills_follow_the_forge() {
    assert_eq!(
        Verdict::of(ReviewDecision::ChangesRequested),
        Verdict::ChangesRequested
    );
    assert_eq!(Verdict::of(ReviewDecision::Unknown), Verdict::Unknown);
    assert_eq!(Merge::of(Mergeable::Conflicting), Merge::Conflicts);
    assert_eq!(Merge::of(Mergeable::Unknown), Merge::Unknown);
    let pills = ReviewPills::of(&Reviews {
        approved: true,
        changes_requested: false,
        commented: true,
        unresolved: 3,
    });
    assert_eq!(
        pills,
        ReviewPills {
            approved: true,
            changes_requested: false,
            unresolved: 3
        }
    );
}

#[test]
fn review_pills_remove_only_the_same_verdict_and_preserve_different_facts() {
    for approved in [false, true] {
        for changes_requested in [false, true] {
            let pills = ReviewPills {
                approved,
                changes_requested,
                unresolved: 2,
            };
            for (decision, expected) in [
                (
                    ReviewDecision::Approved,
                    if approved {
                        Verdict::None
                    } else {
                        Verdict::Approved
                    },
                ),
                (
                    ReviewDecision::ChangesRequested,
                    if changes_requested {
                        Verdict::None
                    } else {
                        Verdict::ChangesRequested
                    },
                ),
                (ReviewDecision::ReviewRequired, Verdict::ReviewRequired),
                (ReviewDecision::None, Verdict::None),
                (ReviewDecision::Unknown, Verdict::Unknown),
            ] {
                assert_eq!(pills.remaining_verdict(decision), expected);
            }
        }
    }
}

#[test]
fn a_draft_or_nonopen_request_never_claims_current_merge_readiness() {
    for (mergeable, expected) in [
        (Mergeable::Mergeable, Merge::Ready),
        (Mergeable::Conflicting, Merge::Conflicts),
        (Mergeable::Blocked, Merge::Blocked),
        (Mergeable::Unstable, Merge::Unstable),
        (Mergeable::Behind, Merge::Behind),
        (Mergeable::Unknown, Merge::Unknown),
    ] {
        for state in [PullState::Merged, PullState::Closed, PullState::Unknown] {
            for draft in [false, true] {
                let mut pull = summary(state, draft);
                pull.mergeable = mergeable;
                assert_eq!(Merge::for_open(&pull), None);
            }
        }
        let mut open = summary(PullState::Open, false);
        open.mergeable = mergeable;
        assert_eq!(Merge::for_open(&open), Some(expected));

        let mut draft = summary(PullState::Open, true);
        draft.mergeable = mergeable;
        assert_eq!(Merge::for_open(&draft), None);
    }
}
