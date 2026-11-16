//! Closed display choices borrow prepared labels rather than format a selector
//! during a frame. This context is separate from Appearance's width labels.

use crate::localization::{Chat, General};
use crate::prefs::{ConversationDisplay, TurnDelivery};

impl<'a> Chat<'a> {
    /// The display picker's existing choice, with no frame-time formatting.
    #[must_use]
    pub fn display_choice(self, display: ConversationDisplay) -> &'a str {
        match display {
            ConversationDisplay::Full => self.display_full(),
            ConversationDisplay::CollapseTools => self.display_collapse_tools(),
        }
    }
}

impl<'a> General<'a> {
    /// The existing delivery choice, borrowed from the prepared catalog.
    #[must_use]
    pub fn delivery(self, delivery: TurnDelivery) -> &'a str {
        match delivery {
            TurnDelivery::Queue => self.queue(),
            TurnDelivery::Steer => self.steer(),
            TurnDelivery::Interrupt => self.interrupt(),
        }
    }

    /// A whole explanation for the selected delivery mode, without formatting.
    #[must_use]
    pub fn delivery_note(self, delivery: TurnDelivery) -> &'a str {
        match delivery {
            TurnDelivery::Queue => self.queue_hint(),
            TurnDelivery::Steer => self.steer_hint(),
            TurnDelivery::Interrupt => self.interrupt_hint(),
        }
    }
}

#[cfg(test)]
#[path = "words_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "general_words_tests.rs"]
mod general_tests;
