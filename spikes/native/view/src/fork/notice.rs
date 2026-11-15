//! Retained fork/origin notices; diagnostics are kept for the staged move only.
use crate::localization::{ForkFailedKind, ForkOriginUnreadKind, FormatError, Translator};
use std::sync::Arc;

/// The original facts needed to reword a retained notice after navigation.
#[derive(Debug)]
pub enum Notice {
    /// The node refused a fork.
    Refused(String),
    /// A source could not be read; a removed source has its own sentence.
    Origin {
        /// Whether the source no longer exists on its machine.
        removed: bool,
        /// Diagnostic retained by the staged wording-preserving move.
        message: String,
    },
}

impl Notice {
    fn format(&self, catalog: &Translator) -> Result<Arc<str>, FormatError> {
        let words = catalog.fork();
        match self {
            Self::Refused(message) => words
                .failed(
                    if message.trim().is_empty() {
                        ForkFailedKind::Plain
                    } else {
                        ForkFailedKind::Detail
                    },
                    message,
                )
                .map(Arc::from),
            Self::Origin { removed, message } => {
                let message = message.trim().trim_end_matches('.');
                words
                    .origin_unread(
                        if *removed {
                            ForkOriginUnreadKind::Removed
                        } else if message.is_empty() {
                            ForkOriginUnreadKind::Plain
                        } else {
                            ForkOriginUnreadKind::Detail
                        },
                        message,
                    )
                    .map(Arc::from)
            }
        }
    }
}

/// Shared text retained by the host at the response/language event.
#[derive(Debug)]
pub struct PreparedNotice {
    outcome: Notice,
    words: Arc<str>,
}

impl PreparedNotice {
    /// # Errors
    /// Keeps the formatter error if both translation and fallback fail.
    pub fn new(outcome: Notice, catalog: &Translator) -> Result<Self, FormatError> {
        Ok(Self {
            words: outcome.format(catalog)?,
            outcome,
        })
    }

    /// Reword without another request, navigation or mutation of input.
    /// # Errors
    /// Keeps the previous prepared text if formatting fails.
    pub fn translate(&mut self, catalog: &Translator) -> Result<(), FormatError> {
        self.words = self.outcome.format(catalog)?;
        Ok(())
    }

    /// Share retained text without allocation in a frame.
    #[must_use]
    pub fn words(&self) -> Arc<str> {
        Arc::clone(&self.words)
    }
}
