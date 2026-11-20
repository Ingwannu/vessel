//! Keys as the bytes a terminal program reads.
//!
//! The names are the ones a screen can mean. The bytes they become were
//! not read from Paseo, so these sequences are this model's, pinned by
//! the tests, not a claim that Paseo sends the same bytes.

use super::grid::TerminalModel;

/// A key the person pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A scalar, as typed.
    Char(char),
    /// Confirm.
    Enter,
    /// The next tab stop.
    Tab,
    /// Erase the previous cell.
    Backspace,
    /// Cursor up.
    Up,
    /// Cursor down.
    Down,
    /// Cursor left.
    Left,
    /// Cursor right.
    Right,
}

/// Modifiers held with a key.
///
/// Only the plain press is encoded. Paseo's modified-key bytes were not
/// read, so a modifier is not a second encoding invented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mods {
    /// No modifier.
    Plain,
}

/// Which cursor-key sequences the program asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorKeys {
    /// `ESC [ A` and the other three.
    #[default]
    Normal,
    /// `ESC O A` and the other three (DEC private mode 1).
    Application,
}

impl TerminalModel {
    /// The bytes `key` becomes in the cursor-key mode the program set.
    ///
    /// Nothing is sent. The link writes the bytes as terminal input.
    #[must_use]
    pub fn key(&self, key: Key, mods: Mods) -> Vec<u8> {
        encode(key, mods, self.cursor_keys())
    }
}

/// `key` as PTY bytes. `mods` is matched so a new modifier cannot be ignored
/// by a wildcard.
#[must_use]
fn encode(key: Key, mods: Mods, cursor: CursorKeys) -> Vec<u8> {
    match mods {
        Mods::Plain => encode_plain(key, cursor),
    }
}

fn encode_plain(key: Key, cursor: CursorKeys) -> Vec<u8> {
    match key {
        Key::Char(ch) => {
            let mut buf = [0u8; 4];
            let encoded = ch.encode_utf8(&mut buf);
            encoded.as_bytes().to_vec()
        }
        Key::Enter => vec![b'\r'],
        Key::Tab => vec![b'\t'],
        Key::Backspace => vec![0x7f],
        Key::Up => arrow(b'A', cursor),
        Key::Down => arrow(b'B', cursor),
        Key::Right => arrow(b'C', cursor),
        Key::Left => arrow(b'D', cursor),
    }
}

fn arrow(final_byte: u8, cursor: CursorKeys) -> Vec<u8> {
    match cursor {
        CursorKeys::Normal => vec![0x1b, b'[', final_byte],
        CursorKeys::Application => vec![0x1b, b'O', final_byte],
    }
}
