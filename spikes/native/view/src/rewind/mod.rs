//! Typed rewind recoveries and retained frame notices, independent of UI/I/O.
//! Machine diagnostics stay at the host boundary; no message text is decoded.

use crate::localization::Translator;
use std::sync::Arc;
use vessel_proto::ErrorCode;

/// The safe recovery available from the node’s machine-readable error kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// The selected point cannot be used to rewind this conversation. Choose another message and try again.
    Point,
    /// This device is not allowed to rewind this conversation.
    Access,
    /// Update Vessel on this device and the machine before rewinding.
    Version,
    /// This conversation is no longer available.
    Missing,
    /// This conversation cannot be rewound right now. Open it again before retrying.
    NotReady,
    /// This agent cannot rewind conversations.
    Unsupported,
    /// This conversation's agent is not installed on the machine.
    AgentMissing,
    /// The agent could not be started to rewind this conversation. Try again.
    Start,
    /// The machine cannot rewind this conversation right now. Try again later.
    Unavailable,
    /// Could not confirm the rewind. Open the conversation again before retrying.
    Failed,
}

impl From<ErrorCode> for Problem {
    fn from(code: ErrorCode) -> Self {
        match code {
            ErrorCode::InvalidRequest => Self::Point,
            ErrorCode::Unauthorized | ErrorCode::Forbidden => Self::Access,
            ErrorCode::ProtocolMismatch => Self::Version,
            ErrorCode::NotFound => Self::Missing,
            ErrorCode::Conflict => Self::NotReady,
            ErrorCode::Unsupported => Self::Unsupported,
            ErrorCode::HarnessNotInstalled => Self::AgentMissing,
            ErrorCode::SpawnFailed => Self::Start,
            ErrorCode::Unavailable => Self::Unavailable,
            ErrorCode::Internal => Self::Failed,
        }
    }
}

impl Problem {
    /// Borrow a complete recovery sentence from the already prepared catalog.
    #[must_use]
    pub fn words(self, catalog: &Translator) -> &str {
        let words = catalog.rewind();
        match self {
            Self::Point => words.problem_point(),
            Self::Access => words.problem_access(),
            Self::Version => words.problem_version(),
            Self::Missing => words.problem_missing(),
            Self::NotReady => words.problem_not_ready(),
            Self::Unsupported => words.problem_unsupported(),
            Self::AgentMissing => words.problem_agent_missing(),
            Self::Start => words.problem_start(),
            Self::Unavailable => words.problem_unavailable(),
            Self::Failed => words.failed(),
        }
    }
}

/// A typed rewind outcome retained while its frame notice may still be visible.
#[derive(Debug)]
pub struct FailureNotice {
    problem: Problem,
    words: Arc<str>,
}

impl FailureNotice {
    /// Prepare a safe notice after a typed refusal, without its diagnostic.
    #[must_use]
    pub fn new(problem: Problem, catalog: &Translator) -> Self {
        Self {
            problem,
            words: Arc::from(problem.words(catalog)),
        }
    }

    /// Reword the same refusal without another request or input change.
    pub fn translate(&mut self, catalog: &Translator) {
        self.words = Arc::from(self.problem.words(catalog));
    }

    /// Share prepared text without allocating during a frame.
    #[must_use]
    pub fn words(&self) -> Arc<str> {
        Arc::clone(&self.words)
    }
}

#[cfg(test)]
mod tests;
