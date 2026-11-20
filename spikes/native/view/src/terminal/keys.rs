//! A key pressed in a terminal, as the bytes its program reads: Paseo's
//! `encodeTerminalKeyInput` (`packages/protocol/src/terminal-key-input.ts`),
//! the encoding its pane uses for every key xterm does not type itself.
//!
//! [Decision Log]
//! - Purpose: a shell tab takes the keyboard (TRACKER 482). A typed
//!   character reaches the program as its own UTF-8 (the text input path);
//!   Enter, the arrows, the function keys and every Ctrl or Alt chord need
//!   the escape sequences a terminal sends for them.
//! - Existing assets: the grid keeps the program's DECCKM mode
//!   (`CursorKeys`); nothing in the app encodes keys yet.
//! - Alternatives: an encoder of the app's own (an xterm table), or a
//!   crate (`termwiz`'s key encoder brings its whole terminal stack).
//! - Choice: Paseo's function, case by case, so a program sees the bytes it
//!   sees under Paseo. Its kitty keyboard and win32 input modes are left
//!   out: the grid does not track the flags a program sets for them, so a
//!   modified Enter is a plain carriage return here (a stated difference).
//! - Bounds: a sequence is at most a few bytes; nothing is stored.

use super::grid::CursorKeys;

/// A function key, F1 to F12.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Function(u8);

impl Function {
    /// The highest function key Paseo encodes.
    pub const LAST: u8 = 12;

    /// F`number`; `None` outside 1 to 12.
    #[must_use]
    pub const fn new(number: u8) -> Option<Self> {
        if matches!(number, 1..=Self::LAST) {
            Some(Self(number))
        } else {
            None
        }
    }

    /// Its number.
    #[must_use]
    pub const fn number(self) -> u8 {
        self.0
    }
}

/// A key with a name rather than a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Named {
    /// Enter (Return).
    Enter,
    /// Tab.
    Tab,
    /// Backspace.
    Backspace,
    /// Escape.
    Escape,
    /// Arrow up.
    Up,
    /// Arrow down.
    Down,
    /// Arrow left.
    Left,
    /// Arrow right.
    Right,
    /// Home.
    Home,
    /// End.
    End,
    /// Insert.
    Insert,
    /// Delete (forward).
    Delete,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// A function key.
    F(Function),
}

/// A key: a character, or a named key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A character as the layout produced it (shifted already).
    Char(char),
    /// A named key.
    Named(Named),
}

/// The modifiers held with a key, as one set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mods(u8);

impl Mods {
    /// None held.
    pub const NONE: Self = Self(0);
    /// Shift.
    pub const SHIFT: Self = Self(1);
    /// Alt (Option).
    pub const ALT: Self = Self(2);
    /// Control.
    pub const CTRL: Self = Self(4);
    /// Meta (Command, the platform key).
    pub const META: Self = Self(8);

    /// This set and `other`.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether every modifier of `other` is held.
    #[must_use]
    pub const fn has(self, other: Self) -> bool {
        (self.0 & other.0) == other.0 && other.0 != 0
    }

    /// Whether nothing is held.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// xterm's modifier parameter: 1, plus 1 for Shift, 2 for Alt, 4 for
    /// Control and 8 for Meta (Paseo's `modifierParam`). The set holds those
    /// four bits only, so the sum is at most 16.
    const fn param(self) -> u8 {
        self.0.saturating_add(1)
    }
}

/// The bytes the program reads for `key` held with `mods`, its arrows in
/// the program's `cursor` mode. Empty when the key sends nothing.
#[must_use]
pub fn encode(key: Key, mods: Mods, cursor: CursorKeys) -> String {
    match key {
        Key::Char(character) => printable(character, mods),
        Key::Named(named) => named_key(named, mods, cursor),
    }
}

fn alt_prefixed(sequence: &str, mods: Mods) -> String {
    if mods.has(Mods::ALT) {
        format!("\x1b{sequence}")
    } else {
        sequence.to_owned()
    }
}

fn printable(character: char, mods: Mods) -> String {
    let shown = if mods.has(Mods::SHIFT) {
        character.to_uppercase().next().unwrap_or(character)
    } else {
        character
    };
    if mods.has(Mods::CTRL) {
        return alt_prefixed(&control(shown), mods);
    }
    alt_prefixed(shown.encode_utf8(&mut [0; 4]), mods)
}

/// Control with a character: a letter is its control code, a few symbols
/// and digits are xterm's, any other ASCII character keeps its low five
/// bits (Paseo's `encodeCtrlChar` and `ctrlSymbolCode`).
fn control(character: char) -> String {
    let upper = character.to_ascii_uppercase();
    let code = match upper {
        'A'..='Z' => u32::from(upper).checked_sub(64),
        ' ' | '@' | '2' => Some(0x00),
        '[' | '3' => Some(0x1b),
        '\\' | '4' => Some(0x1c),
        ']' | '5' => Some(0x1d),
        '^' | '6' => Some(0x1e),
        '_' | '/' | '7' => Some(0x1f),
        '8' | '?' => Some(0x7f),
        other if other.is_ascii() => Some(u32::from(other) & 0x1f),
        _ => None,
    };
    code.and_then(char::from_u32)
        .unwrap_or(character)
        .to_string()
}

fn csi(last: char, mods: Mods) -> String {
    match mods.param() {
        1 => format!("\x1b[{last}"),
        param => format!("\x1b[1;{param}{last}"),
    }
}

fn ss3(last: char, mods: Mods) -> String {
    match mods.param() {
        1 => format!("\x1bO{last}"),
        _ => csi(last, mods),
    }
}

fn tilde(number: u8, mods: Mods) -> String {
    match mods.param() {
        1 => format!("\x1b[{number}~"),
        param => format!("\x1b[{number};{param}~"),
    }
}

fn arrow(last: char, mods: Mods, cursor: CursorKeys) -> String {
    match cursor {
        CursorKeys::Application => ss3(last, mods),
        CursorKeys::Normal => csi(last, mods),
    }
}

fn function(key: Function, mods: Mods) -> String {
    match key.number() {
        1 => ss3('P', mods),
        2 => ss3('Q', mods),
        3 => ss3('R', mods),
        4 => ss3('S', mods),
        5 => tilde(15, mods),
        6 => tilde(17, mods),
        7 => tilde(18, mods),
        8 => tilde(19, mods),
        9 => tilde(20, mods),
        10 => tilde(21, mods),
        11 => tilde(23, mods),
        _ => tilde(24, mods),
    }
}

fn named_key(named: Named, mods: Mods, cursor: CursorKeys) -> String {
    match named {
        Named::Enter => "\r".to_owned(),
        Named::Tab => {
            if mods == Mods::SHIFT {
                "\x1b[Z".to_owned()
            } else {
                alt_prefixed("\t", mods)
            }
        }
        Named::Backspace => alt_prefixed("\x7f", mods),
        Named::Escape => "\x1b".to_owned(),
        Named::Up => arrow('A', mods, cursor),
        Named::Down => arrow('B', mods, cursor),
        Named::Right => arrow('C', mods, cursor),
        Named::Left => arrow('D', mods, cursor),
        Named::Home => csi('H', mods),
        Named::End => csi('F', mods),
        Named::Insert => tilde(2, mods),
        Named::Delete => tilde(3, mods),
        Named::PageUp => tilde(5, mods),
        Named::PageDown => tilde(6, mods),
        Named::F(key) => function(key, mods),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(key: Named, mods: Mods) -> String {
        encode(Key::Named(key), mods, CursorKeys::Normal)
    }

    #[test]
    fn control_letters_and_symbols_are_xterms_codes() {
        let ctrl = |c| encode(Key::Char(c), Mods::CTRL, CursorKeys::Normal);
        assert_eq!(ctrl('c'), "\x03");
        assert_eq!(ctrl('C'), "\x03");
        assert_eq!(ctrl('a'), "\x01");
        assert_eq!(ctrl(' '), "\x00");
        assert_eq!(ctrl('['), "\x1b");
        assert_eq!(ctrl('/'), "\x1f");
        assert_eq!(ctrl('?'), "\x7f");
        assert_eq!(
            encode(
                Key::Char('b'),
                Mods::CTRL.with(Mods::ALT),
                CursorKeys::Normal
            ),
            "\x1b\x02"
        );
    }

    #[test]
    fn alt_prefixes_escape_and_shift_uppercases() {
        assert_eq!(
            encode(Key::Char('f'), Mods::ALT, CursorKeys::Normal),
            "\x1bf"
        );
        assert_eq!(
            encode(Key::Char('한'), Mods::NONE, CursorKeys::Normal),
            "한"
        );
        assert_eq!(encode(Key::Char('x'), Mods::SHIFT, CursorKeys::Normal), "X");
    }

    #[test]
    fn arrows_follow_the_programs_cursor_mode() {
        assert_eq!(named(Named::Up, Mods::NONE), "\x1b[A");
        assert_eq!(
            encode(Key::Named(Named::Up), Mods::NONE, CursorKeys::Application),
            "\x1bOA"
        );
        assert_eq!(named(Named::Left, Mods::CTRL), "\x1b[1;5D");
        assert_eq!(
            encode(
                Key::Named(Named::Right),
                Mods::SHIFT,
                CursorKeys::Application
            ),
            "\x1b[1;2C"
        );
    }

    #[test]
    fn editing_and_function_keys_are_paseos_sequences() {
        assert_eq!(named(Named::Enter, Mods::NONE), "\r");
        assert_eq!(named(Named::Enter, Mods::SHIFT), "\r");
        assert_eq!(named(Named::Tab, Mods::NONE), "\t");
        assert_eq!(named(Named::Tab, Mods::SHIFT), "\x1b[Z");
        assert_eq!(named(Named::Backspace, Mods::NONE), "\x7f");
        assert_eq!(named(Named::Backspace, Mods::ALT), "\x1b\x7f");
        assert_eq!(named(Named::Escape, Mods::NONE), "\x1b");
        assert_eq!(named(Named::Home, Mods::NONE), "\x1b[H");
        assert_eq!(named(Named::End, Mods::SHIFT), "\x1b[1;2F");
        assert_eq!(named(Named::Delete, Mods::NONE), "\x1b[3~");
        assert_eq!(named(Named::PageUp, Mods::CTRL), "\x1b[5;5~");
        let f = |n| Named::F(Function::new(n).unwrap());
        assert_eq!(named(f(1), Mods::NONE), "\x1bOP");
        assert_eq!(named(f(4), Mods::CTRL), "\x1b[1;5S");
        assert_eq!(named(f(5), Mods::NONE), "\x1b[15~");
        assert_eq!(named(f(12), Mods::SHIFT), "\x1b[24;2~");
        assert_eq!(Function::new(0), None);
        assert_eq!(Function::new(13), None);
    }

    #[test]
    fn a_set_reports_what_it_holds() {
        let held = Mods::CTRL.with(Mods::SHIFT);
        assert!(held.has(Mods::CTRL) && held.has(Mods::SHIFT));
        assert!(!held.has(Mods::ALT) && !held.has(Mods::NONE));
        assert!(Mods::NONE.is_empty() && !held.is_empty());
        assert_eq!(Mods::CTRL.with(Mods::ALT).with(Mods::SHIFT).param(), 8);
    }
}
