//! Cached history words and typed links, prepared outside painting.
use super::{
    Outcome, outcome,
    reader::{Reader, SessionLink, Visit},
};
use crate::schedules::{
    clock::{Clock, ClockError},
    page::Body,
    presentation::Notice,
};
use crate::{
    Lang,
    badge::Variant,
    localization::{Number, Translator},
};
use std::sync::Arc;
use vessel_proto::automation::{RunErrorCode, RunNumber, RunOutcome, ScheduleRun, SkipReason};

/// One read-only run row; every time is in the explicit viewer calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// Stable node-authored number, never a list index.
    pub number: RunNumber,
    /// Whole number/time heading, already formatted.
    pub heading: Arc<str>,
    /// Accepted outcome words from the existing table.
    pub outcome: Arc<str>,
    /// Shared badge semantics, selected here rather than by App.
    pub variant: Variant,
    /// Localized typed reason; original private diagnostic stays in Reader.
    pub reason: Option<Arc<str>>,
    /// Original physical destination; absent when no session was created.
    pub link: Option<SessionLink>,
}
/// A comparable display cache; no field/transport work runs during rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// Captured close/refresh/row callbacks retain this history visit.
    pub visit: Visit,
    /// Whole localized dialog heading.
    pub title: Arc<str>,
    /// Distinct loading/empty/failed/content surface.
    pub body: Body,
    /// Current authoritative run rows, no failed/retired cached payload.
    pub runs: Vec<Run>,
    /// One failure explanation, never a second failure badge for that fact.
    pub notice: Option<Notice>,
}
/// Format only a currently open history; full typed formatting errors survive.
/// # Errors
/// Retains viewer/calendar/catalog failures for the existing localized fallback.
pub fn prepare(
    reader: &Reader,
    clock: &Clock,
    catalog: &Translator,
) -> Result<Option<Page>, ClockError> {
    let (Some(visit), Some(body), Some(title)) = (reader.visit(), reader.body(), reader.title())
    else {
        return Ok(None);
    };
    let title = catalog.schedules().history_title(title)?;
    let runs = reader
        .runs()
        .iter()
        .map(|run| prepared_run(reader, visit, run, clock, catalog))
        .collect::<Result<_, _>>()?;
    let notice = reader.failure().map(|failure| Notice {
        label: crate::schedules::locale::failure(failure, catalog).into(),
        variant: failure.variant(),
    });
    Ok(Some(Page {
        visit,
        title: title.into(),
        body,
        runs,
        notice,
    }))
}
fn prepared_run(
    reader: &Reader,
    visit: Visit,
    run: &ScheduleRun,
    clock: &Clock,
    catalog: &Translator,
) -> Result<Run, ClockError> {
    let state = outcome(&run.outcome);
    let heading = catalog
        .schedules()
        .history_run_at(&clock.at(run.at)?, Number::count(run.number.get()))?;
    Ok(Run {
        number: run.number,
        heading: heading.into(),
        outcome: Lang::from_tag(catalog.locale().tag())
            .schedules()
            .outcome(state)
            .into(),
        variant: badge(state),
        reason: reason(&run.outcome, catalog)?.map(Into::into),
        link: reader.session_link(visit, run.number),
    })
}
fn badge(outcome: Outcome) -> Variant {
    match outcome {
        Outcome::Finished | Outcome::Prompted | Outcome::Steered => Variant::Success,
        Outcome::Failed | Outcome::Refused => Variant::Error,
        Outcome::Waiting | Outcome::Limited => Variant::Warning,
        Outcome::Making
        | Outcome::Running
        | Outcome::Queued
        | Outcome::Skipped
        | Outcome::Cancelled
        | Outcome::SessionEnded
        | Outcome::Interrupted
        | Outcome::Unknown => Variant::Muted,
    }
}
fn reason(
    outcome: &RunOutcome,
    catalog: &Translator,
) -> Result<Option<String>, crate::localization::FormatError> {
    let words = catalog.schedules();
    Ok(match outcome {
        RunOutcome::Skipped { reason } => Some(match reason {
            SkipReason::PreviousRunning => words.history_previous_running().to_owned(),
            SkipReason::PreviousWaiting => words.history_previous_waiting().to_owned(),
            SkipReason::Stopped => words.history_stopped().to_owned(),
            SkipReason::Missed { count } => words.history_missed(Number::count(*count))?,
            SkipReason::Unknown => words.history_reason_unknown().to_owned(),
        }),
        RunOutcome::Failed { error, .. } => Some(error_label(error.code, catalog).to_owned()),
        RunOutcome::Making
        | RunOutcome::Started { .. }
        | RunOutcome::Waiting { .. }
        | RunOutcome::Finished { .. }
        | RunOutcome::Prompted { .. }
        | RunOutcome::Interrupted
        | RunOutcome::Unknown => None,
    })
}
fn error_label(code: RunErrorCode, catalog: &Translator) -> &str {
    let words = catalog.schedules();
    match code {
        RunErrorCode::InvalidRequest => words.history_run_invalid(),
        RunErrorCode::Unauthorized => words.history_run_unauthorized(),
        RunErrorCode::Forbidden => words.history_run_forbidden(),
        RunErrorCode::ProtocolMismatch => words.history_run_protocol(),
        RunErrorCode::NotFound => words.history_run_not_found(),
        RunErrorCode::Conflict => words.history_run_conflict(),
        RunErrorCode::Unsupported => words.history_run_unsupported(),
        RunErrorCode::HarnessNotInstalled => words.history_run_agent_missing(),
        RunErrorCode::SpawnFailed => words.history_run_spawn(),
        RunErrorCode::Unavailable => words.history_run_unavailable(),
        RunErrorCode::Internal => words.history_run_internal(),
        RunErrorCode::Unknown => words.history_reason_unknown(),
    }
}
#[cfg(test)]
mod tests;
