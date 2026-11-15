use super::*;
fn key(raw: &str) -> DismissalKey {
    raw.to_owned().try_into().unwrap()
}
#[test]
fn durable_notices_wait_for_storage_but_ephemeral_notices_and_priority_still_work() {
    let mut state = State::default();
    let token = state
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
    assert_eq!(state.dismiss(&"setup", token).unwrap(), Some("Setup"));
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
        state.register(MAX_CALLOUTS, Priority::new(100), None, 99),
        Err(Problem::ProvidersFull)
    );
    assert_eq!(state.active().unwrap().content(), &0);
    let _token = state.register(0, Priority::default(), None, 100).unwrap();
    assert_eq!(state.active().unwrap().content(), &100);
    state.next = u64::MAX;
    assert_eq!(
        state.register(0, Priority::new(100), None, 200),
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
    let token = state
        .register(1, Priority::default(), Some(key("new-project")), "Keep me")
        .unwrap();
    assert_eq!(state.dismiss(&1, token), Err(Problem::DismissalsFull));
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

#[test]
fn sidebar_callout_preferences_round_trip_without_replacing_other_choices() {
    use crate::prefs::{PreferenceChange, Preferences, Theme};
    let initial = Preferences::default();
    let old_document = serde_json::to_value(&initial).unwrap();
    assert!(old_document.get("sidebar_callout_dismissals").is_none());
    let initial = initial
        .changed(&PreferenceChange::Theme(Theme::Light))
        .unwrap();
    let keys =
        Dismissals::try_from(vec![key("setup:one"), key("setup:one"), key("release:two")]).unwrap();
    let changed = initial
        .changed(&PreferenceChange::SidebarCalloutDismissals(keys.clone()))
        .unwrap();
    assert_eq!(changed.appearance.theme, Theme::Light);
    assert_eq!(
        changed.sidebar_callout_dismissals.keys(),
        &[key("setup:one"), key("release:two")]
    );
    let encoded = serde_json::to_string(&changed).unwrap();
    let restored: Preferences = serde_json::from_str(&encoded).unwrap();
    assert_eq!(restored.sidebar_callout_dismissals, keys);
    let mut restarted = State::default();
    restarted
        .load_dismissed(restored.sidebar_callout_dismissals.keys())
        .unwrap();
    let _token = restarted
        .register("setup", Priority::default(), Some(key("setup:one")), ())
        .unwrap();
    assert!(restarted.active().is_none());
}

#[test]
fn sidebar_callout_malformed_preference_band_does_not_erase_theme() {
    use crate::prefs::{Preferences, Theme};
    for invalid in [
        serde_json::json!(["valid", "\n"]),
        serde_json::json!(vec!["same"; MAX_DISMISSALS + 1]),
        serde_json::json!({"invalid": true}),
    ] {
        let prefs: Preferences = serde_json::from_value(serde_json::json!({
            "appearance": {"theme": "light"},
            "sidebar_callout_dismissals": invalid,
        }))
        .unwrap();
        assert!(prefs.sidebar_callout_dismissals.is_empty());
        assert_eq!(prefs.appearance.theme, Theme::Light);
    }
    assert_eq!(
        Dismissals::try_from(vec![key("same"); MAX_DISMISSALS + 1]),
        Err(Problem::DismissalsFull)
    );
}

#[test]
fn sidebar_callout_source_identity_keeps_exact_scoped_names_separate_from_dismissal_keys() {
    let a = SourceId::try_from("setup:node-a:project-a".to_owned()).unwrap();
    let b = SourceId::try_from("setup:node-a:project-b".to_owned()).unwrap();
    assert_ne!(a, b);
    assert_eq!(a.as_str(), "setup:node-a:project-a");
    assert_ne!(
        SourceId::try_from(" source ".to_owned()).unwrap(),
        SourceId::try_from("source".to_owned()).unwrap()
    );
    assert_eq!(key(" source "), key("source"));
    assert_eq!(SourceId::try_from(String::new()), Err(InvalidKey));
    assert_eq!(
        SourceId::try_from("x".repeat(MAX_DISMISSAL_KEY_BYTES + 1)),
        Err(InvalidKey)
    );
    assert_eq!(
        SourceId::try_from("bad\nsource".to_owned()),
        Err(InvalidKey)
    );
}

#[test]
fn sidebar_callout_input_requires_the_active_registration_after_replace_or_reconcile() {
    let mut state = State::default();
    state.load_dismissed(&[]).unwrap();
    let old = state.register(1, Priority::default(), None, "Old").unwrap();
    let current = state
        .register(1, Priority::default(), Some(key("new")), "New")
        .unwrap();
    assert_eq!(state.active().unwrap().token(), current);
    assert_eq!(state.dismiss(&1, old), Ok(None));
    assert!(state.dismissed().is_empty());
    state.load_dismissed(&[key("new")]).unwrap();
    assert_eq!(state.dismiss(&1, current), Ok(None));
    state.load_dismissed(&[]).unwrap();
    let higher = state.register(2, Priority::new(1), None, "Higher").unwrap();
    assert_eq!(state.dismiss(&1, current), Ok(None));
    assert_eq!(state.dismiss(&2, higher), Ok(Some("Higher")));
    assert_eq!(state.dismiss(&1, current), Ok(Some("New")));
    assert_eq!(state.dismissed(), &[key("new")]);
}
