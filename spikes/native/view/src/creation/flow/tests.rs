//! A completion is not permission to consume another draft or take another route.

use super::*;

const FIRST: Ticket = Ticket::new(0);
const SECOND: Ticket = Ticket::new(1);

#[test]
fn duplicate_submit_and_regressive_progress_are_ignored() {
    let mut flow = Flow::default();
    assert_eq!(flow.begin(FIRST, Stage::Workspace), Transition::Applied);
    assert_eq!(flow.begin(SECOND, Stage::Resource), Transition::Ignored);
    assert_eq!(
        flow.progress(SECOND, Stage::InitialInput),
        Transition::Ignored
    );
    assert_eq!(
        flow.progress(FIRST, Stage::InitialInput),
        Transition::Applied
    );
    assert_eq!(flow.progress(FIRST, Stage::Resource), Transition::Ignored);
}

#[test]
fn leaving_is_sticky_for_every_in_flight_stage() {
    for stage in [Stage::Workspace, Stage::Resource, Stage::InitialInput] {
        let mut flow = Flow::default();
        assert_eq!(flow.begin(FIRST, stage), Transition::Applied);
        flow.leave();
        assert_eq!(flow.complete(FIRST), Completion::Background);
        assert_eq!(flow.complete(FIRST), Completion::Ignored);
        assert!(flow.may_submit());
    }
}

#[test]
fn explicit_refusals_unlock_retry_but_unknown_outcome_does_not() {
    for failure in [
        Failure::QueueFull,
        Failure::Offline,
        Failure::Unsupported,
        Failure::Refused(ErrorCode::Forbidden),
    ] {
        let mut flow = Flow::default();
        assert_eq!(flow.begin(FIRST, Stage::Resource), Transition::Applied);
        assert_eq!(flow.fail(SECOND, failure), Transition::Ignored);
        assert_eq!(flow.fail(FIRST, failure), Transition::Applied);
        assert!(flow.may_submit());
        assert_eq!(flow.complete(FIRST), Completion::Ignored);
        assert_eq!(flow.begin(SECOND, Stage::Resource), Transition::Applied);
    }
    let mut flow = Flow::default();
    assert_eq!(flow.begin(FIRST, Stage::Resource), Transition::Applied);
    assert_eq!(
        flow.fail(FIRST, Failure::OutcomeUnknown),
        Transition::Applied
    );
    assert!(!flow.may_submit());
    assert_eq!(flow.begin(SECOND, Stage::Resource), Transition::Ignored);
    flow.leave();
    assert_eq!(flow.complete(FIRST), Completion::Background);
}

#[test]
fn a_late_failure_cannot_change_another_submission_or_a_completed_one() {
    let mut flow = Flow::default();
    assert_eq!(flow.begin(FIRST, Stage::Resource), Transition::Applied);
    assert_eq!(flow.complete(FIRST), Completion::Open);
    assert_eq!(flow.fail(FIRST, Failure::Offline), Transition::Ignored);
    assert_eq!(flow.begin(SECOND, Stage::Workspace), Transition::Applied);
    assert_eq!(flow.complete(FIRST), Completion::Ignored);
    assert_eq!(flow.fail(FIRST, Failure::Offline), Transition::Ignored);
    assert_eq!(flow.complete(SECOND), Completion::Open);
}
