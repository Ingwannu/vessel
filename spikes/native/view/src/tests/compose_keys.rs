//! The page's `data/compose-keys.test.ts`: which press sends a prompt.

use crate::prefs::{EnterPress, SendKey, send_key_label, sends, sends_other};

fn enter() -> EnterPress {
    EnterPress {
        enter: true,
        ..EnterPress::default()
    }
}

#[test]
fn enter_mode_enter_and_mod_enter_send_shift_or_alt_starts_a_line() {
    let mode = SendKey::Enter;
    assert!(sends(enter(), mode));
    assert!(sends(
        EnterPress {
            meta: true,
            ..enter()
        },
        mode
    ));
    assert!(!sends(
        EnterPress {
            shift: true,
            ..enter()
        },
        mode
    ));
    assert!(!sends(
        EnterPress {
            alt: true,
            ..enter()
        },
        mode
    ));
}

#[test]
fn mod_enter_mode_only_mod_enter_sends() {
    let mode = SendKey::ModEnter;
    assert!(!sends(enter(), mode));
    assert!(sends(
        EnterPress {
            ctrl: true,
            ..enter()
        },
        mode
    ));
    assert!(!sends(
        EnterPress {
            ctrl: true,
            shift: true,
            ..enter()
        },
        mode
    ));
}

#[test]
fn an_input_methods_enter_never_sends_another_key_never_does() {
    assert!(!sends(
        EnterPress {
            composing: true,
            ..enter()
        },
        SendKey::Enter
    ));
    assert!(!sends(EnterPress::default(), SendKey::Enter));
}

#[test]
fn labels() {
    assert_eq!(send_key_label(SendKey::Enter, "⌘"), "Enter");
    assert_eq!(send_key_label(SendKey::ModEnter, "Ctrl"), "Ctrl+Enter");
}

#[test]
fn the_other_way_mod_enter_or_mod_shift_enter_where_mod_enter_sends() {
    let press = EnterPress {
        ctrl: true,
        ..enter()
    };
    assert!(sends_other(press, SendKey::Enter));
    assert!(!sends_other(press, SendKey::ModEnter));
    assert!(sends_other(
        EnterPress {
            shift: true,
            ..press
        },
        SendKey::ModEnter
    ));
    assert!(!sends_other(
        EnterPress {
            ctrl: false,
            ..press
        },
        SendKey::Enter
    ));
    assert!(!sends_other(
        EnterPress {
            composing: true,
            ..press
        },
        SendKey::Enter
    ));
}
