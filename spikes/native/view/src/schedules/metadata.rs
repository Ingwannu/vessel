//! Prepare row metadata from authoritative node times and the viewer's
//! calendar. The calendar is supplied by the app so this module stays pure.
use super::{
    next_due,
    time::{Age, Countdown},
};
use crate::localization::{FormatError, Number, Translator};
use chrono::{DateTime, Datelike as _, NaiveDate, Utc};
use vessel_proto::automation::ScheduleInfo;

/// Host-calendar projection, invoked only for dates that outgrew relative copy.
pub type Calendar = fn(DateTime<Utc>) -> Option<NaiveDate>;

pub(super) fn prepare(
    info: &ScheduleInfo,
    host: Option<&str>,
    now: DateTime<Utc>,
    calendar: Calendar,
    catalog: &Translator,
    show_next: bool,
) -> Result<String, FormatError> {
    let words = catalog.schedules();
    let cadence = super::locale::cadence(&info.definition.cadence, catalog)?;
    let age = age(Age::of(info.created_at, now), calendar, catalog)?;
    let created = words.created_age(&age)?;
    let last = match &info.last_run {
        None => words.never_run().to_owned(),
        Some(run) => words.last_run_age(&self::age(Age::of(run.at, now), calendar, catalog)?)?,
    };
    let meta = match next_due(info).filter(|_| show_next) {
        Some(next) => {
            let countdown = Countdown::of(next, now).label(catalog)?;
            words.metadata_next(&cadence, &created, &last, &words.next_run(&countdown)?)?
        }
        None => words.metadata(&cadence, &created, &last)?,
    };
    match host {
        Some(host) => words.metadata_host(host, &meta),
        None => Ok(meta),
    }
}

pub(super) fn age(
    age: Age,
    calendar: Calendar,
    catalog: &Translator,
) -> Result<String, FormatError> {
    let words = catalog.schedules();
    match age {
        Age::Now => Ok(words.age_now().to_owned()),
        Age::Minutes(n) => words.age_minutes(Number::count(n)),
        Age::Hours(n) => words.age_hours(Number::count(n)),
        Age::Days(n) => words.age_days(Number::count(n)),
        Age::Date(at) => {
            let date = calendar(at).unwrap_or_else(|| at.date_naive());
            words.age_date(Number::count(date.day()), Number::count(date.month()))
        }
    }
}
