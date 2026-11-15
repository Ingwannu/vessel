//! Whole permission-read recovery sentences; no remote prose crosses here.
use super::{DevicesWords, Lang};
use crate::{devices::Problem, localization::Translator};
use vessel_proto::ErrorCode;
impl DevicesWords {
    /// One whole recovery sentence for an interrupted sent operation.
    #[must_use]
    pub fn unconfirmed<'a>(
        &self,
        operation: crate::devices::Unconfirmed,
        t: &'a Translator,
    ) -> &'a str {
        let words = t.device_authority();
        match operation {
            crate::devices::Unconfirmed::Invitation => words.uncertain_invitation(),
            crate::devices::Unconfirmed::Revocation => words.uncertain_revocation(),
            crate::devices::Unconfirmed::Both => words.uncertain_both(),
        }
    }

    /// A failed self query remains unknown, including a semantic refusal.
    #[must_use]
    pub fn authority_problem<'a>(&self, why: Problem, t: &'a Translator) -> &'a str {
        let words = t.device_authority();
        match (self.lang, why) {
            (Lang::En | Lang::Ko, Problem::TargetChanged) => words.changed(),
            (Lang::En | Lang::Ko, Problem::Offline) => words.offline(),
            (Lang::En | Lang::Ko, Problem::Exhausted) => words.exhausted(),
            (Lang::En | Lang::Ko, Problem::InvalidReply | Problem::NoRoute) => words.invalid(),
            (Lang::En | Lang::Ko, Problem::AccessNotConfirmed) => words.not_confirmed(),
            (
                Lang::En | Lang::Ko,
                Problem::Refused(ErrorCode::Unsupported | ErrorCode::ProtocolMismatch),
            ) => words.unsupported(),
            (
                Lang::En | Lang::Ko,
                Problem::Refused(ErrorCode::Unauthorized | ErrorCode::Forbidden),
            ) => words.refused(),
            (
                Lang::En | Lang::Ko,
                Problem::Refused(
                    ErrorCode::InvalidRequest
                    | ErrorCode::NotFound
                    | ErrorCode::Conflict
                    | ErrorCode::HarnessNotInstalled
                    | ErrorCode::SpawnFailed
                    | ErrorCode::Unavailable
                    | ErrorCode::Internal,
                ),
            ) => words.unavailable(),
        }
    }
}

#[cfg(test)]
mod tests;
