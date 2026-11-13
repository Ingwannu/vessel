//! Every word the conversation's screens show, in English and Korean.
//!
//! [Decision Log]
//! - Purpose: every string through one table per language from the first
//!   line (the milestone's rule); the English is the page's, word for
//!   word, so a page test's expected sentence is this table's English
//!   (ADR 0036: one behaviour, checked on both sides).
//! - Constraint: the app's string table (`Text`, the lead's) is being
//!   written at the same time on another branch; editing it from here
//!   would conflict on every line.
//! - Choice: the conversation's words as a table of their own, the same
//!   shape as the app's (a language field, a `pick` for fixed words,
//!   methods for sentences with values, each language's word order its
//!   own), reached from the app's table by one field once both are on
//!   one branch. The view logic returns typed values (a run's counts, a
//!   request's outcome); only this module turns them into words.
//! - Cost: two tables until they are joined; `Lang` here is the app's
//!   until then.

mod entries;
mod numbers;
mod session;

pub use numbers::{grouped, meter_tokens, short_tokens};

/// A language the words are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    /// English.
    En,
    /// Korean.
    Ko,
}

/// The conversation's words in one language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Words {
    lang: Lang,
}

/// The English words (the page's).
pub const EN: Words = Words { lang: Lang::En };

/// The Korean words.
pub const KO: Words = Words { lang: Lang::Ko };

impl Words {
    /// The words of `lang`.
    #[must_use]
    pub const fn of(lang: Lang) -> Self {
        Self { lang }
    }

    /// Which language these are.
    #[must_use]
    pub const fn lang(self) -> Lang {
        self.lang
    }

    /// A fixed word, by language.
    #[must_use]
    pub const fn pick(self, en: &'static str, ko: &'static str) -> &'static str {
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
