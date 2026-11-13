//! Paseo's `utils/time.ts`: no seconds that become stale immediately,
//! floor elapsed units, then a calendar date exactly at seven days.
//! The page's older rounded `Ago` remains for its existing parity cases.
//!
//! [Decision Log]
//! - Purpose: exact history is read in the viewer's zone, to the second.
//! - Constraint: view must do no system I/O or guess the node's zone.
//! - Choice: the app reads its local offset once and passes `FixedOffset`.
//! - Why: the same explicit offset governs old calendar dates and hover
//!   instants, so they cannot disagree around midnight.
//! - Cost: a timezone change during this app run takes effect on restart.

use chrono::{DateTime, FixedOffset, NaiveDate, Utc};

/// A relative instant before it is worded by the language table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Age {
    /// Less than one minute ago, including a future clock skew.
    Now,
    /// Whole minutes, below one hour.
    Minutes(i64),
    /// Whole hours, below one day.
    Hours(i64),
    /// Whole days, below seven days.
    Days(i64),
    /// A calendar date, from seven days onward.
    Date(NaiveDate),
}

impl Age {
    /// The reference's thresholds, with older dates in the viewer's zone.
    /// The caller supplies its offset; view never reads the system clock.
    #[must_use]
    pub fn of(at: DateTime<Utc>, now: DateTime<Utc>, offset: FixedOffset) -> Self {
        let millis = now.signed_duration_since(at).num_milliseconds().max(0);
        match millis {
            0..60_000 => Self::Now,
            60_000..3_600_000 => Self::Minutes(millis.div_euclid(60_000)),
            3_600_000..86_400_000 => Self::Hours(millis.div_euclid(3_600_000)),
            86_400_000..604_800_000 => Self::Days(millis.div_euclid(86_400_000)),
            _ => Self::Date(at.with_timezone(&offset).date_naive()),
        }
    }
}

/// An invitation's remaining lifetime, rounded down to whole minutes.
///
/// [Decision Log]
/// - Purpose: a ten-minute invitation should read as time left, with no
///   stale absolute machine timestamp in the row's title.
/// - Constraints: expiry is authoritative, independent of reveal masking.
/// - Alternatives: ceil minutes (overstates time left), or second polling.
/// - Choice: whole minutes, a final under-one-minute sentence, then the
///   existing expired sentence; wake only when the wording changes.
/// - Cost: a wall-clock correction is reflected at the next minute at most.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Remaining {
    /// Whole minutes left (at least one).
    Minutes(i64),
    /// Positive lifetime shorter than one minute.
    Soon,
    /// No longer valid.
    Expired,
}

impl Remaining {
    /// Interpret the node's expiry without reading a clock or doing I/O.
    #[must_use]
    pub fn of(expires: DateTime<Utc>, now: DateTime<Utc>) -> Self {
        let millis = expires.signed_duration_since(now).num_milliseconds();
        if now >= expires {
            Self::Expired
        } else if millis < 60_000 {
            Self::Soon
        } else {
            Self::Minutes(millis.div_euclid(60_000))
        }
    }

    /// Time until the displayed whole-minute count changes or expires.
    /// The extra millisecond crosses an exact minute's floor boundary.
    #[must_use]
    pub fn next_change(expires: DateTime<Utc>, now: DateTime<Utc>) -> Option<std::time::Duration> {
        let millis = expires.signed_duration_since(now).num_milliseconds();
        if now >= expires {
            return None;
        }
        let wait = millis
            .rem_euclid(60_000)
            .saturating_add(1)
            .min(millis.max(1));
        Some(std::time::Duration::from_millis(
            u64::try_from(wait).unwrap_or(60_000),
        ))
    }
}

/// A timestamp in the viewer's local offset, to the second. The zone is
/// named explicitly; fractional seconds from storage do not reach text.
#[must_use]
pub fn exact_time(at: DateTime<Utc>, offset: FixedOffset) -> String {
    let seconds = offset.local_minus_utc();
    let sign = if seconds < 0 { "-" } else { "+" };
    let magnitude = seconds.unsigned_abs();
    let hours = magnitude.div_euclid(3600);
    let minutes = magnitude.rem_euclid(3600).div_euclid(60);
    let seconds = magnitude.rem_euclid(60);
    let zone = if seconds != 0 {
        format!("UTC{sign}{hours}:{minutes:02}:{seconds:02}")
    } else if minutes != 0 {
        format!("UTC{sign}{hours}:{minutes:02}")
    } else {
        format!("UTC{sign}{hours}")
    };
    format!(
        "{} ({zone})",
        at.with_timezone(&offset).format("%Y-%m-%d %H:%M:%S")
    )
}
