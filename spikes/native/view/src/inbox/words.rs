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

/// Shared status headings for the sidebar and its keyboard ordering.
#[must_use]
pub fn bucket(bucket: super::buckets::Bucket, words: &Translator) -> &str {
    use super::buckets::Bucket;
    let words = words.inbox();
    match bucket {
        Bucket::NeedsInput => words.bucket_input(),
        Bucket::Failed => words.bucket_failed(),
        Bucket::Working => words.bucket_working(),
        Bucket::Review => words.bucket_review(),
        Bucket::Done => words.bucket_done(),
    }
}

/// A plain shell has unknown activity, even though it contributes no active
/// workspace bucket. Aggregate Done must not be displayed as a completed job.
#[must_use]
pub fn terminal<'a>(row: &super::terminals::Row, words: &'a Translator) -> &'a str {
    use vessel_proto::notifications::{TerminalActivity, TerminalAttentionReason};
    let words = words.inbox();
    if row.bucket() == Some(super::buckets::Bucket::NeedsInput) {
        return words.waiting();
    }
    if row.failed {
        return words.failed();
    }
    match row.attention.map(|state| state.activity) {
        Some(TerminalActivity::Idle) => words.terminal_idle(),
        Some(TerminalActivity::Working) => words.bucket_working(),
        Some(TerminalActivity::Attention {
            reason: TerminalAttentionReason::Finished,
        }) => words.finished(),
        Some(TerminalActivity::Attention {
            reason: TerminalAttentionReason::NeedsInput,
        }) => words.waiting(),
        Some(
            TerminalActivity::Untracked
            | TerminalActivity::Unknown
            | TerminalActivity::Attention {
                reason: TerminalAttentionReason::Unknown,
            },
        )
        | None => words.terminal_unknown(),
    }
}
