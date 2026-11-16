//! The actual production key table, evaluated from already typed event/mode data.
use super::key::{Function, Kind};
use super::{InputError, Key, Modifier, Modifiers};
use crate::{InputModes, ModeState};
use std::fmt::Write as _;

/// Every supported key sequence fits this fixed allocation, including expanded casing.
const KEY_BYTES: usize = 32;

pub(super) fn key(key: Key, mods: Modifiers, modes: InputModes) -> Result<String, InputError> {
    let mut text = String::new();
    text.try_reserve_exact(KEY_BYTES)
        .map_err(|_| InputError::Unavailable)?;
    match key.kind() {
        Kind::Character(ch) => character(&mut text, ch, mods)?,
        Kind::Enter => enter(&mut text, mods, modes)?,
        Kind::Tab => {
            if mods.shift == Modifier::Pressed && mods.parameter() == 2 {
                text.push_str("\x1b[Z");
            } else {
                alt(&mut text, mods);
                text.push('\t');
            }
        }
        Kind::Backspace => {
            alt(&mut text, mods);
            text.push('\x7f');
        }
        Kind::Escape => text.push('\x1b'),
        Kind::Up => arrow(&mut text, 'A', mods, modes)?,
        Kind::Down => arrow(&mut text, 'B', mods, modes)?,
        Kind::Right => arrow(&mut text, 'C', mods, modes)?,
        Kind::Left => arrow(&mut text, 'D', mods, modes)?,
        Kind::Home => csi(&mut text, 'H', mods)?,
        Kind::End => csi(&mut text, 'F', mods)?,
        Kind::Insert => tilde(&mut text, 2, mods)?,
        Kind::Delete => tilde(&mut text, 3, mods)?,
        Kind::PageUp => tilde(&mut text, 5, mods)?,
        Kind::PageDown => tilde(&mut text, 6, mods)?,
        Kind::Function(function) => function_key(&mut text, function, mods)?,
    }
    Ok(text)
}

fn alt(text: &mut String, mods: Modifiers) {
    if mods.alt == Modifier::Pressed {
        text.push('\x1b');
    }
}

fn character(text: &mut String, ch: char, mods: Modifiers) -> Result<(), InputError> {
    let value = if mods.shift == Modifier::Pressed {
        ch.to_uppercase().collect::<String>()
    } else {
        ch.to_string()
    };
    alt(text, mods);
    if mods.control == Modifier::Released {
        text.push_str(&value);
        return Ok(());
    }
    let upper = value.to_uppercase();
    let mut uppercase = upper.chars();
    if let Some(ch) = uppercase.next()
        && uppercase.next().is_none()
        && ch.is_ascii_uppercase()
    {
        text.push(char::from_u32(u32::from(ch) - u32::from('@')).ok_or(InputError::Unavailable)?);
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

fn enter(text: &mut String, mods: Modifiers, modes: InputModes) -> Result<(), InputError> {
    let parameter = mods.parameter();
    let win32_modifiers = mods.shift.bit() + mods.control.bit() + mods.alt.bit();
    if parameter > 1 && modes.win32 == ModeState::Enabled && win32_modifiers > 0 {
        let state = mods.shift.bit() * 16 + mods.control.bit() * 8 + mods.alt.bit() * 2;
        write!(text, "\x1b[13;28;13;1;{state};1_").map_err(|_| InputError::Unavailable)?;
    } else if parameter > 1 && modes.kitty.get() > 0 {
        write!(text, "\x1b[13;{parameter}u").map_err(|_| InputError::Unavailable)?;
    } else {
        text.push('\r');
    }
    Ok(())
}

fn csi(text: &mut String, end: char, mods: Modifiers) -> Result<(), InputError> {
    let parameter = mods.parameter();
    if parameter == 1 {
        write!(text, "\x1b[{end}")
    } else {
        write!(text, "\x1b[1;{parameter}{end}")
    }
    .map_err(|_| InputError::Unavailable)
}

fn ss3(text: &mut String, end: char, mods: Modifiers) -> Result<(), InputError> {
    if mods.parameter() == 1 {
        write!(text, "\x1bO{end}").map_err(|_| InputError::Unavailable)
    } else {
        csi(text, end, mods)
    }
}

fn arrow(
    text: &mut String,
    end: char,
    mods: Modifiers,
    modes: InputModes,
) -> Result<(), InputError> {
    match modes.application_cursor {
        ModeState::Enabled => ss3(text, end, mods),
        ModeState::Disabled => csi(text, end, mods),
    }
}

fn tilde(text: &mut String, code: u8, mods: Modifiers) -> Result<(), InputError> {
    let parameter = mods.parameter();
    if parameter == 1 {
        write!(text, "\x1b[{code}~")
    } else {
        write!(text, "\x1b[{code};{parameter}~")
    }
    .map_err(|_| InputError::Unavailable)
}

fn function_key(text: &mut String, key: Function, mods: Modifiers) -> Result<(), InputError> {
    match key {
        Function::One => ss3(text, 'P', mods),
        Function::Two => ss3(text, 'Q', mods),
        Function::Three => ss3(text, 'R', mods),
        Function::Four => ss3(text, 'S', mods),
        Function::Five => tilde(text, 15, mods),
        Function::Six => tilde(text, 17, mods),
        Function::Seven => tilde(text, 18, mods),
        Function::Eight => tilde(text, 19, mods),
        Function::Nine => tilde(text, 20, mods),
        Function::Ten => tilde(text, 21, mods),
        Function::Eleven => tilde(text, 23, mods),
        Function::Twelve => tilde(text, 24, mods),
    }
}
