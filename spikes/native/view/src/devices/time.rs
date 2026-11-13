//! Paseo's `utils/time.ts`: no seconds that become stale immediately,
//! floor elapsed units, then a calendar date exactly at seven days.
//! The page's older rounded `Ago` remains for its existing parity cases.

use chrono::{DateTime, NaiveDate, Utc};

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
    /// The reference's thresholds. Exact timestamps are shown separately
    /// with an explicit UTC zone so the app never guesses a node's zone.
    #[must_use]
    pub fn of(at: DateTime<Utc>, now: DateTime<Utc>) -> Self {
        let millis = now.signed_duration_since(at).num_milliseconds().max(0);
        match millis {
            0..60_000 => Self::Now,
            60_000..3_600_000 => Self::Minutes(millis.div_euclid(60_000)),
            3_600_000..86_400_000 => Self::Hours(millis.div_euclid(3_600_000)),
            86_400_000..604_800_000 => Self::Days(millis.div_euclid(86_400_000)),
            _ => Self::Date(at.date_naive()),
        }
    }
}
