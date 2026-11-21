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
#[must_use]
pub fn pending(action: Action, catalog: &Translator) -> &str {
    let words = catalog.schedules();
    match action {
        Action::Pause => words.pausing(),
        Action::Resume => words.resuming(),
        Action::RunNow => words.starting(),
        Action::Delete => words.deleting(),
    }
}

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
        Failure::Node(code) => match code {
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

/// Localized badge for the node's state.
#[must_use]
pub fn badge(status: super::Status, catalog: &Translator) -> &str {
    use super::Status;
    use vessel_proto::automation::EndReason;
    let words = catalog.schedules();
    match status {
        Status::Active => words.active(),
        Status::Paused => words.paused(),
        Status::Ended(reason) => match reason {
            EndReason::MaxRuns => words.finished(),
            EndReason::Expired => words.expired(),
            EndReason::TargetGone => words.target_gone(),
            EndReason::CreatorGone => words.creator_gone(),
            EndReason::Unknown => words.unknown(),
        },
        Status::Unknown => words.unknown(),
    }
}

/// Action copy. Only self-owned schedules carry the heartbeat noun.
#[must_use]
pub fn action_label(action: Action, heartbeat: bool, catalog: &Translator) -> &str {
    let words = catalog.schedules();
    match action {
        Action::Pause => words.pause(),
        Action::Resume => words.resume(),
        Action::RunNow => words.run_now(),
        Action::Delete if heartbeat => words.delete_heartbeat(),
        Action::Delete => words.delete(),
    }
}

pub(super) fn cadence(
    cadence: &vessel_proto::automation::Cadence,
    catalog: &Translator,
) -> Result<String, crate::localization::FormatError> {
    use super::description::{CronShape, cron_shape};
    use chrono::Timelike as _;
    use vessel_proto::automation::Cadence;

    let words = catalog.schedules();
    match cadence {
        Cadence::Every { seconds, .. } => interval(*seconds, catalog),
        Cadence::Cron { expr, zone } => match cron_shape(expr) {
            Some(CronShape::EveryMinute) => Ok(words.every_minute().to_owned()),
            Some(CronShape::Hourly(at)) if at.minute() == 0 => Ok(words.every_hour().to_owned()),
            Some(CronShape::Hourly(at)) => words.hourly_at(&format!("{:02}", at.minute())),
            Some(CronShape::Fixed { at, days }) => words.fixed_at(
                day(days, catalog),
                &at.format("%H:%M").to_string(),
                zone.as_str(),
            ),
            None => Ok(expr.as_str().to_owned()),
        },
        Cadence::Unknown => Ok(words.newer_value().to_owned()),
    }
}

fn day(days: super::description::Days, catalog: &Translator) -> &str {
    use super::description::Days;
    use chrono::Weekday;
    let words = catalog.schedules();
    match days {
        Days::Daily => words.daily(),
        Days::Weekdays => words.weekdays(),
        Days::Weekends => words.weekends(),
        Days::On(day) => match day {
            Weekday::Mon => words.monday(),
            Weekday::Tue => words.tuesday(),
            Weekday::Wed => words.wednesday(),
            Weekday::Thu => words.thursday(),
            Weekday::Fri => words.friday(),
            Weekday::Sat => words.saturday(),
            Weekday::Sun => words.sunday(),
        },
    }
}

fn interval(
    seconds: vessel_proto::automation::EverySeconds,
    catalog: &Translator,
) -> Result<String, crate::localization::FormatError> {
    use super::cadence::{IntervalParts, IntervalUnit};
    use crate::localization::Number;

    let words = catalog.schedules();
    let whole = IntervalParts::of(seconds);
    let single = match (whole.unit(), whole.count()) {
        (IntervalUnit::Minutes, 1) => Some(words.every_minute()),
        (IntervalUnit::Hours, 1) => Some(words.every_hour()),
        (IntervalUnit::Days, 1) => Some(words.every_day()),
        (
            IntervalUnit::Seconds
            | IntervalUnit::Minutes
            | IntervalUnit::Hours
            | IntervalUnit::Days,
            _,
        ) => None,
    };
    if let Some(single) = single {
        return Ok(single.to_owned());
    }
    let mut remaining = seconds.get();
    let mut parts = Vec::new();
    for unit in IntervalUnit::DESCENDING {
        let count = if whole.unit() == IntervalUnit::Seconds {
            let count = remaining.checked_div(unit.seconds()).unwrap_or_default();
            remaining = remaining.checked_rem(unit.seconds()).unwrap_or_default();
            count
        } else if whole.unit() == unit {
            whole.count()
        } else {
            0
        };
        if count == 0 {
            continue;
        }
        let count = Number::count(count);
        parts.push(match unit {
            IntervalUnit::Days => words.interval_days(count),
            IntervalUnit::Hours => words.interval_hours(count),
            IntervalUnit::Minutes => words.interval_minutes(count),
            IntervalUnit::Seconds => words.interval_seconds(count),
        }?);
    }
    words.every_interval(&parts.join(" "))
}
