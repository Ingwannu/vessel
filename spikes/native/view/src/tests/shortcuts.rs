//! `web/src/data/shortcuts/{keys,match,edit,listing}.test.ts`: the keys'
//! grammar, matching a press, changing a key and Settings' rows.

use std::collections::BTreeMap;

use crate::Lang;
use crate::prefs::{
    ActionId, Bindings, KeyBinding, PreferenceChange, Preferences, ShortcutEdit, ShortcutEdits,
    ShortcutTarget,
};
use crate::shortcuts::{
    Action, Binding, Chord, Clash, KeyPress, Matched, Platform, Refusal, SEQUENCE_MS,
    ShortcutFilter, ShortcutGroup, assign, binding_words, chord_caps, chord_of, clash_of, code_of,
    matched, modified_count, overlaps, refusal, remove_key, restore, shortcut_row, shortcut_rows,
};

mod capture;
mod theme;

type Overrides = BTreeMap<ActionId, Bindings>;

const MAC: Platform = Platform::Mac;
const OTHER: Platform = Platform::Other;

fn press(key: &str, code: &str) -> KeyPress {
    KeyPress {
        key: key.to_owned(),
        code: code.to_owned(),
        ..KeyPress::default()
    }
}

fn binding(text: &str) -> Binding {
    Binding::parse(text).unwrap()
}

fn chord(text: &str) -> Chord {
    binding(text).first
}

fn bindings(texts: &[&str]) -> Bindings {
    let keys: Vec<KeyBinding> = texts.iter().map(|text| text.parse().unwrap()).collect();
    Bindings::try_from(keys).unwrap()
}

fn overrides(pairs: &[(&str, &[&str])]) -> Overrides {
    pairs
        .iter()
        .map(|(id, keys)| (id.parse().unwrap(), bindings(keys)))
        .collect()
}

fn keys(texts: &[&str]) -> ShortcutTarget {
    ShortcutTarget::Bindings {
        keys: bindings(texts),
    }
}

fn edit(action: Action, to: ShortcutTarget) -> ShortcutEdit {
    ShortcutEdit {
        action: action.action_id().unwrap(),
        to,
    }
}

fn kept(edits: Vec<ShortcutEdit>, preferences: &Preferences) -> Preferences {
    preferences
        .changed(&PreferenceChange::Shortcuts {
            edits: ShortcutEdits::try_from(edits).unwrap(),
        })
        .unwrap()
}

#[test]
fn bindings_that_parse_and_come_back_as_written() {
    let accepted = [
        "Mod+K",
        "Alt+[",
        "G S",
        "/",
        "Shift+/",
        "Mod+Shift+B",
        "Ctrl+Alt+Shift+Meta+F12",
        "Mod+=",
        "Mod+\\",
    ];
    for text in accepted {
        assert_eq!(binding(text).text(), text);
    }
}

#[test]
fn text_that_is_not_a_binding() {
    let long = "x".repeat(40);
    let refused = [
        "",
        "k",
        "?",
        "Plus",
        "Shift+Mod+K",
        "Mod+",
        "Mod+Mod+K",
        "A B C",
        "Mod+K ",
        "Hyper+K",
        long.as_str(),
    ];
    for text in refused {
        assert_eq!(Binding::parse(text), None, "{text}");
    }
}

#[test]
fn command_is_mod_on_a_mac_control_is_mod_elsewhere() {
    let command = KeyPress {
        meta: true,
        ..press("k", "KeyK")
    };
    let control = KeyPress {
        ctrl: true,
        ..press("k", "KeyK")
    };
    let on_mac = chord_of(&command, MAC).unwrap();
    assert!(on_mac.primary);
    assert_eq!(on_mac.key, "K");
    let control_on_mac = chord_of(&control, MAC).unwrap();
    assert!(!control_on_mac.primary);
    assert!(control_on_mac.ctrl);
    let control_elsewhere = chord_of(&control, OTHER).unwrap();
    assert!(control_elsewhere.primary);
    assert!(!control_elsewhere.ctrl);
}

#[test]
fn the_key_is_its_position_whatever_it_types() {
    let option = |key: &str, code: &str| KeyPress {
        alt: true,
        ..press(key, code)
    };
    let key = |press: &KeyPress, platform| chord_of(press, platform).map(|chord| chord.key);
    assert_eq!(key(&option("“", "BracketLeft"), MAC).as_deref(), Some("["));
    assert_eq!(key(&option("¡", "Digit1"), MAC).as_deref(), Some("1"));
    assert_eq!(key(&press("Process", "KeyN"), OTHER).as_deref(), Some("N"));
    assert_eq!(key(&press("ㅜ", "KeyN"), OTHER).as_deref(), Some("N"));
}

#[test]
fn shift_is_always_written_a_modifier_alone_is_nothing() {
    let question = KeyPress {
        shift: true,
        ..press("?", "Slash")
    };
    let slash = chord_of(&question, OTHER).unwrap();
    assert_eq!(slash.key, "/");
    assert!(slash.shift);
    let shift = KeyPress {
        shift: true,
        ..press("Shift", "ShiftLeft")
    };
    assert_eq!(chord_of(&shift, OTHER), None);
    let up = chord_of(&press("ArrowUp", "ArrowUp"), OTHER).unwrap();
    assert_eq!(up.key, "Up");
    let no_code = chord_of(&press("n", ""), OTHER).unwrap();
    assert_eq!(no_code.key, "N");
}

#[test]
fn glyphs_on_a_mac_names_elsewhere() {
    let then = Lang::En.shortcuts().then;
    let shifted = chord("Mod+Shift+B");
    assert_eq!(chord_caps(&shifted, MAC), ["⇧", "⌘", "B"]);
    assert_eq!(chord_caps(&shifted, OTHER), ["Ctrl", "Shift", "B"]);
    assert_eq!(
        binding_words(&binding("Mod+Shift+B"), OTHER, then),
        "Ctrl+Shift+B"
    );
    assert_eq!(binding_words(&binding("G S"), MAC, then), "G then S");
}

#[test]
fn platforms() {
    assert_eq!(Platform::of("MacIntel", ""), MAC);
    assert_eq!(
        Platform::of("Linux x86_64", "Mozilla/5.0 (X11; Linux)"),
        OTHER
    );
}

#[test]
fn a_sequence_overlaps_the_key_it_starts_with() {
    assert!(overlaps(&binding("G"), &binding("G S"), OTHER));
    assert!(!overlaps(&binding("G S"), &binding("G M"), OTHER));
}

#[test]
fn mod_is_resolved_before_comparing() {
    assert!(overlaps(&binding("Mod+K"), &binding("Ctrl+K"), OTHER));
    assert!(!overlaps(&binding("Mod+K"), &binding("Ctrl+K"), MAC));
    assert!(overlaps(&binding("Mod+K"), &binding("Meta+K"), MAC));
}

fn run(action: Action) -> Matched {
    Matched::Run {
        action,
        index: None,
    }
}

#[test]
fn a_chord_runs_its_action() {
    let none = Overrides::new();
    assert_eq!(
        matched(&chord("Mod+K"), None, &none, 0, OTHER),
        run(Action::Palette)
    );
    assert_eq!(
        matched(&chord("/"), None, &none, 0, OTHER),
        run(Action::Palette)
    );
}

#[test]
fn a_sequence_waits_for_its_second_key_within_the_time() {
    let none = Overrides::new();
    let Matched::Wait(pending) = matched(&chord("G"), None, &none, 100, OTHER) else {
        panic!("G waits");
    };
    assert_eq!(
        matched(&chord("S"), Some(&pending), &none, 100 + SEQUENCE_MS, OTHER),
        run(Action::GoBoard)
    );
    assert_eq!(
        matched(&chord("S"), Some(&pending), &none, 101 + SEQUENCE_MS, OTHER),
        Matched::None
    );
}

#[test]
fn alt_and_a_digit_go_to_that_session() {
    let none = Overrides::new();
    assert_eq!(
        matched(&chord("Alt+3"), None, &none, 0, OTHER),
        Matched::Run {
            action: Action::JumpSession,
            index: Some(3),
        }
    );
    assert_eq!(
        matched(&chord("Alt+0"), None, &none, 0, OTHER),
        Matched::None
    );
}

#[test]
fn an_override_replaces_the_defaults_an_empty_one_unsets() {
    let find = overrides(&[("find", &["Mod+J"])]);
    assert_eq!(
        matched(&chord("Mod+J"), None, &find, 0, OTHER),
        run(Action::Find)
    );
    assert_eq!(
        matched(&chord("Mod+F"), None, &find, 0, OTHER),
        Matched::None
    );
    let unset = overrides(&[("toggle-sidebar", &[])]);
    assert_eq!(
        matched(&chord("Mod+B"), None, &unset, 0, OTHER),
        Matched::None
    );
}

#[test]
fn a_modifier_is_needed_and_the_systems_own_keys_are_refused() {
    let words = Lang::En.shortcuts();
    let said = |text: &str, platform| refusal(&chord(text), platform).map(|r| words.refusal(r));
    assert_eq!(said("K", OTHER), Some("Use Ctrl or Alt with another key."));
    assert_eq!(said("Mod+T", MAC), Some(words.refusal(Refusal::Kept)));
    assert_eq!(said("Mod+2", OTHER), Some(words.refusal(Refusal::Kept)));
    assert_eq!(
        said("Mod+C", OTHER),
        Some("That shortcut is reserved for standard editing commands.")
    );
    assert_eq!(said("Mod+Shift+K", OTHER), None);
}

#[test]
fn clashes_with_another_action_with_a_fixed_one_with_itself() {
    let none = Overrides::new();
    let clash = |text: &str| clash_of(Action::Find, &binding(text), &none, OTHER);
    assert_eq!(clash("Mod+K"), Some(Clash::Taken(Action::Palette)));
    assert_eq!(clash("Alt+4"), Some(Clash::Fixed(Action::JumpSession)));
    assert_eq!(clash("Mod+F"), Some(Clash::Same(Action::Find)));
    assert_eq!(clash("G"), Some(Clash::Taken(Action::GoBoard)));
    assert_eq!(clash("Mod+J"), None);
}

#[test]
fn the_clash_sentences() {
    assert_eq!(
        Lang::En
            .shortcuts()
            .clash(Clash::Taken(Action::Palette), "Ctrl+K", Action::Find),
        "Ctrl+K is assigned to Open the command palette. Reassign it to Find in the session?"
    );
}

#[test]
fn a_reassignment_moves_the_key_in_one_change() {
    let none = Overrides::new();
    let edits = assign(
        Action::Find,
        &binding("Mod+K"),
        &none,
        Some(0),
        Some(Action::Palette),
        OTHER,
    );
    assert_eq!(
        edits,
        [
            edit(Action::Find, keys(&["Mod+K"])),
            edit(Action::Palette, keys(&["/"])),
        ]
    );
}

#[test]
fn another_key_at_most_two_removing_one_the_defaults_again() {
    let none = Overrides::new();
    assert_eq!(
        assign(Action::Find, &binding("Mod+J"), &none, None, None, OTHER),
        [edit(Action::Find, keys(&["Mod+F", "Mod+J"]))]
    );
    let palette = assign(Action::Palette, &binding("Mod+J"), &none, None, None, OTHER);
    assert_eq!(palette[0].to, keys(&["/", "Mod+J"]));
    assert_eq!(
        remove_key(Action::Palette, 0, &none),
        [edit(Action::Palette, keys(&["/"]))]
    );
    assert_eq!(
        restore(Action::Find),
        [edit(Action::Find, ShortcutTarget::Default)]
    );
}

#[test]
fn edits_kept_change_the_preferences_shortcuts() {
    let start = Preferences::default();
    let moved = kept(
        assign(
            Action::Find,
            &binding("Mod+K"),
            &start.shortcuts,
            Some(0),
            Some(Action::Palette),
            OTHER,
        ),
        &start,
    );
    assert_eq!(
        moved.shortcuts,
        overrides(&[("find", &["Mod+K"]), ("palette", &["/"])])
    );
    let restored = kept(restore(Action::Find), &moved);
    assert_eq!(restored.shortcuts, overrides(&[("palette", &["/"])]));
    assert_eq!(modified_count(&restored.shortcuts), 1);
}

#[test]
fn every_registry_id_and_default_is_one_the_preferences_keep() {
    for action in Action::ALL {
        assert!(action.action_id().is_some(), "{}", action.id());
        for text in action.defaults() {
            assert!(text.parse::<KeyBinding>().is_ok(), "{text}");
        }
    }
}

#[test]
fn a_row_says_its_keys_as_the_platform_writes_them() {
    let none = Overrides::new();
    let words = Lang::En.shortcuts();
    assert_eq!(
        shortcut_row(Action::ToggleInspector, &none, MAC, words).words,
        ["⇧⌘B"]
    );
    assert_eq!(
        shortcut_row(Action::ToggleInspector, &none, OTHER, words).words,
        ["Ctrl+Shift+B"]
    );
    let jump = shortcut_row(Action::JumpSession, &none, OTHER, words);
    assert!(jump.fixed);
    assert_eq!(jump.words, ["Alt+1–9"]);
}

#[test]
fn search_by_name_or_keys_modified_shows_the_changed_only() {
    let none = Overrides::new();
    let words = Lang::En.shortcuts();
    let actions = |group, overrides: &Overrides, query, filter| {
        shortcut_rows(group, overrides, OTHER, query, filter, words)
            .into_iter()
            .map(|row| row.action)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        actions(
            ShortcutGroup::General,
            &none,
            "palette",
            ShortcutFilter::All
        ),
        [Action::Palette]
    );
    assert_eq!(
        actions(ShortcutGroup::Session, &none, "ctrl+f", ShortcutFilter::All),
        [Action::Find]
    );
    let changed = overrides(&[("find", &["Mod+J"])]);
    assert_eq!(
        actions(
            ShortcutGroup::Session,
            &changed,
            "",
            ShortcutFilter::Modified
        ),
        [Action::Find]
    );
    assert_eq!(modified_count(&changed), 1);
}

#[test]
fn a_clash_kept_in_the_file_is_shown_on_both_rows() {
    let kept = overrides(&[("find", &["Mod+K"])]);
    let words = Lang::En.shortcuts();
    assert_eq!(
        shortcut_row(Action::Find, &kept, OTHER, words).clashes,
        [Action::Palette]
    );
    assert_eq!(
        shortcut_row(Action::Palette, &kept, OTHER, words).clashes,
        [Action::Find]
    );
}

#[test]
fn every_action_and_group_has_its_words_in_both_languages() {
    for lang in [Lang::En, Lang::Ko] {
        let words = lang.shortcuts();
        for action in Action::ALL {
            assert!(!words.label(action).is_empty(), "{action:?}");
            assert_eq!(Action::of_id(action.id()), Some(action));
        }
        for group in ShortcutGroup::ALL {
            assert!(!words.group(group).is_empty(), "{group:?}");
        }
    }
    assert_eq!(
        Lang::Ko.shortcuts().label(Action::ToggleInspector),
        "요약 보이기·숨기기"
    );
}

// The app's own: a key as its toolkit names it, at its position, so the
// one listener matches it as the page's matches a browser's press.

#[test]
fn reads_the_toolkits_key_names_as_positions() {
    let cases = [
        ("k", "KeyK"),
        ("K", "KeyK"),
        ("7", "Digit7"),
        ("/", "Slash"),
        ("?", "Slash"),
        ("[", "BracketLeft"),
        ("{", "BracketLeft"),
        ("`", "Backquote"),
        ("~", "Backquote"),
        ("!", "Digit1"),
        ("up", "ArrowUp"),
        ("enter", "Enter"),
        ("escape", "Escape"),
        ("space", "Space"),
        ("pagedown", "PageDown"),
        ("f5", "F5"),
        ("f12", "F12"),
        ("f13", ""),
        ("insert", ""),
        ("ㅏ", ""),
    ];
    for (key, code) in cases {
        assert_eq!(code_of(key), code, "{key}");
    }
}

#[test]
fn a_shifted_sign_matches_as_its_position_with_shift() {
    let shifted = KeyPress {
        key: "?".to_owned(),
        code: code_of("?"),
        shift: true,
        ..KeyPress::default()
    };
    assert_eq!(chord_of(&shifted, OTHER), Some(chord("Shift+/")));
    let palette = KeyPress {
        key: "k".to_owned(),
        code: code_of("k"),
        ctrl: true,
        ..KeyPress::default()
    };
    assert_eq!(chord_of(&palette, OTHER), Some(chord("Mod+K")));
}
