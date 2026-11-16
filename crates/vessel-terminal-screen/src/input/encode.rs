//! The actual production key table, evaluated from already typed event/mode data.
use super::key::{Function, Kind};
use super::{InputError, Key, Modifier, Modifiers};
use crate::{InputModes, ModeState};
use std::fmt::Write as _;

/// Every supported key sequence fits this fixed allocation, including expanded casing.
const KEY_BYTES: usize = 32;

pub(super) fn key(key: Key, modifiers: Modifiers, modes: InputModes) -> Result<String, InputError> {
    let mut text = String::new();
    text.try_reserve_exact(KEY_BYTES)
        .map_err(|_encoding| InputError::Unavailable)?;
    match key.kind() {
        Kind::Character(ch) => character(&mut text, ch, modifiers)?,
        Kind::Enter => enter(&mut text, modifiers, modes)?,
        Kind::Tab => {
            if modifiers.shift == Modifier::Pressed && modifiers.parameter() == 2 {
                text.push_str("\x1b[Z");
            } else {
                alt(&mut text, modifiers);
                text.push('\t');
            }
        }
        Kind::Backspace => {
            alt(&mut text, modifiers);
            text.push('\x7f');
        }
        Kind::Escape => text.push('\x1b'),
        Kind::Up => arrow(&mut text, 'A', modifiers, modes)?,
        Kind::Down => arrow(&mut text, 'B', modifiers, modes)?,
        Kind::Right => arrow(&mut text, 'C', modifiers, modes)?,
        Kind::Left => arrow(&mut text, 'D', modifiers, modes)?,
        Kind::Home => csi(&mut text, 'H', modifiers)?,
        Kind::End => csi(&mut text, 'F', modifiers)?,
        Kind::Insert => tilde(&mut text, 2, modifiers)?,
        Kind::Delete => tilde(&mut text, 3, modifiers)?,
        Kind::PageUp => tilde(&mut text, 5, modifiers)?,
        Kind::PageDown => tilde(&mut text, 6, modifiers)?,
        Kind::Function(function) => function_key(&mut text, function, modifiers)?,
    }
    Ok(text)
}

fn alt(text: &mut String, modifiers: Modifiers) {
    if modifiers.alt == Modifier::Pressed {
        text.push('\x1b');
    }
}

fn character(text: &mut String, ch: char, modifiers: Modifiers) -> Result<(), InputError> {
    let value = if modifiers.shift == Modifier::Pressed {
        ch.to_uppercase().collect::<String>()
    } else {
        ch.to_string()
    };
    alt(text, modifiers);
    if modifiers.control == Modifier::Released {
        text.push_str(&value);
        return Ok(());
    }
    let upper = value.to_uppercase();
    let mut uppercase = upper.chars();
    if let Some(ch) = uppercase.next()
        && uppercase.next().is_none()
        && ch.is_ascii_uppercase()
    {
        text.push(
            u32::from(ch)
                .checked_sub(u32::from('@'))
                .and_then(char::from_u32)
                .ok_or(InputError::Unavailable)?,
        );
        return Ok(());
    }
    if value.encode_utf16().count() != 1 {
        // Casing can expand one physical key (for example sharp-s). Preserve
        // the reference's expanded string instead of inventing multiple keys.
        text.push_str(&value);
        return Ok(());
    }
    let code = value.chars().next().ok_or(InputError::Unavailable)?;
    let control = match code {
        ' ' | '@' | '2' => '\x00',
        '[' | '3' => '\x1b',
        '\\' | '4' => '\x1c',
        ']' | '5' => '\x1d',
        '^' | '6' => '\x1e',
        '_' | '/' | '7' => '\x1f',
        '8' | '?' => '\x7f',
        _ => char::from_u32(u32::from(code) & 0x1f).ok_or(InputError::Unavailable)?,
    };
    text.push(control);
    Ok(())
}

fn enter(text: &mut String, modifiers: Modifiers, modes: InputModes) -> Result<(), InputError> {
    let parameter = modifiers.parameter();
    let win32_modifiers =
        modifiers.shift.mask(1) | modifiers.control.mask(1) | modifiers.alt.mask(1);
    if parameter > 1 && modes.win32 == ModeState::Enabled && win32_modifiers > 0 {
        let state = modifiers.shift.mask(16) | modifiers.control.mask(8) | modifiers.alt.mask(2);
        write!(text, "\x1b[13;28;13;1;{state};1_").map_err(|_encoding| InputError::Unavailable)?;
    } else if parameter > 1 && modes.kitty.get() > 0 {
        write!(text, "\x1b[13;{parameter}u").map_err(|_encoding| InputError::Unavailable)?;
    } else {
        text.push('\r');
    }
    Ok(())
}

fn csi(text: &mut String, end: char, modifiers: Modifiers) -> Result<(), InputError> {
    let parameter = modifiers.parameter();
    if parameter == 1 {
        write!(text, "\x1b[{end}")
    } else {
        write!(text, "\x1b[1;{parameter}{end}")
    }
    .map_err(|_encoding| InputError::Unavailable)
}

fn ss3(text: &mut String, end: char, modifiers: Modifiers) -> Result<(), InputError> {
    if modifiers.parameter() == 1 {
        write!(text, "\x1bO{end}").map_err(|_encoding| InputError::Unavailable)
    } else {
        csi(text, end, modifiers)
    }
}

fn arrow(
    text: &mut String,
    end: char,
    modifiers: Modifiers,
    modes: InputModes,
) -> Result<(), InputError> {
    match modes.application_cursor {
        ModeState::Enabled => ss3(text, end, modifiers),
        ModeState::Disabled => csi(text, end, modifiers),
    }
}

fn tilde(text: &mut String, code: u8, modifiers: Modifiers) -> Result<(), InputError> {
    let parameter = modifiers.parameter();
    if parameter == 1 {
        write!(text, "\x1b[{code}~")
    } else {
        write!(text, "\x1b[{code};{parameter}~")
    }
    .map_err(|_encoding| InputError::Unavailable)
}

fn function_key(text: &mut String, key: Function, modifiers: Modifiers) -> Result<(), InputError> {
    match key {
        Function::One => ss3(text, 'P', modifiers),
        Function::Two => ss3(text, 'Q', modifiers),
        Function::Three => ss3(text, 'R', modifiers),
        Function::Four => ss3(text, 'S', modifiers),
        Function::Five => tilde(text, 15, modifiers),
        Function::Six => tilde(text, 17, modifiers),
        Function::Seven => tilde(text, 18, modifiers),
        Function::Eight => tilde(text, 19, modifiers),
        Function::Nine => tilde(text, 20, modifiers),
        Function::Ten => tilde(text, 21, modifiers),
        Function::Eleven => tilde(text, 23, modifiers),
        Function::Twelve => tilde(text, 24, modifiers),
    }
}
