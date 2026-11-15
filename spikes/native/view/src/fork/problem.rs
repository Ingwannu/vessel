//! Typed fork and origin-read recoveries; diagnostics never cross into words.
use crate::localization::Translator;
use vessel_proto::ErrorCode;

/// The safe recovery available from the node’s machine-readable error kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// The selected point cannot be used to fork this conversation. Choose another turn and try again.
    Point,
    /// This device is not allowed to fork this conversation.
    Access,
    /// Update Vessel on this device and the machine before forking.
    Version,
    /// This conversation is no longer available.
    Missing,
    /// This conversation cannot be forked right now. Open it again before retrying.
    NotReady,
    /// This agent cannot fork conversations.
    Unsupported,
    /// This conversation's agent is not installed on the machine.
    AgentMissing,
    /// The agent could not be started to fork this conversation. Try again.
    Start,
    /// The machine cannot fork this conversation right now. Try again later.
    Unavailable,
    /// Could not confirm the fork. Check the session list before trying again.
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
        let words = catalog.fork();
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

/// Why the source log could not be read, independent of transport diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginProblem {
    /// The conversation it was forked from was removed.
    Removed,
    /// This device is not allowed to read the conversation it was forked from.
    Access,
    /// Update Vessel on this device and the machine to read the conversation it was forked from.
    Version,
    /// The machine is unavailable. Open the session again to retry reading the conversation it was forked from.
    Unavailable,
    /// The conversation it was forked from could not be read. Open the session again to try again.
    Failed,
}

impl From<Option<ErrorCode>> for OriginProblem {
    fn from(code: Option<ErrorCode>) -> Self {
        match code {
            Some(ErrorCode::NotFound) => Self::Removed,
            Some(ErrorCode::Unauthorized | ErrorCode::Forbidden) => Self::Access,
            Some(ErrorCode::ProtocolMismatch) => Self::Version,
            Some(ErrorCode::Unavailable) | None => Self::Unavailable,
            Some(
                ErrorCode::InvalidRequest
                | ErrorCode::Conflict
                | ErrorCode::Unsupported
                | ErrorCode::HarnessNotInstalled
                | ErrorCode::SpawnFailed
                | ErrorCode::Internal,
            ) => Self::Failed,
        }
    }
}

impl OriginProblem {
    /// Borrow its whole recovery sentence from the already prepared catalog.
    #[must_use]
    pub fn words(self, catalog: &Translator) -> &str {
        let words = catalog.fork();
        match self {
            Self::Removed => words.origin_removed(),
            Self::Access => words.origin_access(),
            Self::Version => words.origin_version(),
            Self::Unavailable => words.origin_unavailable(),
            Self::Failed => words.origin_failed(),
        }
    }
}
