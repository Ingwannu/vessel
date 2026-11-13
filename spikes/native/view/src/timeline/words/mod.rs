//! The conversation's sentences in the app's one string table (`Text`):
//! methods of it, in English and Korean, kept in these files.
//!
//! [Decision Log]
//! - Purpose: every string through the one table, English and Korean
//!   together (the milestone's rule); the English is the page's, word for
//!   word, so a page test's expected sentence is this table's English
//!   (ADR 0036: one behaviour, checked on both sides).
//! - Constraint: `view/src/text.rs` is every track's; a hundred sentences
//!   of one area written into its struct and its two constants would make
//!   each merge a conflict.
//! - Choice: the conversation's sentences are methods of `Text`, as the
//!   table's own sentences with values are (each matching on its
//!   language, so a sentence without its Korean does not compile), in
//!   these files: `impl Text` blocks beside the logic they word. The
//!   view logic returns typed values (a run's counts, a request's
//!   outcome); only these methods turn them into words.
//! - Cost: a fixed word of the conversation is a method (`either`), not a
//!   field of the two constants; both languages are still written side
//!   by side and cannot be left out.

mod entries;
mod numbers;
mod session;

pub use numbers::{grouped, meter_tokens, short_tokens};

use crate::{Lang, Text};

impl Text {
    /// A fixed word of the conversation, by language: both are written
    /// where it is used, so neither can be missing.
    pub(crate) const fn either(&self, en: &'static str, ko: &'static str) -> &'static str {
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
