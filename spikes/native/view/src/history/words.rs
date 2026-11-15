//! Whole translated failures, never a raw node diagnostic.

use super::Problem;
use crate::localization::Translator;
use vessel_proto::ErrorCode;

impl Problem {
    /// The inline failure next to the history list or import action.
    #[must_use]
    pub fn words(self, translator: &Translator) -> &str {
        let words = translator.history();
        match self {
            Self::Offline => words.offline(),
            Self::Unsent => words.unsent(),
            Self::Unsupported
            | Self::Refused(ErrorCode::Unsupported | ErrorCode::ProtocolMismatch) => {
                words.unsupported()
            }
            Self::Timeout => words.timeout(),
            Self::Uncertain => words.uncertain(),
            Self::IdentityChanged => words.identity_changed(),
            Self::InvalidReply => words.invalid_reply(),
            Self::Capacity => words.capacity(),
            Self::Exhausted => words.exhausted(),
            Self::Refused(ErrorCode::Forbidden | ErrorCode::Unauthorized) => words.forbidden(),
            Self::Refused(ErrorCode::Conflict) => words.conflict(),
            Self::Refused(
                ErrorCode::InvalidRequest
                | ErrorCode::NotFound
                | ErrorCode::Internal
                | ErrorCode::Unavailable
                | ErrorCode::HarnessNotInstalled
                | ErrorCode::SpawnFailed,
            ) => words.refused(),
        }
    }
}
