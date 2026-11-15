//! One submit listener reads the effective composer gesture policy.

use super::{Kind, MultilineStyle, TextField};

/// Which press sends a composer's text (Settings › General › Send
/// shortcut, the page's `compose-keys.ts`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SendOn {
    /// Enter (and Mod+Enter) sends; Shift+Enter starts a line.
    #[default]
    Enter,
    /// Mod+Enter sends; Enter and Shift+Enter start a line.
    ModEnter,
    /// Compact layouts submit through a button; every Enter remains text input.
    ButtonOnly,
}

impl SendOn {
    /// Whether an Enter that is not Shift+Enter sends; `with_mod` when
    /// Command (Control elsewhere) was held.
    #[must_use]
    pub const fn sends(self, with_mod: bool) -> bool {
        match self {
            Self::Enter => true,
            Self::ModEnter => with_mod,
            Self::ButtonOnly => false,
        }
    }
}

impl TextField {
    /// Which press sends a composer's text (a form field ignores it).
    /// Preference reapplication cannot replace an active compact `ButtonOnly` policy;
    /// the next window-aware composer layout restores its current desktop preference.
    pub fn set_send_on(&self, send: SendOn) {
        if matches!(self.kind, Kind::Lines(_, MultilineStyle::Composer))
            && (self.send.get() != SendOn::ButtonOnly || send == SendOn::ButtonOnly)
        {
            self.send.set(send);
        }
    }
}
