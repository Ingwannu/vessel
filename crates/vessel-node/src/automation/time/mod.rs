//! When a schedule is due (ADR 0033): the next due time after an instant,
//! for an elapsed interval or a cron expression in a time zone, and what
//! to do about due times that passed unrun. Pure: every function takes
//! the instant it reasons about, so the tests give the clock and none of
//! them waits.
//!
//! [Decision Log]
//! - Purpose: "at nine on weekdays in Seoul" and "every 30 minutes" as
//!   instants the scheduler can sleep until, with every edge decided: a
//!   wall-clock time a daylight-saving change skips or repeats, a node
//!   stopped across due times, a clock stepped either way.
//! - Constraint: the scheduler keeps due times as UTC instants (one ordered
//!   index, compared to one clock reading), so all of a zone's knowledge
//!   is spent here, once per run.
//! - Alternatives: a cron crate (see `vessel_proto::automation`'s cron
//!   module); Paseo's minute-by-minute walk over real instants (a year of
//!   minutes per search, and a time a gap skips never runs); GOAT's server
//!   zone only.
//! - Choice: walk civil (wall-clock) minutes that match the expression,
//!   field by field ([`cron`]), and turn each into an instant with the
//!   zone's rules from `jiff`'s bundled IANA database ([`Zones`]). A civil
//!   time in a gap becomes the instant the gap ends; one in a fold its
//!   first occurrence; an instant not after the one asked about is passed
//!   over. So a time skipped by the clocks runs once, right after the
//!   jump (Vixie cron's rule), a repeated one runs once, and the walk
//!   never returns a due time twice.
//! - Cost: the database is the one built into the binary: a zone whose
//!   rules change after a release is computed by the old rules until the
//!   next one. In exchange the same schedule gives the same times on every
//!   platform and in every test, and the node reads no zone files.

mod cron;
mod missed;

use chrono::{DateTime, Utc};
use jiff::Timestamp;
use jiff::tz::{TimeZone, TimeZoneDatabase};
use vessel_proto::automation::{Cadence, CronExpr, EverySeconds, TimeZoneName};

pub(crate) use missed::{Decision, missed, too_old};

/// How far ahead a cron's next time is looked for. Every expression the
/// grammar takes that matches some day matches one within 8 years (a
/// 29 February skips a year only around a century not divisible by 400),
/// so an expression with none there never runs.
pub(crate) const LOOK_AHEAD_YEARS: i16 = 8;

/// The time zones, from the IANA database bundled into the binary.
#[derive(Debug)]
pub(crate) struct Zones {
    database: TimeZoneDatabase,
}

impl Default for Zones {
    fn default() -> Self {
        Self {
            database: TimeZoneDatabase::bundled(),
        }
    }
}

/// A zone the database does not have.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown time zone `{0}` (an IANA name, such as Europe/London or Asia/Seoul)")]
pub(crate) struct UnknownZone(pub(crate) TimeZoneName);

impl Zones {
    /// The zone `name` names.
    pub(crate) fn zone(&self, name: &TimeZoneName) -> Result<TimeZone, UnknownZone> {
        self.database
            .get(name.as_str())
            .map_err(|_unknown| UnknownZone(name.clone()))
    }
}

/// A cadence ready to compute with: its zone looked up, its anchor set.
#[derive(Debug)]
pub(crate) enum Due<'a> {
    /// Every `seconds` from `anchor`.
    Every {
        seconds: EverySeconds,
        anchor: DateTime<Utc>,
    },
    /// The expression's times in `zone`.
    Cron { expr: &'a CronExpr, zone: TimeZone },
}

/// What [`Due::of`] cannot compute with.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum NotComputable {
    /// The zone is not in the database.
    #[error(transparent)]
    Zone(#[from] UnknownZone),
    /// A cadence a newer node wrote.
    #[error("a cadence this node does not know")]
    Unknown,
}

impl<'a> Due<'a> {
    /// The first due time strictly after `after`; `None` when there is
    /// none (a cron with no matching day within [`LOOK_AHEAD_YEARS`], or
    /// past what a time can hold).
    pub(crate) fn next_after(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Self::Every { seconds, anchor } => every_after(*seconds, *anchor, after),
            Self::Cron { expr, zone } => {
                let next = cron::next_after(expr.fields(), zone, timestamp(after)?)?;
                datetime(next)
            }
        }
    }

    /// `cadence` with its zone looked up in `zones`; an `every` without an
    /// anchor counts from `set_at`, when the cadence was set.
    pub(crate) fn of(
        cadence: &'a Cadence,
        set_at: DateTime<Utc>,
        zones: &Zones,
    ) -> Result<Self, NotComputable> {
        match cadence {
            Cadence::Every { seconds, anchor } => Ok(Self::Every {
                seconds: *seconds,
                anchor: anchor.unwrap_or(set_at),
            }),
            Cadence::Cron { expr, zone } => Ok(Self::Cron {
                expr,
                zone: zones.zone(zone)?,
            }),
            Cadence::Unknown => Err(NotComputable::Unknown),
        }
    }
}

/// `anchor + k × seconds` for the smallest `k ≥ 0` after `after`.
fn every_after(
    seconds: EverySeconds,
    anchor: DateTime<Utc>,
    after: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    if after < anchor {
        return Some(anchor);
    }
    let step = i64::from(seconds.get());
    // Whole intervals since the anchor, then one more: the first due time
    // after `after` (the division rounds down, `after ≥ anchor`).
    let elapsed = after.signed_duration_since(anchor).num_seconds();
    let intervals = elapsed.checked_div(step)?.checked_add(1)?;
    let mut due = anchor.checked_add_signed(chrono::TimeDelta::try_seconds(
        intervals.checked_mul(step)?,
    )?)?;
    // `num_seconds` drops the fraction; a due time equal to `after` (an
    // anchor with nanoseconds) is not after it.
    if due <= after {
        due = due.checked_add_signed(chrono::TimeDelta::try_seconds(step)?)?;
    }
    Some(due)
}

/// A chrono instant as a jiff one.
fn timestamp(at: DateTime<Utc>) -> Option<Timestamp> {
    let nanos = i32::try_from(at.timestamp_subsec_nanos()).ok()?;
    Timestamp::new(at.timestamp(), nanos).ok()
}

/// A jiff instant as a chrono one.
fn datetime(at: Timestamp) -> Option<DateTime<Utc>> {
    let nanos = u32::try_from(at.subsec_nanosecond()).ok()?;
    DateTime::from_timestamp(at.as_second(), nanos)
}

#[cfg(test)]
mod tests;
