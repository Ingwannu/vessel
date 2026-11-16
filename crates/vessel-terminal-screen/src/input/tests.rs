use super::*;
use crate::{KittyFlags, ModeState};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Deserialize;

#[derive(Deserialize)]
struct Oracle {
    cases: Vec<KeyCase>,
    pastes: Vec<PasteCase>,
}
#[derive(Deserialize)]
struct KeyCase {
    key: String,
    mask: u8,
    kitty: u32,
    win32: bool,
    app: bool,
    expected: String,
}
#[derive(Deserialize)]
struct PasteCase {
    text: String,
    bracketed: bool,
    expected: String,
}
fn switch(enabled: bool) -> ModeState {
    if enabled { ModeState::Enabled } else { ModeState::Disabled }
}
fn modifier(mask: u8, bit: u8) -> Modifier {
    if mask & bit == 0 { Modifier::Released } else { Modifier::Pressed }
}
fn modifiers(mask: u8) -> Modifiers {
    Modifiers {
        shift: modifier(mask, 1),
        alt: modifier(mask, 2),
        control: modifier(mask, 4),
        meta: modifier(mask, 8),
    }
}
#[test]
fn all_reference_keys_modifiers_and_modes_match_original_production_encoder() {
    let oracle: Oracle = serde_json::from_str(include_str!("oracle.json")).expect("original oracle");
    assert_eq!(oracle.cases.len(), 6016);
    for case in oracle.cases {
        let modes = InputModes {
            kitty: KittyFlags::from(case.kitty),
            win32: switch(case.win32),
            application_cursor: switch(case.app),
            bracketed_paste: ModeState::Disabled,
        };
        let key = Key::try_from(case.key.as_str()).expect("oracle key");
        let actual = encode_key(key, modifiers(case.mask), modes).expect("bounded key");
        let expected = STANDARD.decode(case.expected).expect("oracle base64");
        assert_eq!(actual.as_bytes(), expected, "key {} mask {} modes {modes:?}", case.key, case.mask);
    }
}
#[test]
fn original_bracketed_paste_and_terminator_defusing_match() {
    let oracle: Oracle = serde_json::from_str(include_str!("oracle.json")).expect("original oracle");
    assert_eq!(oracle.pastes.len(), 10);
    for case in oracle.pastes {
        let modes = InputModes { bracketed_paste: switch(case.bracketed), ..InputModes::default() };
        let expected = STANDARD.decode(case.expected).expect("oracle base64");
        let actual = encode_paste(&case.text, modes, InputBudget::new(expected.len())).expect("exact budget");
        assert_eq!(actual.as_bytes(), expected);
        if !expected.is_empty() {
            assert_eq!(encode_paste(&case.text, modes, InputBudget::new(expected.len() - 1)), Err(InputError::Budget));
        }
    }
}
#[test]
fn text_commits_preserve_unicode_and_refusal_keeps_original_intent() {
    let text = "한글😀e\u{301}\r\n";
    let enough = InputBudget::new(text.len());
    assert_eq!(encode_text(text, enough).expect("text").as_bytes(), text.as_bytes());
    assert_eq!(encode_text(text, InputBudget::new(text.len() - 1)), Err(InputError::Budget));
    assert_eq!(text, "한글😀e\u{301}\r\n");
    assert!(encode_text("", InputBudget::new(0)).expect("empty").as_bytes().is_empty());
    for key in ["", "Unknown", "F13", "😀", "e\u{301}"] {
        assert_eq!(Key::try_from(key), Err(InputError::UnsupportedKey));
    }
}
