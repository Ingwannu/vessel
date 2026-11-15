//! Byte encoding is pure; ordering and admission belong to the existing sender.
use super::{FunctionKey, Key, KeyModes, ModifiedEnter, Modifier, Modifiers};
use crate::terminal::grid::CursorKeys;

/// Encode one normalized key exactly as Paseo's terminal-key-input codec.
/// Platform IME committed text must use the literal-text path instead.
#[must_use]
pub fn encode_key(key: Key, modifiers: Modifiers, modes: KeyModes) -> String {
    match key {
        Key::Character(ch) => character(ch, modifiers),
        Key::Enter => enter(modifiers, modes.enter),
        Key::Tab if modifiers == Modifiers::default().with(Modifier::Shift) => "\x1b[Z".into(),
        Key::Tab => alt("\t".into(), modifiers),
        Key::Backspace => alt("\x7f".into(), modifiers),
        Key::Escape => "\x1b".into(),
        Key::Up => arrow('A', modifiers, modes.cursor),
        Key::Down => arrow('B', modifiers, modes.cursor),
        Key::Right => arrow('C', modifiers, modes.cursor),
        Key::Left => arrow('D', modifiers, modes.cursor),
        Key::Home => csi('H', modifiers),
        Key::End => csi('F', modifiers),
        Key::Insert => tilde(2, modifiers),
        Key::Delete => tilde(3, modifiers),
        Key::PageUp => tilde(5, modifiers),
        Key::PageDown => tilde(6, modifiers),
        Key::Function(key) => function(key, modifiers),
    }
}

fn character(ch: char, modifiers: Modifiers) -> String {
    // Paseo admits a single UTF-16 code unit here. Emoji/IME text uses its
    // separate committed-text path, never a masked surrogate/control byte.
    if ch.len_utf16() != 1 {
        return String::new();
    }
    let text = if modifiers.contains(Modifier::Shift) {
        ch.to_uppercase().collect()
    } else {
        ch.to_string()
    };
    let text = if modifiers.contains(Modifier::Control) {
        control(&text)
    } else {
        text
    };
    alt(text, modifiers)
}

fn control(text: &str) -> String {
    let upper = text.to_uppercase();
    let mut chars = upper.chars();
    if let Some(ch) = chars.next()
        && chars.next().is_none()
        && ch.is_ascii_uppercase()
    {
        // ASCII A..Z maps to control 1..26; the masked value is always u8.
        return char::from(u8::try_from(u32::from(ch) & 0x1f).unwrap_or_default()).into();
    }
    let symbol = match text {
        " " | "@" | "2" => Some('\0'),
        "[" | "3" => Some('\x1b'),
        "\\" | "4" => Some('\x1c'),
        "]" | "5" => Some('\x1d'),
        "^" | "6" => Some('\x1e'),
        "_" | "/" | "7" => Some('\x1f'),
        "8" | "?" => Some('\x7f'),
        _ => None,
    };
    if let Some(symbol) = symbol {
        return symbol.into();
    }
    let mut chars = text.chars();
    if let Some(ch) = chars.next()
        && ch.len_utf16() == 1
        && chars.next().is_none()
    {
        return char::from(u8::try_from(u32::from(ch) & 0x1f).unwrap_or_default()).into();
    }
    text.to_owned()
}

fn alt(text: String, modifiers: Modifiers) -> String {
    if modifiers.contains(Modifier::Alt) {
        format!("\x1b{text}")
    } else {
        text
    }
}

fn csi(final_byte: char, modifiers: Modifiers) -> String {
    let parameter = modifiers.parameter();
    if parameter == 1 {
        format!("\x1b[{final_byte}")
    } else {
        format!("\x1b[1;{parameter}{final_byte}")
    }
}

fn arrow(final_byte: char, modifiers: Modifiers, mode: CursorKeys) -> String {
    match mode {
        CursorKeys::Application if modifiers == Modifiers::default() => {
            format!("\x1bO{final_byte}")
        }
        CursorKeys::Normal | CursorKeys::Application => csi(final_byte, modifiers),
    }
}

fn tilde(number: u8, modifiers: Modifiers) -> String {
    let parameter = modifiers.parameter();
    if parameter == 1 {
        format!("\x1b[{number}~")
    } else {
        format!("\x1b[{number};{parameter}~")
    }
}

fn function(key: FunctionKey, modifiers: Modifiers) -> String {
    match key {
        FunctionKey::F1 => arrow('P', modifiers, CursorKeys::Application),
        FunctionKey::F2 => arrow('Q', modifiers, CursorKeys::Application),
        FunctionKey::F3 => arrow('R', modifiers, CursorKeys::Application),
        FunctionKey::F4 => arrow('S', modifiers, CursorKeys::Application),
        FunctionKey::F5 => tilde(15, modifiers),
        FunctionKey::F6 => tilde(17, modifiers),
        FunctionKey::F7 => tilde(18, modifiers),
        FunctionKey::F8 => tilde(19, modifiers),
        FunctionKey::F9 => tilde(20, modifiers),
        FunctionKey::F10 => tilde(21, modifiers),
        FunctionKey::F11 => tilde(23, modifiers),
        FunctionKey::F12 => tilde(24, modifiers),
    }
}

fn enter(modifiers: Modifiers, mode: ModifiedEnter) -> String {
    if modifiers == Modifiers::default() {
        return "\r".into();
    }
    let control = modifiers.contains(Modifier::Control);
    let shift = modifiers.contains(Modifier::Shift);
    let alt = modifiers.contains(Modifier::Alt);
    if matches!(mode, ModifiedEnter::Win32 | ModifiedEnter::Both) && (control || shift || alt) {
        let state = if alt { 0x02 } else { 0 }
            | if control { 0x08 } else { 0 }
            | if shift { 0x10 } else { 0 };
        return format!("\x1b[13;28;13;1;{state};1_");
    }
    match mode {
        ModifiedEnter::Kitty | ModifiedEnter::Both => format!("\x1b[13;{}u", modifiers.parameter()),
        ModifiedEnter::Legacy | ModifiedEnter::Win32 => "\r".into(),
    }
}
