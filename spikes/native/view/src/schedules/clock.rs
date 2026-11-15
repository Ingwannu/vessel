//! Calendar conversion for display only. Node `state/next_due` are never projected
//! here. Bundled zone rules match the node's pinned Jiff; system discovery stays
//! in Link, outside this I/O-free model and outside painting.
use super::time::Age;
use crate::localization::{Date, FormatError, Number, Translator};
use chrono::{DateTime, Datelike as _, NaiveDate, NaiveTime, Utc};
use jiff::{Timestamp, tz::TimeZone};
use vessel_proto::automation::TimeZoneName;

/// Viewer calendar identity, explicitly unknown rather than a guessed offset.
#[derive(Debug, Clone)]
pub struct Clock {
    name: Option<TimeZoneName>,
    zone: TimeZone,
}
/// A typed display failure. Display remains diagnostic, never screen copy.
#[derive(Debug, thiserror::Error)]
pub enum ClockError {
    /// A validated IANA identity is absent from the bundled zone rules.
    #[error("display time zone: {0}")]
    Zone(#[source] jiff::Error),
    /// Chrono can represent instants outside Jiff's civil calendar bounds.
    #[error("display instant: {0}")]
    Instant(#[source] jiff::Error),
    /// A civil component unexpectedly cannot become a Gregorian argument.
    #[error("display calendar component: {0}")]
    Part(#[source] std::num::TryFromIntError),
    /// Date parts cannot be represented by the display calendar.
    #[error("display calendar range")]
    Calendar,
    /// Full formatting failure remains typed until its visible fallback.
    #[error("display translation: {0}")]
    Format(#[from] FormatError),
}
impl ClockError {
    /// Complete localized error, separate from diagnostic Display.
    #[must_use]
    pub fn label<'a>(&self, catalog: &'a Translator) -> &'a str {
        let words = catalog.schedules();
        match self {
            Self::Zone(_) => words.clock_zone(),
            Self::Instant(_) | Self::Calendar | Self::Part(_) => words.clock_range(),
            Self::Format(_) => words.display_unavailable(),
        }
    }
}
impl Clock {
    /// Resolve an already validated viewer identity using bundled data only.
    /// # Errors
    /// Unknown bundled identity is retained; it never guesses an offset.
    pub fn known(name: TimeZoneName) -> Result<Self, ClockError> {
        let zone = TimeZone::get(name.as_str()).map_err(ClockError::Zone)?;
        Ok(Self {
            name: Some(name),
            zone,
        })
    }
    /// Explicit UTC fallback while the platform's own zone is unavailable.
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            name: None,
            zone: TimeZone::UTC,
        }
    }
    /// Identity for labeling differences from a cron rule's own zone.
    #[must_use]
    pub fn name(&self) -> Option<&TimeZoneName> {
        self.name.as_ref()
    }
    /// Preserve unknown-viewer evidence even though its calendar uses UTC.
    #[must_use]
    pub const fn is_known(&self) -> bool {
        self.name.is_some()
    }
    /// A viewer-local date/time at minute precision, for typed Fluent DATETIME.
    /// # Errors
    /// Out-of-range instants remain an explicit display failure.
    pub fn at(&self, at: DateTime<Utc>) -> Result<Date, ClockError> {
        let timestamp = Timestamp::new(at.timestamp(), 0).map_err(ClockError::Instant)?;
        let local = timestamp.to_zoned(self.zone.clone());
        let month = u32::try_from(local.month()).map_err(ClockError::Part)?;
        let day = u32::try_from(local.day()).map_err(ClockError::Part)?;
        let hour = u32::try_from(local.hour()).map_err(ClockError::Part)?;
        let minute = u32::try_from(local.minute()).map_err(ClockError::Part)?;
        let date = NaiveDate::from_ymd_opt(i32::from(local.year()), month, day)
            .ok_or(ClockError::Calendar)?;
        let time = NaiveTime::from_hms_opt(hour, minute, 0).ok_or(ClockError::Calendar)?;
        Ok(Date::DateTime {
            date,
            time,
            zone: self
                .name
                .as_ref()
                .map_or("UTC", TimeZoneName::as_str)
                .to_owned(),
        })
    }
    /// The reference's whole age phrase, with old dates in the viewer calendar.
    /// # Errors
    /// Retains calendar and translation errors; no raw error is shown.
    pub fn age(&self, age: Age, catalog: &Translator) -> Result<String, ClockError> {
        let words = catalog.schedules();
        Ok(match age {
            Age::Now => words.age_now().to_owned(),
            Age::Minutes(n) => words.age_minutes(Number::count(n))?,
            Age::Hours(n) => words.age_hours(Number::count(n))?,
            Age::Days(n) => words.age_days(Number::count(n))?,
            Age::Date(at) => {
                let Date::DateTime { date, .. } = self.at(at)? else {
                    return Err(ClockError::Calendar);
                };
                let day = Number::count(date.day());
                let month = Number::count(date.month());
                if self.is_known() {
                    words.age_date(day, month)?
                } else {
                    words.age_date_utc(day, month)?
                }
            }
        })
    }
}
#[cfg(test)]
mod tests;
