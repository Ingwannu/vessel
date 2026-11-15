use super::*;
#[test]
fn the_values_carrier_assigns_no_protocol_or_dispatch_surface() {
    assert_eq!(WORKTREE_SETUP_OBSERVATION, None);
}

#[test]
fn command_ordinals_match_first_last_and_approved_list_order_on_roundtrip() {
    let approved: Vec<_> = (0..MAX_PROJECT_COMMANDS)
        .map(|position| format!("command {position}"))
        .collect();
    for (position, command) in approved.iter().enumerate() {
        let ordinal = u32::try_from(position.checked_add(1).unwrap()).unwrap();
        let index = SetupCommandIndex::try_from(ordinal).unwrap();
        let decoded: SetupCommandIndex =
            serde_json::from_str(&serde_json::to_string(&index).unwrap()).unwrap();
        assert_eq!(decoded.get(), ordinal);
        let approved_position = usize::try_from(decoded.get())
            .unwrap()
            .checked_sub(1)
            .unwrap();
        assert_eq!(approved.get(approved_position), Some(command));
    }
    let first = NonZeroU32::MIN.get();
    assert_eq!(SetupCommandIndex::try_from(first).unwrap().get(), first);
    let last = u32::try_from(MAX_PROJECT_COMMANDS).unwrap();
    assert_eq!(SetupCommandIndex::try_from(last).unwrap().get(), last);
    assert!(serde_json::from_str::<SetupCommandIndex>("0").is_err());
    assert!(
        serde_json::from_str::<SetupCommandIndex>(&last.checked_add(1).unwrap().to_string())
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
