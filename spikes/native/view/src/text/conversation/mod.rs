//! The conversation's words (a session's timeline, its title bar and
//! Session actions, find, the context ring, a terminal session's frame),
//! in English and Korean. See `text/mod.rs` for the convention.
//!
//! [Decision Log]
//! - Purpose: every string through the tables, English and Korean
//!   together; the English is the page's, word for word, so a page
//!   test's expected sentence is this table's English (ADR 0036: one
//!   behaviour, checked on both sides).
//! - Constraint: about a hundred sentences, most of them with values in
//!   them (counts, names, outcomes); the view logic returns typed values
//!   (a run's counts, a request's outcome) and only these methods turn
//!   them into words.
//! - Choice: the convention's struct and accessor (`lang.conversation()`),
//!   its sentences as methods in files by subject (an entry's lines,
//!   numbers, the session's frame, a terminal's). A fixed word is written
//!   as an English and Korean pair where it is used (`either`), not as a
//!   field of the two constants: both languages still have to be written,
//!   side by side, or it does not compile.
//! - Cost: the fixed words are not listed in one place per language; a
//!   translator reads the methods.

mod entries;
mod meter;
mod numbers;
mod parking;
mod permission;
mod question;
mod search;
mod session;
mod terminal;

pub use numbers::{grouped, meter_tokens, short_tokens};

use super::Lang;

impl Lang {
    /// The conversation's words in this language.
    #[must_use]
    pub const fn conversation(self) -> &'static ConversationWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// The conversation's words in one language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversationWords {
    lang: Lang,
}

const EN: ConversationWords = ConversationWords { lang: Lang::En };

const KO: ConversationWords = ConversationWords { lang: Lang::Ko };

impl ConversationWords {
    /// Which language these are.
    #[must_use]
    pub const fn lang(&self) -> Lang {
        self.lang
    }

    /// A fixed word, by language: both are written where it is used, so
    /// neither can be missing.
    pub(crate) const fn either(self, en: &'static str, ko: &'static str) -> &'static str {
        match self.lang {
            Lang::En => en,
            Lang::Ko => ko,
        }
    }
}

/// `n` of something, with the English noun's number (Korean counts with a
/// counter word and no plural, so its sentences are written whole).
fn plural(n: u64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}
