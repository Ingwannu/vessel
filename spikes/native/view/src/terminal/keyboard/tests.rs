//! Frozen expected bytes come from executing Paseo's actual encoder, not this implementation.
use super::*;

#[derive(serde::Deserialize)]
struct Reference {
    keys: Vec<String>,
    cases: Vec<(usize, u8, u8, usize, String)>,
}

fn key(name: &str) -> Key {
    match name {
        "Enter" => Key::Enter,
        "Tab" => Key::Tab,
        "Backspace" => Key::Backspace,
        "Escape" => Key::Escape,
        "ArrowUp" => Key::Up,
        "ArrowDown" => Key::Down,
        "ArrowRight" => Key::Right,
        "ArrowLeft" => Key::Left,
        "Home" => Key::Home,
        "End" => Key::End,
        "Insert" => Key::Insert,
        "Delete" => Key::Delete,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        name if name.starts_with('F') => {
            let number: usize = name.strip_prefix('F').unwrap().parse().unwrap();
            Key::Function(
                *[
                    FunctionKey::F1,
                    FunctionKey::F2,
                    FunctionKey::F3,
                    FunctionKey::F4,
                    FunctionKey::F5,
                    FunctionKey::F6,
                    FunctionKey::F7,
                    FunctionKey::F8,
                    FunctionKey::F9,
                    FunctionKey::F10,
                    FunctionKey::F11,
                    FunctionKey::F12,
                ]
                .get(number.checked_sub(1).unwrap())
                .unwrap(),
            )
        }
        name => {
            let mut chars = name.chars();
            let first = chars.next().unwrap();
            assert!(chars.next().is_none());
            Key::Character(first)
        }
    }
}

#[test]
fn all_named_keys_modifiers_protocols_and_cursor_modes_match_paseo_bytes() {
    let reference: Reference =
        serde_json::from_str(include_str!("fixtures/paseo-key-input.json")).unwrap();
    assert_eq!(reference.cases.len(), 4352);
    for (index, mask, application, mode, expected) in reference.cases {
        assert!(mask < 16);
        let name = &reference.keys[index];
        let modes = KeyModes {
            cursor: match application {
                0 => CursorKeys::Normal,
                1 => CursorKeys::Application,
                _ => panic!("invalid fixture"),
            },
            enter: *[
                ModifiedEnter::Legacy,
                ModifiedEnter::Kitty,
                ModifiedEnter::Win32,
                ModifiedEnter::Both,
            ]
            .get(mode)
            .unwrap(),
        };
        assert_eq!(
            encode_key(key(name), Modifiers(mask), modes),
            expected,
            "{name}, modifiers={mask}, modes={modes:?}"
        );
    }
}

#[test]
fn modifier_admission_is_idempotent_and_only_sets_its_own_flag() {
    let mut modifiers = Modifiers::default();
    for (modifier, bit) in [
        (Modifier::Shift, 1),
        (Modifier::Alt, 2),
        (Modifier::Control, 4),
        (Modifier::Meta, 8),
    ] {
        let once = modifiers.with(modifier);
        assert_eq!(once, once.with(modifier));
        assert_eq!(once.0, modifiers.0 | bit);
        modifiers = once;
    }
    assert_eq!(modifiers.parameter(), 16);
}
