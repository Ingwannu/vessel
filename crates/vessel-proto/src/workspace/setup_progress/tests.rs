use super::*;
#[test]
fn the_values_carrier_assigns_no_protocol_or_dispatch_surface() {
    assert_eq!(WORKTREE_SETUP_OBSERVATION, None);
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
