//! Preference application over the canonical multiline send policy.

use super::{Kind, MultilineStyle, SendOn, TextField};

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
