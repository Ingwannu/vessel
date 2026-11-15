//! Limits retain successful observation time independently from refresh outcomes.
use chrono::{DateTime, Utc};
use vessel_proto::limits::{AccountLimitState, LimitFailure, LimitSnapshot, UsageLimits};

/// A successful observation may be retained through a refresh or its failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitStatus {
    /// Credential use is explicitly disabled on this machine.
    Off,
    /// No external lookup has been requested.
    NotRequested,
    /// A refresh is in flight; any displayed snapshot is retained.
    Refreshing,
    /// Latest attempt produced this snapshot.
    Current,
    /// Latest attempt failed; any displayed snapshot is retained.
    Failed(LimitFailure),
}

/// Borrowed presentation cannot accidentally update the successful timestamp.
#[derive(Debug)]
pub struct LimitView<'a> {
    /// Latest outcome, which must be rendered even when values remain.
    pub status: LimitStatus,
    /// Successful observation only, including explicit absence of a report.
    pub snapshot: Option<&'a LimitSnapshot>,
    /// Actual cooldown from the node, never a guessed delay.
    pub retry_at: Option<DateTime<Utc>>,
}

/// The current status and original successful values are separate facts.
#[must_use]
pub fn limit_view(limits: &UsageLimits) -> LimitView<'_> {
    match &limits.state {
        AccountLimitState::Off => LimitView {
            status: LimitStatus::Off,
            snapshot: None,
            retry_at: None,
        },
        AccountLimitState::NotRequested => LimitView {
            status: LimitStatus::NotRequested,
            snapshot: None,
            retry_at: None,
        },
        AccountLimitState::Refreshing { retained } => LimitView {
            status: LimitStatus::Refreshing,
            snapshot: retained.as_ref(),
            retry_at: None,
        },
        AccountLimitState::Ready { snapshot } => LimitView {
            status: LimitStatus::Current,
            snapshot: Some(snapshot),
            retry_at: None,
        },
        AccountLimitState::Failed {
            reason,
            retained,
            retry_at,
        } => LimitView {
            status: LimitStatus::Failed(*reason),
            snapshot: retained.as_ref(),
            retry_at: *retry_at,
        },
    }
}

/// Vendor failure strings and credential-store paths never become interface copy.
#[must_use]
pub fn limit_failure(
    reason: LimitFailure,
    words: &'static crate::localization::Translator,
) -> &'static str {
    let text = words.usage();
    match reason {
        LimitFailure::NoLogin => text.no_login(),
        LimitFailure::Expired => text.expired(),
        LimitFailure::Rejected => text.rejected(),
        LimitFailure::Unreachable => text.unreachable(),
        LimitFailure::ChangedResponse => text.changed_response(),
        LimitFailure::Unsupported => text.unsupported_account(),
        LimitFailure::UnsafeLoginStore => text.unsafe_login(),
        LimitFailure::DifferentRoute => text.different_route(),
        LimitFailure::Unassociated => text.unassociated(),
        LimitFailure::RateLimited => text.rate_limited(),
        LimitFailure::Busy => text.busy(),
    }
}

#[cfg(test)]
mod tests;
