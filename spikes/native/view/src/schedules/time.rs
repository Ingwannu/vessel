//! Pure schedule distances, after Paseo utils/time.ts and schedule-format.ts.
//! The existing global Ago rounds and keeps days forever, so changing it would
//! alter other screens. These reference rules deliberately floor past ages,
//! round future distances, and switch past ages to calendar dates at one week.
use crate::localization::{FormatError, Number, Translator};
use chrono::{DateTime, Utc};
const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;
const WEEK_MS: i64 = 604_800_000;

/// A past distance, before localized wording or viewer calendar conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Age {
    /// Includes future instants, as the reference does.
    Now,
    /// Whole elapsed minutes, rounded down.
    Minutes(u32),
    /// Whole elapsed hours, rounded down.
    Hours(u32),
    /// Whole elapsed days below one week, rounded down.
    Days(u32),
    /// Original instant retained for the viewer's own calendar date.
    Date(DateTime<Utc>),
}
impl Age {
    /// The reference's thresholds, independent of rendering and local zone.
    #[must_use]
    pub fn of(at: DateTime<Utc>, now: DateTime<Utc>) -> Self {
        let elapsed = now.signed_duration_since(at).num_milliseconds();
        if elapsed < MINUTE_MS {
            return Self::Now;
        }
        if elapsed < HOUR_MS {
            return Self::Minutes(count(elapsed, MINUTE_MS));
        }
        if elapsed < DAY_MS {
            return Self::Hours(count(elapsed, HOUR_MS));
        }
        if elapsed < WEEK_MS {
            return Self::Days(count(elapsed, DAY_MS));
        }
        Self::Date(at)
    }
}
fn count(value: i64, divisor: i64) -> u32 {
    // Positive bounded threshold branches above prove these quotients fit.
    u32::try_from(value.checked_div(divisor).unwrap_or_default()).unwrap_or_default()
}
/// A next-run distance, never a locally projected next-run instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Countdown {
    /// Due in less than a minute, or already due on the node's last snapshot.
    Soon,
    /// Reference rounded minutes.
    Minutes(u32),
    /// Reference rounded hours.
    Hours(u32),
    /// Reference rounded days; full chrono range fits u32 days.
    Days(u32),
}
impl Countdown {
    /// Classify only the node-supplied instant, rounding half up like Math.round.
    #[must_use]
    pub fn of(at: DateTime<Utc>, now: DateTime<Utc>) -> Self {
        let ahead = at.signed_duration_since(now).num_milliseconds();
        if ahead < MINUTE_MS {
            return Self::Soon;
        }
        if ahead < HOUR_MS {
            return Self::Minutes(rounded(ahead, MINUTE_MS));
        }
        if ahead < DAY_MS {
            return Self::Hours(rounded(ahead, HOUR_MS));
        }
        Self::Days(rounded(ahead, DAY_MS))
    }
    /// Resolve a whole phrase through the generated language API, off paint.
    /// # Errors
    /// Retains an unexpected translation-engine failure for typed presentation.
    pub fn label(self, catalog: &Translator) -> Result<String, FormatError> {
        let words = catalog.schedules();
        match self {
            Self::Soon => Ok(words.countdown_soon().to_owned()),
            Self::Minutes(n) => words.countdown_minutes(Number::count(n)),
            Self::Hours(n) => words.countdown_hours(Number::count(n)),
            Self::Days(n) => words.countdown_days(Number::count(n)),
        }
    }
}
fn rounded(value: i64, divisor: i64) -> u32 {
    // Saturation keeps this total even at an external timestamp edge. Chrono's
    // entire representable range is far below u32 days.
    count(value.saturating_add(divisor.div_euclid(2)), divisor)
}

/// Reference wake tiers, ordered from fastest to slowest. No tier means an
/// absolute date: it needs no clock task until its data/language changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RefreshTier {
    /// Near ages and active next-run countdowns.
    Minute,
    /// Hour ages wake twice per hour, as the reference ticker does.
    HalfHour,
    /// Day ages wake once per hour.
    Hour,
}
impl RefreshTier {
    const fn milliseconds(self) -> i64 {
        match self {
            Self::Minute => MINUTE_MS,
            Self::HalfHour => HOUR_MS.div_euclid(2),
            Self::Hour => HOUR_MS,
        }
    }
    /// Align all labels to one wall-clock boundary, including before Unix epoch.
    /// Each enum period is positive; checked subtraction keeps the edge total.
    #[must_use]
    pub fn after(self, now: DateTime<Utc>) -> std::time::Duration {
        let period = self.milliseconds();
        let remainder = now.timestamp_millis().rem_euclid(period);
        let delay = period.checked_sub(remainder).unwrap_or(period);
        std::time::Duration::from_millis(delay.unsigned_abs())
    }
}
impl Age {
    /// Same tier as Paseo `describeAge`, without a separate timer per row.
    #[must_use]
    pub const fn refresh_tier(self) -> Option<RefreshTier> {
        match self {
            Self::Now | Self::Minutes(_) => Some(RefreshTier::Minute),
            Self::Hours(_) => Some(RefreshTier::HalfHour),
            Self::Days(_) => Some(RefreshTier::Hour),
            Self::Date(_) => None,
        }
    }
}

#[cfg(test)]
mod tests;
