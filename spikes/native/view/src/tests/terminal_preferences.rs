//! The native preference document reuses the one terminal count and default.
use crate::prefs::{PreferenceChange, Preferences, Theme, TurnDelivery};
use crate::terminal::ScrollbackLines;

#[test]
fn terminal_preferences_and_changes_round_trip_zero_and_the_existing_maximum() {
    for count in [0, ScrollbackLines::MAX] {
        let limit = ScrollbackLines::new(count).unwrap();
        let change = PreferenceChange::TerminalScrollback(limit);
        let encoded = serde_json::to_string(&change).unwrap();
        assert_eq!(
            encoded,
            format!(r#"{{"set":"terminal_scrollback","value":{count}}}"#)
        );
        assert_eq!(
            serde_json::from_str::<PreferenceChange>(&encoded).unwrap(),
            change
        );
        let preferences = Preferences::default().changed(&change).unwrap();
        assert_eq!(preferences.terminal_scrollback, limit);
        assert_eq!(
            serde_json::from_str::<Preferences>(&serde_json::to_string(&preferences).unwrap())
                .unwrap(),
            preferences
        );
    }
    let over = ScrollbackLines::MAX.checked_add(1).unwrap();
    assert!(
        serde_json::from_str::<PreferenceChange>(&format!(
            r#"{{"set":"terminal_scrollback","value":{over}}}"#
        ))
        .is_err()
    );
}

#[test]
fn unreadable_stored_limits_default_without_resetting_the_other_fields() {
    let over = ScrollbackLines::MAX.checked_add(1).unwrap().to_string();
    for raw in ["null", "true", "\"bad\"", "-1", "1.25", over.as_str()] {
        let preferences: Preferences = serde_json::from_str(&format!(
            r#"{{"terminal_scrollback":{raw},"composer":{{"delivery":"queue"}}}}"#
        ))
        .unwrap();
        assert_eq!(preferences.terminal_scrollback, ScrollbackLines::DEFAULT);
        assert_eq!(preferences.composer.delivery, TurnDelivery::Queue);
    }
    let missing: Preferences =
        serde_json::from_str(r#"{"composer":{"delivery":"queue"}}"#).unwrap();
    assert_eq!(missing.terminal_scrollback, ScrollbackLines::DEFAULT);
    assert_eq!(missing.composer.delivery, TurnDelivery::Queue);
}

#[test]
fn a_terminal_change_retains_other_preferences_and_does_not_mutate_its_input() {
    let before = Preferences::default()
        .changed(&PreferenceChange::Theme(Theme::Dark))
        .unwrap()
        .changed(&PreferenceChange::Delivery(TurnDelivery::Queue))
        .unwrap();
    let next = before
        .changed(&PreferenceChange::TerminalScrollback(
            ScrollbackLines::new(0).unwrap(),
        ))
        .unwrap();
    assert_eq!(before.terminal_scrollback, ScrollbackLines::DEFAULT);
    assert_eq!(next.terminal_scrollback.get(), 0);
    let mut restored = next;
    restored.terminal_scrollback = before.terminal_scrollback;
    assert_eq!(restored, before);
}
