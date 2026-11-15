use super::*;
fn key(raw: &str) -> DismissalKey {
    raw.to_owned().try_into().unwrap()
}
#[test]
fn durable_notices_wait_for_storage_but_ephemeral_notices_and_priority_still_work() {
    let mut state = State::default();
    let _token = state
        .register(
            "setup",
            Priority::new(100),
            Some(key(" setup:one ")),
            "Setup",
        )
        .unwrap();
    let _token = state
        .register("update", Priority::default(), None, "Update")
        .unwrap();
    assert_eq!(state.active().unwrap().id(), &"update");
    state.load_dismissed(&[]).unwrap();
    assert_eq!(state.active().unwrap().content(), &"Setup");
    assert_eq!(state.dismiss(&"setup").unwrap(), Some("Setup"));
    assert_eq!(state.active().unwrap().content(), &"Update");
    assert_eq!(state.dismissed(), &[key("setup:one")]);
    let _token = state
        .register(
            "setup",
            Priority::new(100),
            Some(key("setup:one")),
            "Setup again",
        )
        .unwrap();
    assert_eq!(state.active().unwrap().id(), &"update");
    let _token = state
        .register(
            "setup",
            Priority::new(100),
            Some(key("setup:two")),
            "Other project",
        )
        .unwrap();
    assert_eq!(state.active().unwrap().content(), &"Other project");
}
#[test]
fn stale_cleanup_never_removes_replacement_and_equal_priority_retains_original_order() {
    let mut state = State::default();
    let old = state
        .register(1, Priority::default(), None, "First")
        .unwrap();
    let _token = state
        .register(2, Priority::default(), None, "Second")
        .unwrap();
    let current = state
        .register(1, Priority::default(), None, "First refreshed")
        .unwrap();
    state.unregister(&1, old);
    assert_eq!(state.active().unwrap().content(), &"First refreshed");
    state.unregister(&1, current);
    assert_eq!(state.active().unwrap().content(), &"Second");
    state.clear();
    assert!(state.active().is_none());
    let next = state
        .register(1, Priority::default(), None, "New lifetime")
        .unwrap();
    assert_ne!(next, current);
    state.unregister(&1, current);
    assert_eq!(state.active().unwrap().content(), &"New lifetime");
}
#[test]
fn capacity_and_counter_refusals_preserve_the_preceding_registration() {
    let mut state = State::default();
    for id in 0..MAX_CALLOUTS {
        let _token = state.register(id, Priority::default(), None, id).unwrap();
    }
    assert_eq!(
        let _token = state.register(MAX_CALLOUTS, Priority::new(100), None, 99),
        Err(Problem::ProvidersFull)
    );
    assert_eq!(state.active().unwrap().content(), &0);
    let _token = state.register(0, Priority::default(), None, 100).unwrap();
    assert_eq!(state.active().unwrap().content(), &100);
    state.next = u64::MAX;
    assert_eq!(
        let _token = state.register(0, Priority::new(100), None, 200),
        Err(Problem::TokensExhausted)
    );
    assert_eq!(state.active().unwrap().content(), &100);
}
#[test]
fn malformed_and_oversized_dismissals_are_refused_without_dropping_existing_keys() {
    assert_eq!(DismissalKey::try_from(" \t ".to_owned()), Err(InvalidKey));
    assert_eq!(
        DismissalKey::try_from("one\ntwo".to_owned()),
        Err(InvalidKey)
    );
    assert_eq!(
        DismissalKey::try_from("x".repeat(MAX_DISMISSAL_KEY_BYTES + 1)),
        Err(InvalidKey)
    );
    assert_eq!(key("  project:one  "), key("project:one"));
    let keys = (0..MAX_DISMISSALS)
        .map(|n| key(&format!("project:{n}")))
        .collect::<Vec<_>>();
    let mut state = State::default();
    state.load_dismissed(&keys).unwrap();
    let _token = state
        .register(1, Priority::default(), Some(key("new-project")), "Keep me")
        .unwrap();
    assert_eq!(state.dismiss(&1), Err(Problem::DismissalsFull));
    assert_eq!(state.active().unwrap().content(), &"Keep me");
    assert_eq!(state.dismissed(), keys);
    let too_many = vec![key("duplicate"); MAX_DISMISSALS + 1];
    assert_eq!(
        state.load_dismissed(&too_many),
        Err(Problem::DismissalsFull)
    );
    assert_eq!(state.dismissed(), keys);
    // Existing durable keys survive a provider reset and a JSON restart.
    let disk = serde_json::to_string(state.dismissed()).unwrap();
    let restored: Vec<DismissalKey> = serde_json::from_str(&disk).unwrap();
    state.clear();
    assert_eq!(state.dismissed(), keys);
    let mut restarted = State::default();
    restarted.load_dismissed(&restored).unwrap();
    let _token = restarted
        .register(1, Priority::default(), Some(keys[0].clone()), "Do not show")
        .unwrap();
    assert!(restarted.active().is_none());
}
