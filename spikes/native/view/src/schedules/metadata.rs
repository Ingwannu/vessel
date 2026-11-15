//! Whole row metadata is prepared when its inputs change, never while painting.
//! The node supplies every state and `next_due`; this module formats them only.
use super::{
    clock::{Clock, ClockError},
    history, next_due,
    time::{Age, Countdown},
};
use crate::{Lang, localization::Translator};
use chrono::{DateTime, Utc};
use vessel_proto::automation::{Cadence, ScheduleInfo};

/// Cadence → creation → result → next run, matching the reference row's order.
/// # Errors
/// Calendar/catalog failures remain typed for a localized row fallback.
pub fn prepare(
    info: &ScheduleInfo,
    host: Option<&str>,
    now: DateTime<Utc>,
    clock: &Clock,
    catalog: &Translator,
) -> Result<String, ClockError> {
    let lang = Lang::from_tag(catalog.locale().tag());
    let words = catalog.schedules();
    let cadence = lang.schedule_cadence(&info.definition.cadence);
    let age = clock.age(Age::of(info.created_at, now), catalog)?;
    let created = words.created_age(&age)?;
    let last = match &info.last_run {
        None => words.never_run().to_owned(),
        Some(run) => {
            let age = clock.age(Age::of(run.at, now), catalog)?;
            words.last_result_age(
                &age,
                lang.schedules().outcome(history::outcome(&run.outcome)),
            )?
        }
    };
    let rule_zone = match &info.definition.cadence {
        Cadence::Cron { zone, .. } => Some(zone),
        Cadence::Every { .. } | Cadence::Unknown => None,
    };
    let different = rule_zone.filter(|zone| clock.name() != Some(*zone));
    let meta = match next_due(info) {
        Some(next) => {
            let at = clock.at(next)?;
            let countdown = Countdown::of(next, now).label(catalog)?;
            let next = match (clock.is_known(), different) {
                (true, None) => words.next_known(&at, &countdown)?,
                (false, None) => words.next_utc(&at, &countdown)?,
                (true, Some(zone)) => words.next_known_rule(&at, &countdown, zone.as_str())?,
                (false, Some(zone)) => words.next_utc_rule(&at, &countdown, zone.as_str())?,
            };
            words.metadata_next(&cadence, &created, &last, &next)?
        }
        None => match rule_zone {
            Some(zone) => words.metadata_rule(&cadence, &created, &last, zone.as_str())?,
            None => words.metadata(&cadence, &created, &last)?,
        },
    };
    Ok(match host {
        Some(host) => words.metadata_host(host, &meta)?,
        None => meta,
    })
}
