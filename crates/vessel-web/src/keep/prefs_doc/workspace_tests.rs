use super::{Decoded, PrefsDoc};
use crate::wire::{OpenLocationDestination, OpenLocationSource, PreferenceChange};

#[test]
fn a_workspace_edit_preserves_other_choices_and_unread_fields() {
    let original = br#"{"terminal":{"scrollback":1000001},"open_location":{"diffs":"side","pull_requests":"future"}}"#;
    let Decoded::Current(doc) = PrefsDoc::decode(original).unwrap() else {
        panic!("current preferences");
    };
    assert_eq!(u32::from(doc.values().terminal.scrollback), 10_000);
    let after = doc
        .changed(&PreferenceChange::OpenLocation {
            source: OpenLocationSource::ExplorerFiles,
            destination: OpenLocationDestination::Side,
        })
        .unwrap();
    let written: serde_json::Value = serde_json::from_slice(&after.encode().unwrap()).unwrap();
    assert_eq!(written["open_location"]["explorer_files"], "side");
    assert_eq!(written["open_location"]["diffs"], "side");
    assert_eq!(written["open_location"]["pull_requests"], "future");
    assert_eq!(written["terminal"]["scrollback"], 1_000_001);
}

#[test]
fn the_terminal_change_accepts_only_the_buffer_range() {
    for (lines, valid) in [
        ("-1", false),
        ("0", true),
        ("1000000", true),
        ("1000001", false),
    ] {
        let change = format!(r#"{{"set":"terminal_scrollback","value":{lines}}}"#);
        assert_eq!(
            serde_json::from_str::<PreferenceChange>(&change).is_ok(),
            valid
        );
    }
}

#[test]
fn the_editor_choice_defaults_off_and_keeps_other_fields() {
    let original = br#"{"editor":{"future":true},"terminal":{"scrollback":50}}"#;
    let Decoded::Current(doc) = PrefsDoc::decode(original).unwrap() else {
        panic!("current preferences");
    };
    assert!(!doc.values().editor.vim_keybindings);
    let changed = doc
        .changed(&PreferenceChange::VimKeybindings(true))
        .unwrap();
    assert!(changed.values().editor.vim_keybindings);
    let written: serde_json::Value = serde_json::from_slice(&changed.encode().unwrap()).unwrap();
    assert_eq!(written["editor"]["vim_keybindings"], true);
    assert_eq!(written["editor"]["future"], true);
    assert_eq!(written["terminal"]["scrollback"], 50);
}
