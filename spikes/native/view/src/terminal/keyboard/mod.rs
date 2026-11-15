//! Native key intents use Paseo's protocol encoder; committed IME text bypasses it.
//! This module consumes modes, never interprets terminal output or owns focus.
mod encode;
#[cfg(test)]
mod tests;

use super::grid::CursorKeys;

/// A physical modifier, admitted through a closed set rather than a raw mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    /// Shift.
    Shift,
    /// Alt/Option.
    Alt,
    /// Control.
    Control,
    /// Meta/Command.
    Meta,
}

/// Any combination of the four protocol modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers(u8);

impl Modifiers {
    /// Add one held modifier. Repeated admission is idempotent.
    #[must_use]
    pub const fn with(self, modifier: Modifier) -> Self {
        Self(self.0 | Self::bit(modifier))
    }

    const fn bit(modifier: Modifier) -> u8 {
        match modifier {
            Modifier::Shift => 1,
            Modifier::Alt => 2,
            Modifier::Control => 4,
            Modifier::Meta => 8,
        }
    }

    const fn contains(self, modifier: Modifier) -> bool {
        self.0 & Self::bit(modifier) != 0
    }

    const fn parameter(self) -> u8 {
        // The private mask contains only the four admitted bits.
        self.0.saturating_add(1)
    }
}

/// Function keys supported by Paseo's key protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionKey {
    /// F1.
    F1,
    /// F2.
    F2,
    /// F3.
    F3,
    /// F4.
    F4,
    /// F5.
    F5,
    /// F6.
    F6,
    /// F7.
    F7,
    /// F8.
    F8,
    /// F9.
    F9,
    /// F10.
    F10,
    /// F11.
    F11,
    /// F12.
    F12,
}

/// A key name resolved at the platform boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// One key scalar; committed multi-scalar IME text is sent literally instead.
    Character(char),
    /// Return/Enter.
    Enter,
    /// Tab.
    Tab,
    /// Backspace.
    Backspace,
    /// Escape.
    Escape,
    /// Cursor up.
    Up,
    /// Cursor down.
    Down,
    /// Cursor right.
    Right,
    /// Cursor left.
    Left,
    /// Home.
    Home,
    /// End.
    End,
    /// Insert.
    Insert,
    /// Delete.
    Delete,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// F1 through F12.
    Function(FunctionKey),
}

/// Negotiated modified-Enter protocols; Win32 has priority for its modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModifiedEnter {
    /// Legacy Enter.
    #[default]
    Legacy,
    /// Kitty keyboard flags are nonzero.
    Kitty,
    /// Win32 input mode is active.
    Win32,
    /// Both are active; Meta-only still uses Kitty.
    Both,
}

/// Engine-owned modes needed to encode a key; no transport or focus state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyModes {
    /// DECCKM application cursor mode.
    pub cursor: CursorKeys,
    /// Negotiated modified Enter.
    pub enter: ModifiedEnter,
}

pub use encode::encode_key;
