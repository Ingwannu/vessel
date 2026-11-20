//! The checks section's words (Paseo's `checksOverview` messages): a
//! group's label, a part of the detail line, the detail line itself and a
//! running check's time, each from the language's catalog.
//!
//! A message with a value fails to format only with a broken bundle; it
//! then reads as empty, so nothing panics while a frame is drawn.

use super::checks::{Presentation, Summary, Timing, duration};
use crate::localization::{Number, Translator};

/// A count at most `u32::MAX`.
fn number(count: usize) -> Number {
    Number::count(u32::try_from(count).unwrap_or(u32::MAX))
}

impl Presentation {
    /// One part of the detail line ("2 failing").
    #[must_use]
    pub fn count_words(self, count: usize, catalog: &Translator) -> String {
        let words = catalog.pulls();
        match self {
            Self::Failure => words.count_failing(number(count)),
            Self::Pending => words.count_pending(number(count)),
            Self::Success => words.count_passed(number(count)),
            Self::Ignored => words.count_skipped(number(count)),
        }
        .unwrap_or_default()
    }

    /// A group's label ("3 failing checks").
    #[must_use]
    pub fn group_words(self, count: usize, catalog: &Translator) -> String {
        let words = catalog.pulls();
        match self {
            Self::Failure => words.group_failing(number(count)),
            Self::Pending => words.group_pending(number(count)),
            Self::Success => words.group_passed(number(count)),
            Self::Ignored => words.group_skipped(number(count)),
        }
        .unwrap_or_default()
    }
}

impl Summary<'_> {
    /// The detail line ("2 failing, 4 in progress, 1 successful checks");
    /// empty with no checks.
    #[must_use]
    pub fn detail_words(&self, catalog: &Translator) -> String {
        let parts: Vec<String> = self
            .parts()
            .into_iter()
            .map(|(presentation, count)| presentation.count_words(count, catalog))
            .collect();
        if parts.is_empty() {
            return String::new();
        }
        catalog
            .pulls()
            .checks_detail(&parts.join(", "), number(self.total))
            .unwrap_or_default()
    }
}

impl Timing {
    /// A row's time: the duration, or "running 2m" while it runs.
    #[must_use]
    pub fn words(self, catalog: &Translator) -> String {
        let spent = duration(self.seconds);
        if self.running {
            catalog.pulls().check_running(&spent).unwrap_or(spent)
        } else {
            spent
        }
    }
}

#[cfg(test)]
mod tests;
