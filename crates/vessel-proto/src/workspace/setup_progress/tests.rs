use super::*;
use crate::{Call, Method, MethodShape};

#[test]
fn pending_setup_calls_have_one_table_row_and_no_assigned_protocol() {
    let worktree = WorktreeId::random();
    let call = Call::SetupProgress(ReadSetupProgress {
        worktree,
        after: None,
        wait_ms: None,
    });
    assert_eq!(call.since(), WORKTREE_SETUP_OBSERVATION);
    assert_eq!(Method::SetupProgress.since(), None);
    assert_eq!(Method::CancelSetup.since(), None);
    assert_eq!(Method::SetupProgress.shape(), MethodShape::Unary);
    assert_eq!(Method::CancelSetup.shape(), MethodShape::Unary);
}

#[test]
fn command_indices_reject_values_outside_the_parser_bound_on_decode() {
    assert_eq!(
        serde_json::from_str::<SetupCommandIndex>("0")
            .unwrap()
            .get(),
        0
    );
    assert!(
        serde_json::from_str::<SetupCommandIndex>(&MAX_SETUP_PROGRESS_COMMANDS.to_string())
            .is_err()
    );
    assert!(serde_json::from_str::<SetupCommandIndex>("-1").is_err());
}

#[test]
fn generations_are_distinct_and_unknown_states_remain_unknown() {
    assert_ne!(SetupRunId::random(), SetupRunId::random());
    assert_eq!(
        serde_json::from_str::<SetupRunState>("\"future\"").unwrap(),
        SetupRunState::Unknown
    );
    let empty = SetupProgressReply { progress: None };
    assert_eq!(
        serde_json::from_str::<SetupProgressReply>(&serde_json::to_string(&empty).unwrap())
            .unwrap(),
        empty
    );
}

#[test]
fn observation_waits_keep_their_explicit_millisecond_unit_on_the_wire() {
    let request = ReadSetupProgress {
        worktree: WorktreeId::random(),
        after: None,
        wait_ms: Some(DurationMs::from_duration(std::time::Duration::from_millis(
            1234,
        ))),
    };
    let wire = serde_json::to_string(&request).unwrap();
    assert!(wire.contains("\"wait_ms\":1234"));
    assert_eq!(
        serde_json::from_str::<ReadSetupProgress>(&wire).unwrap(),
        request
    );
}
