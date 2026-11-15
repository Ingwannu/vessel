//! Only typed outcomes are retained here; the host logs original diagnostics.
use super::{OriginProblem, Problem};
use crate::localization::Translator;
use std::sync::Arc;

/// The safe recovery needed to reword a retained notice after navigation.
#[derive(Debug, Clone, Copy)]
pub enum Notice {
    /// The node refused a fork.
    Refused(Problem),
    /// The source conversation could not be read.
    Origin(OriginProblem),
}

impl Notice {
    fn words(self, catalog: &Translator) -> &str {
        match self {
            Self::Refused(problem) => problem.words(catalog),
            Self::Origin(problem) => problem.words(catalog),
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
    /// Prepare a safe recovery without accepting diagnostic text.
    #[must_use]
    pub fn new(outcome: Notice, catalog: &Translator) -> Self {
        Self {
            words: Arc::from(outcome.words(catalog)),
            outcome,
        }
    }

    /// Reword without another request, navigation or mutation of input.
    pub fn translate(&mut self, catalog: &Translator) {
        self.words = Arc::from(self.outcome.words(catalog));
    }

    /// Share retained text without allocation in a frame.
    #[must_use]
    pub fn words(&self) -> Arc<str> {
        Arc::clone(&self.words)
    }
}
