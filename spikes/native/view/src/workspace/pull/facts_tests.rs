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
