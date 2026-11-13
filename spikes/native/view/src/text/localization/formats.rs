//! [Decision Log]
//! - Purpose: folder-owned numeric rules and whole date/time patterns without
//!   ICU data, regeneration tools, calendar code or a second language table.
//! - Choice: Fluent's NUMBER stays numeric for plural matching; its formatter
//!   hook changes display. DATETIME feeds typed Gregorian/local-clock parts to
//!   a separate pattern bundle. It cannot recursively call DATETIME.
//! - Costs: dynamic values allocate. Callers resolve zones before this boundary;
//!   this is not calendar conversion, time-zone arithmetic or complete Intl.
use super::{
    Date, number,
    runtime::{Bundle, InitError, format_one},
};
use chrono::{Datelike as _, Timelike as _};
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};

pub(super) fn install(
    bundle: &mut Bundle,
    tag: &str,
    sources: &[&str],
    pseudo: bool,
) -> Result<(), InitError> {
    bundle.set_formatter(Some(number::display));
    let translated = patterns(tag, sources, pseudo)?;
    let english_sources = super::EMBEDDED
        .get(super::ENGLISH.index)
        .ok_or(InitError::MissingLocale)?
        .sources;
    let english = patterns("en", english_sources, pseudo)?;
    let cycle = number::rules(tag).ok_or(InitError::FormatRules)?.hour_cycle;
    let english_cycle = number::rules("en")
        .ok_or(InitError::FormatRules)?
        .hour_cycle;
    bundle
        .add_function("DATETIME", move |positional, _named| {
            let Some(FluentValue::String(value)) = positional.first() else {
                return FluentValue::Error;
            };
            let Ok(value) = serde_json::from_str::<Date>(value) else {
                return FluentValue::Error;
            };
            let (key, args) = parts(&value, cycle);
            match format_one(&translated, key, Some(&args)).or_else(|_| {
                // A missing whole time pattern uses English's cycle as well as its
                // words. A partial folder must not leak its hour convention into it.
                let (_, args) = parts(&value, english_cycle);
                format_one(&english, key, Some(&args))
            }) {
                Ok(value) => FluentValue::from(value),
                Err(_) => FluentValue::Error,
            }
        })
        .map_err(|error| InitError::Bundle(vec![error]))?;
    Ok(())
}

fn patterns(tag: &str, sources: &[&str], pseudo: bool) -> Result<Bundle, InitError> {
    let language = tag
        .parse()
        .map_err(|e| InitError::Locale(std::sync::Arc::new(e)))?;
    let mut bundle = Bundle::new_concurrent(vec![language]);
    bundle
        .add_function("NUMBER", number::fluent)
        .map_err(|e| InitError::Bundle(vec![e]))?;
    bundle.set_formatter(Some(number::display));
    bundle.set_use_isolating(false);
    if pseudo {
        bundle.set_transform(Some(super::runtime::pseudo));
    }
    // Resources retain their own references, including month/weekday names.
    // No DATETIME callback is installed here; build validation rejects it in
    // formatting patterns, preventing recursion through another message.
    for source in sources {
        let resource =
            FluentResource::try_new((*source).to_owned()).map_err(|(_, e)| InitError::Parse(e))?;
        bundle.add_resource(resource).map_err(InitError::Bundle)?;
    }
    Ok(bundle)
}

fn parts<'a>(value: &'a Date, cycle: &'a str) -> (&'static str, FluentArgs<'a>) {
    let mut args = FluentArgs::new();
    let (key, date, time) = match value {
        Date::Day(date) => ("formats-date", Some(date), None),
        Date::Time { time, zone } => ("formats-time", None, Some((time, zone))),
        Date::DateTime { date, time, zone } => {
            ("formats-date-time", Some(date), Some((time, zone)))
        }
    };
    if let Some(date) = date {
        args.set("year", date.year());
        args.set("month", date.month());
        args.set("day", date.day());
        args.set("weekday", date.weekday().number_from_monday());
    }
    if let Some((time, zone)) = time {
        args.set("hour-cycle", cycle);
        args.set("hour", time.hour());
        args.set(
            "hour-twelve",
            match time.hour() % 12 {
                0 => 12,
                hour => hour,
            },
        );
        args.set("minute", time.minute());
        args.set("period", if time.hour() < 12 { "am" } else { "pm" });
        args.set("zone", zone.as_str());
    }
    (key, args)
}
