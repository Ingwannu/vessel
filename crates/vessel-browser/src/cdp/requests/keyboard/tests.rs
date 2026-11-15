//! Protocol/input-state fixtures, never native/global key injection.
use super::*;
use serde_json::json;
fn input(key: &str) -> RequiredInput {
    RequiredInput::try_from(key.to_owned()).unwrap()
}
#[test]
fn raw_key_repeat_release_and_modifier_units_stay_target_owned() {
    let mut state = KeyboardState::default();
    let mut modifiers = KeyModifiers::default();
    modifiers.include(vessel_proto::browser::KeyModifier::Control);
    let down = state
        .event(&input("a"), KeyAction::Down, modifiers)
        .unwrap();
    assert!(!format!("{down:?}").contains("KeyA"));
    assert_eq!(
        serde_json::to_value(down).unwrap(),
        json!({"type":"rawKeyDown","key":"a","code":"KeyA","windowsVirtualKeyCode":65,"modifiers":2,"autoRepeat":false})
    );
    let repeated = state
        .event(&input("A"), KeyAction::Down, modifiers)
        .unwrap();
    assert_eq!(serde_json::to_value(repeated).unwrap()["autoRepeat"], true);
    let releases = state.release_all();
    assert_eq!(releases.len(), 1);
    let release = serde_json::to_value(releases.into_iter().next().unwrap()).unwrap();
    assert_eq!(release["type"], "keyUp");
    assert_eq!(release["modifiers"], 0);
    assert!(state.release_all().is_empty());
    assert!(matches!(
        state.event(&input("a"), KeyAction::Up, modifiers),
        Err(KeyboardError::NotPressed)
    ));
}
#[test]
fn unsupported_layout_ime_chords_or_raw_protocol_input_never_become_text() {
    let mut state = KeyboardState::default();
    for refused in [
        "Control+a",
        "한",
        "ShiftRight",
        "F0",
        "F13",
        "F01",
        "\\n",
        "Input.dispatchKeyEvent",
    ] {
        assert!(matches!(
            state.event(&input(refused), KeyAction::Down, KeyModifiers::default()),
            Err(KeyboardError::Unsupported)
        ));
    }
    assert!(state.release_all().is_empty());
    for accepted in [
        "Enter",
        "Backspace",
        "ArrowLeft",
        "Tab",
        "Control",
        "F12",
        "0",
        "z",
        " ",
    ] {
        let _down = state
            .event(&input(accepted), KeyAction::Down, KeyModifiers::default())
            .unwrap();
        let _up = state
            .event(&input(accepted), KeyAction::Up, KeyModifiers::default())
            .unwrap();
    }
    assert!(state.release_all().is_empty());
}
