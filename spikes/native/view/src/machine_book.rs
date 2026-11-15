//! The machine book's edit contract, independent of files and GPUI.
//!
//! [Decision Log]
//! - Purpose: a failed rename/forget must leave the selected machine intact.
//! - Constraints: names obey the CLI's ASCII grammar; file errors can contain
//!   credentials and must never become user-facing text.
//! - Alternatives: optimistic selection, or parsing error messages in a screen.
//! - Choice: validated names and typed edits; selection follows only success.
//! - Costs: link maps file failures once; UI language never depends on an OS error.

use crate::machines::NameProblem;

/// A validated name in the user's machine book.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name(String);

impl Name {
    /// Parse at the input boundary; whitespace is not part of a name.
    ///
    /// # Errors
    /// The name does not satisfy the book's grammar.
    pub fn parse(text: &str) -> Result<Self, NameProblem> {
        match crate::machines::name_problem(text, &[], None) {
            Some(problem) => Err(problem),
            None => Ok(Self(text.to_owned())),
        }
    }

    /// The validated text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A durable edit, sent only after any required confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// Move an entry whole; credentials and default follow the entry.
    Rename {
        /// Its existing name.
        from: Name,
        /// The validated replacement.
        to: Name,
    },
    /// Select where new sessions start by default.
    MakeDefault(Name),
    /// Disconnect and remove only the local book entry.
    Forget(Name),
}

impl Edit {
    /// Selection for a completed write; a failed write keeps it exactly as it was.
    #[must_use]
    pub fn selected_after(
        &self,
        result: &Result<(), Failure>,
        selected: Option<&str>,
    ) -> Option<String> {
        if result.is_err() {
            return selected.map(str::to_owned);
        }
        match self {
            Self::Rename { from, to } if selected == Some(from.as_str()) => {
                Some(to.as_str().to_owned())
            }
            Self::Forget(name) if selected == Some(name.as_str()) => None,
            Self::Rename { .. } | Self::MakeDefault(_) | Self::Forget(_) => {
                selected.map(str::to_owned)
            }
        }
    }
}

/// Safe, language-independent reasons. Raw file content is never carried here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// Safe file location and problem, shared with the app's file boundary.
    File(crate::files::FileFault),
    /// No configuration directory was found.
    NoConfiguration,
    /// The book could not be read.
    Read,
    /// The file is invalid or its default names a missing machine.
    InvalidBook,
    /// The entry was removed since the page was read.
    Missing,
    /// Another entry already uses that name.
    Taken,
    /// Writing the book failed.
    Write,
    /// The blocking file task stopped unexpectedly.
    Worker,
    /// The browser has no local book.
    Unavailable,
    /// The app's link did not take the request (its queue full, or
    /// stopped): nothing was read or written.
    Unsent,
}
