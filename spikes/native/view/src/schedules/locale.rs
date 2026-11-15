//! New list words use the generated catalog; accepted cadence/action/error
//! tables remain in their original module until their separately owned move.
//!
//! [Decision Log]
//! - Purpose: an empty state and pending action have distinct complete words.
//! - Constraint: the existing accepted tables are not migrated or duplicated.
//! - Alternatives: render-time string keys or another pair of Rust tables.
//! - Choice: exhaustive typed choices over the generated area accessor.
//! - Why and cost: catalog shape is checked at build time; labels borrow the
//!   prepared language cache and contain no formatting work while painting.

use super::{Action, Bucket};
use crate::localization::Translator;

/// Filter-specific empty title, never reused as a badge or filter label.
#[must_use]
pub fn empty_title(bucket: Bucket, catalog: &Translator) -> &str {
    let words = catalog.schedules();
    match bucket {
        Bucket::Active => words.empty_active(),
        Bucket::Ended => words.empty_ended(),
        Bucket::Unknown => words.empty_unknown(),
    }
}

/// Only the active empty state has the reference's cadence explanation.
#[must_use]
pub fn empty_hint(bucket: Bucket, catalog: &Translator) -> Option<&str> {
    match bucket {
        Bucket::Active => Some(catalog.schedules().empty_description()),
        Bucket::Ended | Bucket::Unknown => None,
    }
}

/// A pending label replaces its exact menu action, with no second row signal.
/// Edit opens the form owner and has no row-local pending label.
#[must_use]
pub fn pending(action: Action, catalog: &Translator) -> Option<&str> {
    let words = catalog.schedules();
    match action {
        Action::Pause => Some(words.pausing()),
        Action::Resume => Some(words.resuming()),
        Action::RunNow => Some(words.starting()),
        Action::Delete => Some(words.deleting()),
        Action::Edit => None,
    }
}

#[cfg(test)]
mod failure_tests;
#[cfg(test)]
mod tests;

/// Read/mutation feedback except the separately accepted read-only authority
/// sentence. Full diagnostic sources stay in the model; none becomes UI copy.
#[must_use]
pub fn failure<'a>(failure: &super::load::Failure, catalog: &'a Translator) -> &'a str {
    use super::load::Failure;
    use vessel_proto::ErrorCode;
    let words = catalog.schedules();
    match failure {
        Failure::Older => words.failed_older(),
        Failure::NotEnabled => words.failed_not_enabled(),
        Failure::Offline => words.failed_offline(),
        Failure::ConnectionChanged => words.failed_connection_changed(),
        Failure::TimedOut => words.failed_timeout(),
        Failure::Busy => words.failed_busy(),
        Failure::Unexpected => words.failed_unexpected(),
        Failure::Unconfirmed => words.write_unconfirmed(),
        Failure::Exhausted => words.failed_exhausted(),
        Failure::NoTransport => words.failed_no_transport(),
        Failure::Preferences => words.failed_preferences(),
        Failure::Node(wire) => match wire.code {
            ErrorCode::InvalidRequest => words.failed_invalid(),
            ErrorCode::Unauthorized => words.failed_unauthorized(),
            ErrorCode::Forbidden => words.failed_view_forbidden(),
            ErrorCode::ProtocolMismatch => words.failed_protocol(),
            ErrorCode::NotFound => words.failed_not_found(),
            ErrorCode::Conflict => words.failed_conflict(),
            ErrorCode::Unsupported => words.failed_unsupported(),
            ErrorCode::HarnessNotInstalled => words.failed_agent_missing(),
            ErrorCode::SpawnFailed => words.failed_spawn(),
            ErrorCode::Unavailable => words.failed_unavailable(),
            ErrorCode::Internal => words.failed_internal(),
        },
    }
}
