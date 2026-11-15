//! Whole translated notices; raw node/provider errors never enter the UI.
use super::TrackProblem;
use crate::localization::Translator;
impl TrackProblem {
    /// Safe inline notice for this typed failure.
    #[must_use]
    pub fn words(self, translator: &Translator) -> &str {
        let words = translator.subagents();
        match self {
            Self::Offline => words.offline(),
            Self::Unsupported => words.unsupported(),
            Self::IdentityChanged => words.identity_changed(),
            Self::Timeout => words.timeout(),
            Self::Uncertain => words.uncertain(),
            Self::InvalidReply => words.invalid_reply(),
            Self::Exhausted => words.exhausted(),
            Self::Busy => words.busy(),
            Self::RetentionLimit => words.retention_limit(),
            Self::Refused(_) => words.refused(),
        }
    }
}
