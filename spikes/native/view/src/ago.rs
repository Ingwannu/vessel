//! How long ago something happened, in the page's steps (`format.ts`'s
//! `ago`): seconds under a minute, minutes under an hour, hours under two
//! days, then days, each rounded to the nearest.

use chrono::{DateTime, Utc};

/// A time before now, in its largest fitting unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ago {
    /// Under a minute.
    Seconds(i64),
    /// Under an hour.
    Minutes(i64),
    /// Under two days.
    Hours(i64),
    /// Two days or more.
    Days(i64),
}

impl Ago {
    /// How long before `now` `at` was (a time after `now` is no time ago).
    #[must_use]
    pub fn of(at: DateTime<Utc>, now: DateTime<Utc>) -> Self {
        let millis = now.signed_duration_since(at).num_milliseconds().max(0);
        let seconds = nearest(millis, 1000);
        if seconds < 60 {
            return Self::Seconds(seconds);
        }
        let minutes = nearest(seconds, 60);
        if minutes < 60 {
            return Self::Minutes(minutes);
        }
        let hours = nearest(minutes, 60);
        if hours < 48 {
            return Self::Hours(hours);
        }
        Self::Days(nearest(hours, 24))
    }

    /// Whether it is under a minute.
    #[must_use]
    pub const fn just_now(self) -> bool {
        matches!(self, Self::Seconds(_))
    }
}

/// `count / unit` rounded half up (`Math.round`), for a count of zero or
/// more.
fn nearest(count: i64, unit: i64) -> i64 {
    count
        .saturating_add(unit.div_euclid(2))
        .checked_div_euclid(unit)
        .unwrap_or(0)
}
