//! Rewind notices prepared at an answer or language change, never in a frame.
//! The operation's capabilities and composer restoration remain in timeline/app.
//!
//! [Decision Log]
//! - Purpose: migrate the existing menu and notice words independently of the
//!   later correction to the node's English-only diagnostic boundary.
//! - Choice: retain notice input and shared formatted text, using the generated
//!   catalog; an existing notice can then change language without another act.

use std::sync::Arc;

use crate::localization::{FormatError, RewindFailedKind, Translator};

/// A retained refusal notice. The following typed-error correction replaces
/// its diagnostic input; this move preserves the former displayed sentence.
#[derive(Debug)]
pub struct FailureNotice {
    message: Arc<str>,
    words: Arc<str>,
}

impl FailureNotice {
    /// Prepare the notice when the machine answers.
    /// # Errors
    /// Returns a typed formatting error if translation and English both fail.
    pub fn new(message: &str, catalog: &Translator) -> Result<Self, FormatError> {
        Ok(Self {
            message: Arc::from(message),
            words: failed_words(message, catalog)?,
        })
    }

    /// Reword a retained outcome after changing language.
    /// # Errors
    /// Leaves the previous text intact if formatting fails.
    pub fn translate(&mut self, catalog: &Translator) -> Result<(), FormatError> {
        self.words = failed_words(&self.message, catalog)?;
        Ok(())
    }

    /// Share the prepared text without copying it in a frame.
    #[must_use]
    pub fn words(&self) -> Arc<str> {
        Arc::clone(&self.words)
    }
}

fn failed_words(message: &str, catalog: &Translator) -> Result<Arc<str>, FormatError> {
    let kind = if message.trim().is_empty() {
        RewindFailedKind::Plain
    } else {
        RewindFailedKind::Detail
    };
    catalog.rewind().failed(kind, message).map(Arc::from)
}

#[cfg(test)]
mod tests;
