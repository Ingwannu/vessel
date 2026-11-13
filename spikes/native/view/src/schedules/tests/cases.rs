//! Stable fixture ids map explicitly to decisions; Debug is never a contract.

use super::super::{Action, Bucket, Variant, history::Outcome};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ExpectedBucket {
    Active,
    Ended,
    Unknown,
}
impl From<ExpectedBucket> for Bucket {
    fn from(value: ExpectedBucket) -> Self {
        match value {
            ExpectedBucket::Active => Self::Active,
            ExpectedBucket::Ended => Self::Ended,
            ExpectedBucket::Unknown => Self::Unknown,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ExpectedAction {
    Edit,
    Pause,
    Resume,
    RunNow,
    Delete,
}
impl From<ExpectedAction> for Action {
    fn from(value: ExpectedAction) -> Self {
        match value {
            ExpectedAction::Edit => Self::Edit,
            ExpectedAction::Pause => Self::Pause,
            ExpectedAction::Resume => Self::Resume,
            ExpectedAction::RunNow => Self::RunNow,
            ExpectedAction::Delete => Self::Delete,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ExpectedBadge {
    Success,
    Muted,
    Error,
}
impl From<ExpectedBadge> for Variant {
    fn from(value: ExpectedBadge) -> Self {
        match value {
            ExpectedBadge::Success => Self::Success,
            ExpectedBadge::Muted => Self::Muted,
            ExpectedBadge::Error => Self::Error,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ExpectedOutcome {
    Making,
    Running,
    Waiting,
    Finished,
    Prompted,
    Steered,
    Queued,
    Skipped,
    Failed,
    Cancelled,
    Limited,
    Refused,
    SessionEnded,
    Interrupted,
    Unknown,
}
impl From<ExpectedOutcome> for Outcome {
    fn from(value: ExpectedOutcome) -> Self {
        match value {
            ExpectedOutcome::Making => Self::Making,
            ExpectedOutcome::Running => Self::Running,
            ExpectedOutcome::Waiting => Self::Waiting,
            ExpectedOutcome::Finished => Self::Finished,
            ExpectedOutcome::Prompted => Self::Prompted,
            ExpectedOutcome::Steered => Self::Steered,
            ExpectedOutcome::Queued => Self::Queued,
            ExpectedOutcome::Skipped => Self::Skipped,
            ExpectedOutcome::Failed => Self::Failed,
            ExpectedOutcome::Cancelled => Self::Cancelled,
            ExpectedOutcome::Limited => Self::Limited,
            ExpectedOutcome::Refused => Self::Refused,
            ExpectedOutcome::SessionEnded => Self::SessionEnded,
            ExpectedOutcome::Interrupted => Self::Interrupted,
            ExpectedOutcome::Unknown => Self::Unknown,
        }
    }
}
