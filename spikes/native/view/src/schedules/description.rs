//! Recognise common cron shapes from the protocol's parsed sets. No second
//! grammar: day/month restrictions must be semantically unrestricted before
//! an hourly/daily description can hide the expression.

use chrono::{NaiveTime, Weekday};
use vessel_proto::automation::CronExpr;

/// Human day groups in Paseo's fixed-time descriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Days {
    /// Every date.
    Daily,
    /// Monday through Friday.
    Weekdays,
    /// Saturday and Sunday.
    Weekends,
    /// One weekday.
    On(Weekday),
}

/// Common, unambiguous shape of a parsed rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CronShape {
    /// Every minute, every hour, every date.
    EveryMinute,
    /// One minute in every hour.
    Hourly(NaiveTime),
    /// One time on a recognised group of weekdays.
    Fixed {
        /// Time of day in the rule's zone.
        at: NaiveTime,
        /// Which dates.
        days: Days,
    },
}

const WEEK: [Weekday; 7] = [
    Weekday::Sun,
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
];

/// Read the existing bitsets; return `None` for shapes requiring the original
/// expression. All 31 date values are checked so Vixie's OR rule cannot turn a
/// date-restricted expression into a misleading weekday-only description.
#[must_use]
pub fn cron_shape(expr: &CronExpr) -> Option<CronShape> {
    let fields = expr.fields();
    if !(1..=12).all(|month| fields.month(month)) {
        return None;
    }
    let weekdays: Vec<_> = (0..7).filter(|weekday| fields.day(1, *weekday)).collect();
    if !(1..=31)
        .all(|date| (0..7).all(|weekday| fields.day(date, weekday) == weekdays.contains(&weekday)))
    {
        return None;
    }
    let days = match weekdays.as_slice() {
        [0, 1, 2, 3, 4, 5, 6] => Days::Daily,
        [1, 2, 3, 4, 5] => Days::Weekdays,
        [0, 6] => Days::Weekends,
        [day] => Days::On(*WEEK.get(usize::from(*day))?),
        _other => return None,
    };
    let all_hours = (0..24).all(|hour| fields.hour(hour));
    let all_minutes = (0..60).all(|minute| fields.minute(minute));
    if days == Days::Daily && all_hours && all_minutes {
        return Some(CronShape::EveryMinute);
    }
    let mut minutes = (0..60).filter(|minute| fields.minute(*minute));
    let minute = minutes.next()?;
    if minutes.next().is_some() {
        return None;
    }
    if days == Days::Daily && all_hours {
        return Some(CronShape::Hourly(NaiveTime::from_hms_opt(
            0,
            u32::from(minute),
            0,
        )?));
    }
    let mut hours = (0..24).filter(|hour| fields.hour(*hour));
    let hour = hours.next()?;
    if hours.next().is_some() {
        return None;
    }
    Some(CronShape::Fixed {
        at: NaiveTime::from_hms_opt(u32::from(hour), u32::from(minute), 0)?,
        days,
    })
}
