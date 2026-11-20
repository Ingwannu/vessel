//! The form: typed fields in, a definition or every current error out.
//!
//! A new cron starts in the device zone. The machine zone is the fallback
//! when the device name did not parse. There is no UTC default.

use chrono::{DateTime, Utc};
use vessel_proto::automation::{
    AfterRun, Cadence, CatchUp, CreateSchedule, CronExpr, EverySeconds, MaxRuns, PromptText,
    ScheduleDefinition, ScheduleInfo, ScheduleName, ScheduleTarget, TimeZoneName, UpdateSchedule,
};
use vessel_proto::{ScheduleId, SessionMode, SessionSpec};

use super::fields::{
    AfterChoice, CadenceField, CapField, Draft, ExpiryField, FormJob, FormReview, Save,
    ScheduleFormError, TargetField, UnknownPiece,
};
use super::preview::reusable;
use super::{BlockedAct, SchedulePeer, ScheduleReach};

/// The zone a new cron starts with: the device zone, else the machine zone,
/// else none. No UTC default.
#[must_use]
pub fn new_cron_zone(device: Option<&TimeZoneName>, peer: &SchedulePeer) -> Option<TimeZoneName> {
    if let Some(zone) = device {
        return Some(zone.clone());
    }
    match peer {
        SchedulePeer::Ready(ready) => ready.zone.clone(),
        SchedulePeer::Older => None,
    }
}

/// The zone an existing cron opens on: its stored zone, not the device's.
#[must_use]
pub fn opened_zone(info: &ScheduleInfo) -> Option<TimeZoneName> {
    match &info.definition.cadence {
        Cadence::Cron { zone, .. } => Some(zone.clone()),
        Cadence::Every { .. } | Cadence::Unknown => None,
    }
}

/// Validate and, when nothing is wrong, build the save. `preview` is reused
/// only when it is this cadence with the anchor filled in.
#[must_use]
pub fn review(
    job: &FormJob,
    draft: &Draft,
    peer: &SchedulePeer,
    preview: Option<&vessel_proto::automation::SchedulePreview>,
) -> FormReview {
    let mut errors = Vec::new();
    match peer {
        SchedulePeer::Older => errors.push(ScheduleFormError::OlderPeer),
        SchedulePeer::Ready(ready) => match ready.reach {
            ScheduleReach::Forwarded => errors.push(ScheduleFormError::Reach(match job {
                FormJob::Create => BlockedAct::Create,
                FormJob::Edit(_) => BlockedAct::Update,
            })),
            ScheduleReach::Direct => {}
        },
    }
    if let FormJob::Edit(info) = job {
        if matches!(info.defined_by, vessel_proto::Creator::Agent { .. }) {
            errors.push(ScheduleFormError::Owner);
        }
        match info.state {
            vessel_proto::automation::ScheduleState::Active { .. }
            | vessel_proto::automation::ScheduleState::Paused { .. } => {}
            vessel_proto::automation::ScheduleState::Ended { .. }
            | vessel_proto::automation::ScheduleState::Unknown => {
                errors.push(ScheduleFormError::NotRunning);
            }
        }
        push_unknown(info, &mut errors);
    }
    let name = read_name(draft, &mut errors);
    let prompt = read_prompt(draft, &mut errors);
    let cadence = read_cadence(draft, preview, &mut errors);
    let max_runs = read_cap(draft, &mut errors);
    let ends_at = read_expiry(draft, &mut errors);
    let target = read_target(job, draft, &mut errors);
    if !errors.is_empty() {
        return FormReview { errors, save: None };
    }
    let (Some(prompt), Some(cadence), Some(target)) = (prompt, cadence, target) else {
        return FormReview { errors, save: None };
    };
    let definition = ScheduleDefinition {
        name,
        cadence,
        target,
        prompt,
        catch_up: catch_up_of(job),
        max_runs,
        ends_at,
    };
    let save = match job {
        FormJob::Create => Save::Create(CreateSchedule {
            definition,
            paused: false,
        }),
        FormJob::Edit(info) => Save::Update(UpdateSchedule {
            schedule: info.id,
            revision: info.revision,
            definition,
        }),
    };
    FormReview {
        errors,
        save: Some(save),
    }
}

/// `schedule.run` for a direct, running schedule. A forwarded peer and a
/// heartbeat do not produce a request.
///
/// # Errors
/// The peer is older than protocol 11, the peer is forwarded, the schedule
/// is a heartbeat, or it is ended or unknown.
#[must_use]
pub fn run_request(
    info: &ScheduleInfo,
    peer: &SchedulePeer,
) -> Result<ScheduleId, ScheduleFormError> {
    match peer {
        SchedulePeer::Older => return Err(ScheduleFormError::OlderPeer),
        SchedulePeer::Ready(ready) => match ready.reach {
            ScheduleReach::Forwarded => return Err(ScheduleFormError::Reach(BlockedAct::Run)),
            ScheduleReach::Direct => {}
        },
    }
    if super::kind(info) == super::ScheduleKind::Heartbeat {
        return Err(ScheduleFormError::Owner);
    }
    match info.state {
        vessel_proto::automation::ScheduleState::Active { .. }
        | vessel_proto::automation::ScheduleState::Paused { .. } => Ok(info.id),
        vessel_proto::automation::ScheduleState::Ended { .. }
        | vessel_proto::automation::ScheduleState::Unknown => Err(ScheduleFormError::NotRunning),
    }
}

fn catch_up_of(job: &FormJob) -> CatchUp {
    match job {
        FormJob::Create => CatchUp::Once,
        FormJob::Edit(info) => info.definition.catch_up,
    }
}

fn push_unknown(info: &ScheduleInfo, errors: &mut Vec<ScheduleFormError>) {
    if matches!(info.definition.cadence, Cadence::Unknown) {
        errors.push(ScheduleFormError::UnknownDefinition(UnknownPiece::Cadence));
    }
    if matches!(info.definition.target, ScheduleTarget::Unknown) {
        errors.push(ScheduleFormError::UnknownDefinition(UnknownPiece::Target));
    }
    if matches!(info.definition.catch_up, CatchUp::Unknown) {
        errors.push(ScheduleFormError::UnknownDefinition(UnknownPiece::CatchUp));
    }
    if let ScheduleTarget::NewSession { after, .. } = &info.definition.target
        && matches!(after, AfterRun::Unknown)
    {
        errors.push(ScheduleFormError::UnknownDefinition(UnknownPiece::AfterRun));
    }
}

fn read_name(draft: &Draft, errors: &mut Vec<ScheduleFormError>) -> Option<ScheduleName> {
    if draft.name.trim().is_empty() {
        return None;
    }
    match draft.name.parse() {
        Ok(name) => Some(name),
        Err(error) => {
            errors.push(ScheduleFormError::Name(error));
            None
        }
    }
}

fn read_prompt(draft: &Draft, errors: &mut Vec<ScheduleFormError>) -> Option<PromptText> {
    match draft.prompt.parse() {
        Ok(prompt) => Some(prompt),
        Err(error) => {
            errors.push(ScheduleFormError::Prompt(error));
            None
        }
    }
}

fn read_cadence(
    draft: &Draft,
    preview: Option<&vessel_proto::automation::SchedulePreview>,
    errors: &mut Vec<ScheduleFormError>,
) -> Option<Cadence> {
    let built = match &draft.cadence {
        CadenceField::Preset(preset) => {
            cron_cadence(preset.expression(), draft.zone.clone(), errors)
        }
        CadenceField::Cron(text) => cron_cadence(text, draft.zone.clone(), errors),
        CadenceField::Every(seconds) => match EverySeconds::try_from(*seconds) {
            Ok(seconds) => Some(Cadence::Every {
                seconds,
                anchor: None,
            }),
            Err(error) => {
                errors.push(ScheduleFormError::Interval(error));
                None
            }
        },
    };
    match (built, preview) {
        (Some(cadence), Some(preview)) => {
            let reused = reusable(&cadence, preview).cloned();
            Some(reused.unwrap_or(cadence))
        }
        (built, _) => built,
    }
}

fn cron_cadence(
    text: &str,
    zone: Option<TimeZoneName>,
    errors: &mut Vec<ScheduleFormError>,
) -> Option<Cadence> {
    let expr = match text.parse::<CronExpr>() {
        Ok(expr) => Some(expr),
        Err(error) => {
            errors.push(ScheduleFormError::Cron(error));
            None
        }
    };
    let Some(zone) = zone else {
        errors.push(ScheduleFormError::Zone);
        return None;
    };
    Some(Cadence::Cron { expr: expr?, zone })
}

fn read_cap(draft: &Draft, errors: &mut Vec<ScheduleFormError>) -> Option<MaxRuns> {
    let CapField::Text(text) = &draft.max_runs else {
        return None;
    };
    if text.trim().is_empty() {
        return None;
    }
    let Ok(parsed) = text.trim().parse::<u32>() else {
        errors.push(ScheduleFormError::Cap);
        return None;
    };
    if parsed == 0 {
        errors.push(ScheduleFormError::ZeroCap);
        return None;
    }
    match MaxRuns::try_from(parsed) {
        Ok(cap) => Some(cap),
        Err(_error) => {
            errors.push(ScheduleFormError::Cap);
            None
        }
    }
}

fn read_expiry(draft: &Draft, errors: &mut Vec<ScheduleFormError>) -> Option<DateTime<Utc>> {
    match draft.ends_at {
        ExpiryField::None => None,
        ExpiryField::At(at) => Some(at),
        ExpiryField::Invalid => {
            errors.push(ScheduleFormError::Expiry);
            None
        }
    }
}

fn read_target(
    job: &FormJob,
    draft: &Draft,
    errors: &mut Vec<ScheduleFormError>,
) -> Option<ScheduleTarget> {
    match &draft.target {
        TargetField::Session(session) => {
            if let FormJob::Edit(info) = job {
                match &info.definition.target {
                    ScheduleTarget::Session { session: loaded } if loaded == session => {}
                    ScheduleTarget::Session { .. }
                    | ScheduleTarget::NewSession { .. }
                    | ScheduleTarget::Unknown => {
                        errors.push(ScheduleFormError::Target);
                        return None;
                    }
                }
            }
            Some(ScheduleTarget::Session { session: *session })
        }
        TargetField::NewSession(spec) => {
            if let FormJob::Edit(info) = job
                && !matches!(info.definition.target, ScheduleTarget::NewSession { .. })
            {
                errors.push(ScheduleFormError::Target);
                return None;
            }
            check_spec(spec, errors);
            if errors.iter().any(spec_error) {
                return None;
            }
            Some(ScheduleTarget::NewSession {
                spec: Box::new(spec.clone()),
                after: match draft.after {
                    AfterChoice::Archive => AfterRun::Archive,
                    AfterChoice::Keep => AfterRun::Keep,
                },
            })
        }
    }
}

fn spec_error(error: &ScheduleFormError) -> bool {
    matches!(
        error,
        ScheduleFormError::Place | ScheduleFormError::Terminal | ScheduleFormError::Resume
    )
}

fn check_spec(spec: &SessionSpec, errors: &mut Vec<ScheduleFormError>) {
    if spec.place.is_none() {
        errors.push(ScheduleFormError::Place);
    }
    match &spec.mode {
        SessionMode::Terminal { .. } => errors.push(ScheduleFormError::Terminal),
        SessionMode::Structured {
            resume: Some(_), ..
        } => errors.push(ScheduleFormError::Resume),
        SessionMode::Structured { resume: None, .. } => {}
    }
}
