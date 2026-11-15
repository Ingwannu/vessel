//! What a schedule's state becomes, as functions of it alone (ADR 0033):
//! its next state after a due time, whether it may still change, its next
//! run's number and its name in sessions. Must not: read a clock or the
//! store (the actor passes `now` in).

use chrono::{DateTime, Utc};
use vessel_proto::Creator;
use vessel_proto::automation::{
    EndReason, RunNumber, ScheduleDefinition, ScheduleInfo, ScheduleState,
};

use crate::automation::error::AutomationError;

/// Whether `info` is an agent's schedule that has ended, for whatever
/// reason: it goes, so it neither counts toward the node's bound nor
/// outlives what it was for (ADR 0033).
pub(super) fn ends_an_agents(info: &ScheduleInfo) -> bool {
    matches!(info.state, ScheduleState::Ended { .. })
        && matches!(info.defined_by, Creator::Agent { .. })
}

/// Ended schedules cannot pause/resume/run. Definition edits separately admit
/// known Ended without reactivation (pinned Paseo schedule/service.ts458–509).
pub(super) fn ended_refused(info: &ScheduleInfo) -> Result<(), AutomationError> {
    match info.state {
        ScheduleState::Ended { .. } | ScheduleState::Unknown => Err(AutomationError::Conflict(
            "the schedule has ended".to_owned(),
        )),
        ScheduleState::Active { .. } | ScheduleState::Paused { .. } => Ok(()),
    }
}

/// Definition updates preserve a known Ended record, not a newer unknown state.
/// Pinned Paseo schedule/service.ts458–509 keeps completed status on update.
pub(super) fn update_admitted(info: &ScheduleInfo) -> Result<(), AutomationError> {
    match info.state {
        ScheduleState::Active { .. } | ScheduleState::Paused { .. } => Ok(()),
        ScheduleState::Ended { reason, .. } => match reason {
            EndReason::MaxRuns
            | EndReason::Expired
            | EndReason::CreatorGone
            | EndReason::TargetGone => Ok(()),
            EndReason::Unknown => Err(AutomationError::Conflict(
                "the schedule has an unknown end reason".to_owned(),
            )),
        },
        ScheduleState::Unknown => Err(AutomationError::Conflict(
            "the schedule has an unknown state".to_owned(),
        )),
    }
}

/// Active at `next`, or ended `expired` when there is none or it is past
/// `ends_at`.
pub(super) fn advanced(
    definition: &ScheduleDefinition,
    next: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> ScheduleState {
    match next {
        Some(next) if definition.ends_at.is_none_or(|ends_at| next <= ends_at) => {
            ScheduleState::Active { next_due: next }
        }
        Some(_) | None => ScheduleState::Ended {
            at: now,
            reason: EndReason::Expired,
        },
    }
}

/// The number the schedule's next run takes.
pub(super) fn next_number(info: &ScheduleInfo) -> RunNumber {
    info.last_run
        .as_ref()
        .and_then(|run| run.number.next())
        .unwrap_or(RunNumber::FIRST)
}

/// What a schedule is called in its sessions: its name, or its id.
pub(super) fn name_of(info: &ScheduleInfo) -> String {
    info.definition
        .name
        .as_ref()
        .map_or_else(|| info.id.to_string(), ToString::to_string)
}
