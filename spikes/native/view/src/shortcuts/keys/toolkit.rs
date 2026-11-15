//! Toolkit spellings at the keyboard input boundary.

use super::{CODES, single};

/// Shifted signs on a US keyboard, by the position they are typed at
/// (a platform may name a shifted key by what it types: `?` for Slash).
pub(super) const SHIFTED: [(char, &str); 21] = [
    ('~', "Backquote"),
    ('!', "Digit1"),
    ('@', "Digit2"),
    ('#', "Digit3"),
    ('$', "Digit4"),
    ('%', "Digit5"),
    ('^', "Digit6"),
    ('&', "Digit7"),
    ('*', "Digit8"),
    ('(', "Digit9"),
    (')', "Digit0"),
    ('_', "Minus"),
    ('+', "Equal"),
    ('{', "BracketLeft"),
    ('}', "BracketRight"),
    ('|', "Backslash"),
    (':', "Semicolon"),
    ('"', "Quote"),
    ('<', "Comma"),
    ('>', "Period"),
    ('?', "Slash"),
];

/// The app's toolkit's names for keys that have a name (lower case), by
/// their position.
const TOOLKIT: [(&str, &str); 15] = [
    ("enter", "Enter"),
    ("escape", "Escape"),
    ("tab", "Tab"),
    ("space", "Space"),
    ("backspace", "Backspace"),
    ("delete", "Delete"),
    ("up", "ArrowUp"),
    ("down", "ArrowDown"),
    ("left", "ArrowLeft"),
    ("right", "ArrowRight"),
    ("home", "Home"),
    ("end", "End"),
    ("pageup", "PageUp"),
    ("pagedown", "PageDown"),
    ("insert", "Insert"),
];

/// The position of a key as the app's toolkit names it (`"k"`, `"K"`,
/// `"?"`, `"pageup"`, `"f5"`), in the DOM's terms (`"KeyK"`, `"Slash"`,
/// `"PageUp"`, `"F5"`); empty when it has none a binding can name. Its
/// letters and signs are those of a US keyboard: the toolkit already
/// reads another layout's key as the US key at its place.
#[must_use]
pub fn code_of(key: &str) -> String {
    if let Some(one) = single(key) {
        if one.is_ascii_alphabetic() {
            return format!("Key{}", one.to_ascii_uppercase());
        }
        if one.is_ascii_digit() {
            return format!("Digit{one}");
        }
        if let Some((code, _)) = CODES.iter().find(|(_, sign)| single(sign) == Some(one)) {
            return (*code).to_owned();
        }
        if let Some((_, code)) = SHIFTED.iter().find(|(sign, _)| *sign == one) {
            return (*code).to_owned();
        }
        return String::new();
    }
    let lower = key.to_ascii_lowercase();
    if let Some((_, code)) = TOOLKIT.iter().find(|(name, _)| *name == lower) {
        return (*code).to_owned();
    }
    let function = lower
        .strip_prefix('f')
        .and_then(|number| number.parse::<u8>().ok())
        .filter(|number| (1..=12).contains(number));
    function.map_or_else(String::new, |number| format!("F{number}"))
}
