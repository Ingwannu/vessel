//! Which press sends a prompt (the page's `compose-keys.ts`), so the
//! composer and Settings agree on what [`SendKey`] means.

use super::SendKey;

/// A key press in the composer, as much of it as decides sending.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "a key event's own flags, as GPUI and the page's KeyboardEvent give them"
)]
pub struct EnterPress {
    /// Whether the key is Enter (another key never sends).
    pub enter: bool,
    /// Shift held.
    pub shift: bool,
    /// Alt (Option) held.
    pub alt: bool,
    /// Command held.
    pub meta: bool,
    /// Control held.
    pub ctrl: bool,
    /// An input method is composing (Korean, Japanese): its Enter commits
    /// the text and never sends.
    pub composing: bool,
}

impl EnterPress {
    /// Command or Control held.
    const fn with_mod(self) -> bool {
        self.meta || self.ctrl
    }
}

/// Whether `press` sends in `mode`: in [`SendKey::Enter`], Enter and
/// Mod+Enter (Shift or Alt starts a line); in [`SendKey::ModEnter`], only
/// Mod+Enter.
#[must_use]
pub const fn sends(press: EnterPress, mode: SendKey) -> bool {
    if !press.enter || press.composing || press.shift {
        return false;
    }
    match mode {
        SendKey::Enter => press.with_mod() || !press.alt,
        SendKey::ModEnter => press.with_mod(),
    }
}

/// Whether `press` sends the other way (the delivery Settings did not
/// choose): Mod+Enter where Enter sends, Mod+Shift+Enter where Mod+Enter
/// does.
#[must_use]
pub const fn sends_other(press: EnterPress, mode: SendKey) -> bool {
    if !press.enter || press.composing || press.alt {
        return false;
    }
    match mode {
        SendKey::Enter => press.with_mod() && !press.shift,
        SendKey::ModEnter => press.with_mod() && press.shift,
    }
}

/// The send key as a person reads it; `primary` is the platform's main
/// modifier (`⌘` on a Mac, `Ctrl` elsewhere).
#[must_use]
pub fn send_key_label(mode: SendKey, primary: &str) -> String {
    match mode {
        SendKey::Enter => "Enter".to_owned(),
        SendKey::ModEnter => format!("{primary}+Enter"),
    }
}
