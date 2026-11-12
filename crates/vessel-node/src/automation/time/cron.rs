//! A cron expression's next time in a zone: walk the matching wall-clock
//! minutes, then turn each into an instant by the zone's rules (the parent
//! module's Decision Log says why this way).

use jiff::Timestamp;
use jiff::civil::{Date, DateTime};
use jiff::tz::{AmbiguousOffset, TimeZone};
use vessel_proto::automation::CronFields;

use super::LOOK_AHEAD_YEARS;

/// The first instant after `after` at which `fields` match a wall-clock
/// minute in `zone`.
pub(super) fn next_after(
    fields: &CronFields,
    zone: &TimeZone,
    after: Timestamp,
) -> Option<Timestamp> {
    let start = zone.to_datetime(after);
    let limit = start.year().checked_add(LOOK_AHEAD_YEARS)?;
    // The minute after the one `after` falls in: cron is per minute.
    let mut civil = WallTime::of(start).next()?;
    loop {
        let found = civil.next_match(fields, limit)?;
        let instant = instant(zone, found.datetime()?)?;
        if instant > after {
            return Some(instant);
        }
        // Not after `after`: the second pass of a fold (its first
        // occurrence came earlier), or a gap that closed at or before it.
        civil = found.next()?;
    }
}

/// The instant a wall-clock minute stands for in `zone`: itself when it
/// exists once; its first occurrence when the clocks went back over it;
/// the instant the clocks jumped when they skipped it.
fn instant(zone: &TimeZone, civil: DateTime) -> Option<Timestamp> {
    match zone.to_ambiguous_timestamp(civil).offset() {
        AmbiguousOffset::Unambiguous { offset } => offset.to_timestamp(civil).ok(),
        // `before` is the offset in force before the clocks went back:
        // with it the minute is its earlier occurrence.
        AmbiguousOffset::Fold { before, .. } => before.to_timestamp(civil).ok(),
        // Read with the offset after the jump, the minute lands before
        // the jump; the zone's next transition from there is the jump.
        AmbiguousOffset::Gap { after, .. } => {
            let early = after.to_timestamp(civil).ok()?;
            zone.following(early).next().map(|jump| jump.timestamp())
        }
    }
}

/// A wall-clock time to the minute: a date, an hour and a minute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WallTime {
    date: Date,
    hour: u8,
    minute: u8,
}

impl WallTime {
    fn of(civil: DateTime) -> Self {
        Self {
            date: civil.date(),
            hour: u8::try_from(civil.hour()).unwrap_or(0),
            minute: u8::try_from(civil.minute()).unwrap_or(0),
        }
    }

    fn datetime(self) -> Option<DateTime> {
        DateTime::new(
            self.date.year(),
            self.date.month(),
            self.date.day(),
            i8::try_from(self.hour).ok()?,
            i8::try_from(self.minute).ok()?,
            0,
            0,
        )
        .ok()
    }

    /// The next minute.
    fn next(self) -> Option<Self> {
        if self.minute < 59 {
            return Some(Self {
                minute: self.minute.checked_add(1)?,
                ..self
            });
        }
        self.next_hour()
    }

    /// The first minute of the next hour.
    fn next_hour(self) -> Option<Self> {
        if self.hour < 23 {
            return Some(Self {
                hour: self.hour.checked_add(1)?,
                minute: 0,
                ..self
            });
        }
        self.next_day()
    }

    /// Midnight of the next day.
    fn next_day(self) -> Option<Self> {
        Some(Self {
            date: self.date.tomorrow().ok()?,
            hour: 0,
            minute: 0,
        })
    }

    /// Midnight of the first day of the next month.
    fn next_month(self) -> Option<Self> {
        let first = self.date.first_of_month();
        let date = if first.month() == 12 {
            Date::new(first.year().checked_add(1)?, 1, 1).ok()?
        } else {
            Date::new(first.year(), first.month().checked_add(1)?, 1).ok()?
        };
        Some(Self {
            date,
            hour: 0,
            minute: 0,
        })
    }

    /// The first minute at or after this one that `fields` match, before
    /// the year `limit` ends. Each step moves to the start of the next
    /// month, day or hour that could match, or finds the minute, so the
    /// walk is at most a few steps per day over the years it may cover.
    fn next_match(self, fields: &CronFields, limit: i16) -> Option<Self> {
        let mut at = self;
        loop {
            if at.date.year() > limit {
                return None;
            }
            let month = u8::try_from(at.date.month()).ok()?;
            if !fields.month(month) {
                at = at.next_month()?;
                continue;
            }
            let day = u8::try_from(at.date.day()).ok()?;
            let weekday = u8::try_from(at.date.weekday().to_sunday_zero_offset()).ok()?;
            if !fields.day(day, weekday) {
                at = at.next_day()?;
                continue;
            }
            let Some(hour) = fields.next_hour(at.hour) else {
                at = at.next_day()?;
                continue;
            };
            if hour != at.hour {
                at = Self {
                    hour,
                    minute: 0,
                    ..at
                };
            }
            match fields.next_minute(at.minute) {
                Some(minute) => return Some(Self { minute, ..at }),
                None => at = at.next_hour()?,
            }
        }
    }
}
