//! Closed display choices borrow prepared labels rather than format a selector
//! during a frame. This context is separate from Appearance's width labels.

use crate::localization::Chat;
use crate::prefs::ConversationDisplay;

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

#[cfg(test)]
#[path = "words_tests.rs"]
mod tests;
