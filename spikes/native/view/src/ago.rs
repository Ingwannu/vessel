//! How long ago something happened, in the page's steps (`format.ts`'s
//! `ago`): seconds under a minute, minutes under an hour, hours under two
//! days, then days, each rounded to the nearest.

use chrono::{DateTime, Datelike as _, Utc};

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

/// Paseo's compact age for a dense row (`utils/time.ts`
/// `describeCompactTimeAgo`, lines 80–84, from `describeAge` lines 36–58):
/// `now` under a minute, then `Nm`, `Nh`, `Nd` through six days, then the
/// UTC month and day (`Jan 15`) with no "ago". The month is the English
/// short name, as `toLocaleDateString("en-US")` writes it.
#[must_use]
pub fn compact_ago(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let seconds = now.signed_duration_since(at).num_seconds();
    if seconds < 60 {
        return "now".to_owned();
    }
    let minutes = quot(seconds, 60);
    if minutes < 60 {
        return format!("{minutes}m");
    }
    let hours = quot(seconds, 3_600);
    if hours < 24 {
        return format!("{hours}h");
    }
    let days = quot(seconds, 86_400);
    if days < 7 {
        return format!("{days}d");
    }
    let index = usize::try_from(at.month0()).unwrap_or(0);
    let month = MONTHS.get(index).copied().unwrap_or("Jan");
    format!("{month} {}", at.day())
}

/// `count / unit`, or zero when `unit` is zero.
fn quot(count: i64, unit: i64) -> i64 {
    count.checked_div(unit).unwrap_or(0)
}

/// `en-US` short months, January first (`Date.toLocaleDateString`).
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `count / unit` rounded half up (`Math.round`), for a count of zero or
/// more.
fn nearest(count: i64, unit: i64) -> i64 {
    count
        .saturating_add(unit.div_euclid(2))
        .checked_div_euclid(unit)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Duration, Utc};

    use super::compact_ago;

    fn at(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn compact_age_is_paseos_row() {
        let now = at("2026-01-15T12:00:00Z");
        assert_eq!(compact_ago(now + Duration::seconds(10), now), "now");
        assert_eq!(compact_ago(now - Duration::seconds(59), now), "now");
        assert_eq!(compact_ago(now - Duration::seconds(60), now), "1m");
        assert_eq!(compact_ago(now - Duration::minutes(5), now), "5m");
        assert_eq!(compact_ago(now - Duration::hours(2), now), "2h");
        assert_eq!(compact_ago(now - Duration::days(3), now), "3d");
        assert_eq!(compact_ago(now - Duration::days(6), now), "6d");
        assert_eq!(
            compact_ago(now - (Duration::days(6) + Duration::hours(23)), now),
            "6d"
        );
        assert_eq!(compact_ago(now - Duration::days(7), now), "Jan 8");
    }
}
