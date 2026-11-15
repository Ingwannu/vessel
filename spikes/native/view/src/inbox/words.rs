//! Whole localized labels for attention state and safe failures. No raw peer
//! diagnostic is used as a notification body or a user-facing failure.
use super::{Content, Problem};
use crate::localization::Translator;
use vessel_proto::ErrorCode;

/// A short kind label shared by list rows and OS notifications.
#[must_use]
pub fn kind<'a>(content: &Content, words: &'a Translator) -> &'a str {
    let words = words.inbox();
    match content {
        Content::Permission { .. } | Content::Question(_) | Content::InSession => words.waiting(),
        Content::Finished => words.finished(),
        Content::Failed => words.failed(),
        Content::Limit => words.limit(),
        Content::Unknown => words.unknown(),
    }
}

/// Safe errors remain translatable after a language change.
#[must_use]
pub fn problem(problem: Problem, words: &Translator) -> &str {
    let words = words.inbox();
    match problem {
        Problem::Offline => words.offline(),
        Problem::InvalidReply => words.invalid(),
        Problem::Exhausted => words.exhausted(),
        Problem::Refused(code) => match code {
            ErrorCode::Unauthorized | ErrorCode::Forbidden => words.forbidden(),
            ErrorCode::Unsupported | ErrorCode::ProtocolMismatch => words.unsupported(),
            ErrorCode::NotFound | ErrorCode::Conflict => words.resolved(),
            ErrorCode::InvalidRequest
            | ErrorCode::Internal
            | ErrorCode::Unavailable
            | ErrorCode::HarnessNotInstalled
            | ErrorCode::SpawnFailed => words.unavailable(),
        },
    }
}
