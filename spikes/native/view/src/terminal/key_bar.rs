//! The virtual key bar under a shell at compact widths (P4): Paseo's
//! `terminal-virtual-keyboard.ts` and the sticky modifiers of its
//! `terminal-pane.tsx`.
//!
//! [Decision Log]
//! - Purpose: on a narrow window a person needs Esc, Tab, the arrows and
//!   Ctrl/Alt/Shift chords without hunting for them. Paseo draws a bar of
//!   them under its terminal pane.
//! - Paseo 0.15.4:
//!   - Three layouts (`terminal-virtual-keyboard.ts` 25-82): two rows, one
//!     wide row, or three columns.
//!   - They are picked by the bar's width (84-97): under 300 the columns;
//!     660 or more and not compact one row; else two rows.
//!   - It shows on its mobile app, and elsewhere only when compact (99-104).
//!   - A modifier is sticky: toggled, then applied to the next key, from the
//!     bar or typed, and cleared (`terminal-pane.tsx` 795-830, 1052-1083;
//!     `utils/terminal-keys.ts` 204-263).
//! - Choice: the three tables, the widths and the sticky rule as data and
//!   pure functions here. Keys are encoded by `keys.rs`, the encoding Paseo
//!   uses, so a bar key and the same key pressed send the same bytes. The
//!   app draws the bar and decides which controls a surface shows: no
//!   Paste and no keyboard toggle on the desktop app, as Paseo's desktop
//!   has neither.
//! - Cost: none beyond the bar. The tables keep Paseo's Paste and toggle
//!   slots, so the rows stay Paseo's when a surface shows them.

use BarKey::{Down, Enter, Escape, Left, Right, Tab, Up};
use Control::{Key as K, KeyboardToggle, Modifier as M, Paste};
use Modifier::{Alt, Ctrl, Shift};

use super::grid::CursorKeys;
use super::keys::{Key, Mods, Named, encode};

/// A key the bar sends (Paseo's `TERMINAL_VIRTUAL_KEY_BUTTONS`,
/// `terminal-key-dispatch.ts` 20-30).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarKey {
    /// Escape.
    Escape,
    /// Tab.
    Tab,
    /// Arrow up.
    Up,
    /// Arrow down.
    Down,
    /// Arrow left.
    Left,
    /// Arrow right.
    Right,
    /// Enter.
    Enter,
}

impl BarKey {
    /// Its label on the bar.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Escape => "Esc",
            Self::Tab => "Tab",
            Self::Up => "\u{2191}",
            Self::Down => "\u{2193}",
            Self::Left => "\u{2190}",
            Self::Right => "\u{2192}",
            Self::Enter => "Enter",
        }
    }

    /// A stable name for its element (Paseo's test id, `terminal-key-<id>`).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Escape => "esc",
            Self::Tab => "tab",
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
            Self::Enter => "enter",
        }
    }

    const fn named(self) -> Named {
        match self {
            Self::Escape => Named::Escape,
            Self::Tab => Named::Tab,
            Self::Up => Named::Up,
            Self::Down => Named::Down,
            Self::Left => Named::Left,
            Self::Right => Named::Right,
            Self::Enter => Named::Enter,
        }
    }
}

/// A sticky modifier (Paseo's labels, `terminal-pane.tsx` 119-123).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    /// Control.
    Ctrl,
    /// Alt.
    Alt,
    /// Shift.
    Shift,
}

impl Modifier {
    /// Its label on the bar.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ctrl => "Ctrl",
            Self::Alt => "Alt",
            Self::Shift => "Shift",
        }
    }

    /// A stable name for its element.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Ctrl => "ctrl",
            Self::Alt => "alt",
            Self::Shift => "shift",
        }
    }
}

/// One control of the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// A key.
    Key(BarKey),
    /// A sticky modifier.
    Modifier(Modifier),
    /// Paste from the clipboard.
    Paste,
    /// Show or hide a soft keyboard.
    KeyboardToggle,
}

/// Two rows (`TERMINAL_VIRTUAL_KEYBOARD_ROWS`, 25-42).
pub const ROWS: &[&[Control]] = &[
    &[K(Escape), K(Tab), M(Ctrl), K(Up), M(Shift), KeyboardToggle],
    &[M(Alt), Paste, K(Left), K(Down), K(Right), K(Enter)],
];

/// One wide row (`TERMINAL_VIRTUAL_KEYBOARD_WIDE_ROWS`, 44-59).
pub const WIDE_ROWS: &[&[Control]] = &[&[
    K(Escape),
    K(Tab),
    M(Ctrl),
    M(Alt),
    M(Shift),
    K(Left),
    K(Down),
    K(Up),
    K(Right),
    Paste,
    KeyboardToggle,
    K(Enter),
]];

/// Three columns (`TERMINAL_VIRTUAL_KEYBOARD_THREE_COLUMN_ROWS`, 61-82).
pub const THREE_COLUMN_ROWS: &[&[Control]] = &[
    &[K(Escape), K(Tab), M(Ctrl)],
    &[M(Alt), M(Shift), KeyboardToggle],
    &[Paste, K(Up), K(Enter)],
    &[K(Left), K(Down), K(Right)],
];

/// From this width, and not compact, the bar is one row (84).
pub const SINGLE_ROW_MIN_WIDTH: f32 = 660.0;

/// Under this width the bar is three columns (85).
pub const THREE_COLUMN_MAX_WIDTH: f32 = 300.0;

/// The widest the bar's rows grow, centred (`terminal-pane.tsx` 117).
pub const MAX_WIDTH: f32 = 880.0;

/// A button's height (Paseo's `keyButton`).
pub const KEY_HEIGHT: f32 = 34.0;

/// The bar's padding, both sides together (`keyboardContainer`: spacing 2
/// on each side). The rows are the window's width less this.
pub const PADDING: f32 = 16.0;

/// The rows for a bar spanning a window `window_width` wide: [`rows`] of
/// the window's width less [`PADDING`], with the thresholds moved instead
/// so nothing is computed at run time.
#[must_use]
pub fn rows_for_window(window_width: f32, compact: bool) -> &'static [&'static [Control]] {
    const THREE_COLUMN_WINDOW: f32 = THREE_COLUMN_MAX_WIDTH + PADDING;
    const SINGLE_ROW_WINDOW: f32 = SINGLE_ROW_MIN_WIDTH + PADDING;
    if window_width > PADDING && window_width < THREE_COLUMN_WINDOW {
        THREE_COLUMN_ROWS
    } else if !compact && window_width >= SINGLE_ROW_WINDOW {
        WIDE_ROWS
    } else {
        ROWS
    }
}

/// The rows for a bar `available_width` wide (Paseo's
/// `resolveTerminalVirtualKeyboardRows`, 87-97); a width of 0 is not yet
/// measured.
#[must_use]
pub fn rows(available_width: f32, compact: bool) -> &'static [&'static [Control]] {
    if available_width > 0.0 && available_width < THREE_COLUMN_MAX_WIDTH {
        THREE_COLUMN_ROWS
    } else if !compact && available_width >= SINGLE_ROW_MIN_WIDTH {
        WIDE_ROWS
    } else {
        ROWS
    }
}

/// The modifiers set on the bar, applied to the next key and cleared.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sticky {
    ctrl: bool,
    alt: bool,
    shift: bool,
}

impl Sticky {
    /// Set `modifier` if it is not, clear it if it is.
    pub const fn toggle(&mut self, modifier: Modifier) {
        match modifier {
            Ctrl => self.ctrl = !self.ctrl,
            Alt => self.alt = !self.alt,
            Shift => self.shift = !self.shift,
        }
    }

    /// Whether `modifier` is set.
    #[must_use]
    pub const fn is_set(self, modifier: Modifier) -> bool {
        match modifier {
            Ctrl => self.ctrl,
            Alt => self.alt,
            Shift => self.shift,
        }
    }

    /// Whether none is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        !(self.ctrl || self.alt || self.shift)
    }

    /// The set modifiers as a key's, clearing them.
    pub const fn take(&mut self) -> Mods {
        let mut mods = Mods::NONE;
        if self.shift {
            mods = mods.with(Mods::SHIFT);
        }
        if self.alt {
            mods = mods.with(Mods::ALT);
        }
        if self.ctrl {
            mods = mods.with(Mods::CTRL);
        }
        *self = Self {
            ctrl: false,
            alt: false,
            shift: false,
        };
        mods
    }
}

/// A bar key with the set modifiers, as its program reads it; the set
/// modifiers clear.
pub fn press(key: BarKey, sticky: &mut Sticky, cursor: CursorKeys) -> String {
    encode(Key::Named(key.named()), sticky.take(), cursor)
}

/// A key pressed on the keyboard while modifiers are set: the key with
/// them as well as those `held`; they clear. A character outside printable
/// ASCII goes as typed, as in [`typed`] (Paseo's `mapTerminalDataToKey`
/// maps printable ASCII only; P4n app review note 1). `None` when none is
/// set.
pub fn pressed(key: Key, held: Mods, sticky: &mut Sticky, cursor: CursorKeys) -> Option<String> {
    if sticky.is_empty() {
        return None;
    }
    let mods = held.with(sticky.take());
    if let Key::Char(only) = key
        && !matches!(only, ' '..='~')
    {
        return Some(only.to_string());
    }
    Some(encode(key, mods, cursor))
}

/// Text typed while modifiers are set (Paseo's
/// `resolvePendingModifierDataInput` with `mapTerminalDataToKey`): one
/// character that names a key (Enter, Tab, Backspace, Escape) or is
/// printable ASCII goes out as that key with them; anything else goes as
/// typed. Either way they clear. `None` when none is set: the text goes as
/// typed.
pub fn typed(text: &str, sticky: &mut Sticky, cursor: CursorKeys) -> Option<String> {
    if sticky.is_empty() {
        return None;
    }
    let mods = sticky.take();
    let mut chars = text.chars();
    let (Some(only), None) = (chars.next(), chars.next()) else {
        return Some(text.to_owned());
    };
    let key = match only {
        '\r' | '\n' => Key::Named(Named::Enter),
        '\t' => Key::Named(Named::Tab),
        '\u{7f}' | '\u{8}' => Key::Named(Named::Backspace),
        '\u{1b}' => Key::Named(Named::Escape),
        ' '..='~' => Key::Char(only),
        _ => return Some(text.to_owned()),
    };
    Some(encode(key, mods, cursor))
}

#[cfg(test)]
#[path = "key_bar_tests.rs"]
mod tests;
