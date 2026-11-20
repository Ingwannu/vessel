//! A detail's runs, and the archive switch as one whole update.
//!
//! The switch is `schedule.update` of the definition with that one field
//! changed, so `defined_by` becomes the person who toggles it. The request
//! has no field that keeps the previous owner (ADR 0072).

use vessel_proto::automation::{
    AfterRun, Cadence, CatchUp, RunEnd, RunError, RunNumber, RunOutcome, RunTrigger, ScheduleInfo,
    ScheduleRun, ScheduleState, ScheduleTarget, SkipReason, UpdateSchedule,
};
use vessel_proto::{Creator, Delivered, SessionId};

use super::fields::{ScheduleFormError, UnknownPiece};
use super::{BlockedAct, SchedulePeer, ScheduleReach};

/// How many due times one caught-up run stands for. The node counted them;
/// this only stops the count being a bare `u32` on the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissedRuns(u32);

impl MissedRuns {
    /// The node's count.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// What made the run, one arm per `RunTrigger`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShownTrigger {
    /// Its due time came.
    Due,
    /// One run standing for missed due times.
    CaughtUp(MissedRuns),
    /// Someone asked.
    Asked(Creator),
    /// A trigger this app does not know.
    Unknown,
}

/// What the run did, one arm per `RunOutcome`. There is no output text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShownOutcome {
    /// Recorded, not sent yet.
    Making,
    /// A new session's first turn is under way.
    Started(SessionId),
    /// That turn waits on a person.
    Waiting(SessionId),
    /// That turn or the session ended.
    Finished {
        /// The session.
        session: SessionId,
        /// How.
        end: RunEnd,
    },
    /// The prompt went into an existing session's queue.
    Prompted {
        /// The session.
        session: SessionId,
        /// Started a turn, or waits.
        delivered: Delivered,
    },
    /// Nothing was sent.
    Skipped(SkipReason),
    /// The call was refused.
    Failed {
        /// The refusal.
        error: RunError,
        /// The session, if one had been started.
        session: Option<SessionId>,
    },
    /// The node stopped while it was being made.
    Interrupted,
    /// An outcome this app does not know.
    Unknown,
}

/// One run, as the detail lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRow {
    /// The run number.
    pub number: RunNumber,
    /// The due time it was for.
    pub due_at: chrono::DateTime<chrono::Utc>,
    /// When it ran.
    pub at: chrono::DateTime<chrono::Utc>,
    /// What made it run.
    pub trigger: ShownTrigger,
    /// What it did.
    pub outcome: ShownOutcome,
}

/// The switch's result. Unchanged sends nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveOutcome {
    /// A whole `schedule.update`. Later runs act as the person who sends it.
    Update(UpdateSchedule),
    /// The schedule already has this after-run.
    Unchanged,
    /// Why no update was built.
    Refused(Vec<ScheduleFormError>),
}

/// Map one run. Every trigger and outcome has an arm.
#[must_use]
pub fn run_row(run: &ScheduleRun) -> RunRow {
    RunRow {
        number: run.number,
        due_at: run.due_at,
        at: run.at,
        trigger: shown_trigger(&run.trigger),
        outcome: shown_outcome(&run.outcome),
    }
}

/// The archive switch as an update, or a refusal. The definition is copied
/// whole; only `after` changes. `UpdateSchedule` cannot name the old owner.
#[must_use]
pub fn archive(
    info: &ScheduleInfo,
    peer: &SchedulePeer,
    choice: super::AfterChoice,
) -> ArchiveOutcome {
    let mut errors = Vec::new();
    refuse_peer(peer, BlockedAct::Update, &mut errors);
    refuse_owner(info, &mut errors);
    refuse_state(info, &mut errors);
    refuse_unknown(info, &mut errors);
    let Some(current) = new_session_after(info, &mut errors) else {
        return ArchiveOutcome::Refused(errors);
    };
    let chosen = match choice {
        super::AfterChoice::Archive => AfterRun::Archive,
        super::AfterChoice::Keep => AfterRun::Keep,
    };
    if !errors.is_empty() {
        return ArchiveOutcome::Refused(errors);
    }
    if current == chosen {
        return ArchiveOutcome::Unchanged;
    }
    let mut definition = info.definition.clone();
    definition.target = match definition.target {
        ScheduleTarget::NewSession { spec, .. } => ScheduleTarget::NewSession {
            spec,
            after: chosen,
        },
        ScheduleTarget::Session { session } => ScheduleTarget::Session { session },
        ScheduleTarget::Unknown => ScheduleTarget::Unknown,
    };
    ArchiveOutcome::Update(UpdateSchedule {
        schedule: info.id,
        revision: info.revision,
        definition,
    })
}

fn shown_trigger(trigger: &RunTrigger) -> ShownTrigger {
    match trigger {
        RunTrigger::Due => ShownTrigger::Due,
        RunTrigger::CaughtUp { missed } => ShownTrigger::CaughtUp(MissedRuns(*missed)),
        RunTrigger::Asked { by } => ShownTrigger::Asked(by.clone()),
        RunTrigger::Unknown => ShownTrigger::Unknown,
    }
}

fn shown_outcome(outcome: &RunOutcome) -> ShownOutcome {
    match outcome {
        RunOutcome::Making => ShownOutcome::Making,
        RunOutcome::Started { session } => ShownOutcome::Started(*session),
        RunOutcome::Waiting { session } => ShownOutcome::Waiting(*session),
        RunOutcome::Finished { session, end } => ShownOutcome::Finished {
            session: *session,
            end: end.clone(),
        },
        RunOutcome::Prompted { session, delivered } => ShownOutcome::Prompted {
            session: *session,
            delivered: *delivered,
        },
        RunOutcome::Skipped { reason } => ShownOutcome::Skipped(reason.clone()),
        RunOutcome::Failed { error, session } => ShownOutcome::Failed {
            error: error.clone(),
            session: *session,
        },
        RunOutcome::Interrupted => ShownOutcome::Interrupted,
        RunOutcome::Unknown => ShownOutcome::Unknown,
    }
}

fn new_session_after(info: &ScheduleInfo, errors: &mut Vec<ScheduleFormError>) -> Option<AfterRun> {
    match &info.definition.target {
        ScheduleTarget::NewSession { after, .. } => match after {
            AfterRun::Unknown => {
                errors.push(ScheduleFormError::UnknownDefinition(UnknownPiece::AfterRun));
                None
            }
            AfterRun::Archive | AfterRun::Keep => Some(*after),
        },
        ScheduleTarget::Session { .. } => {
            errors.push(ScheduleFormError::NotNewSession);
            None
        }
        ScheduleTarget::Unknown => {
            errors.push(ScheduleFormError::UnknownDefinition(UnknownPiece::Target));
            None
        }
    }
}

fn refuse_peer(peer: &SchedulePeer, act: BlockedAct, errors: &mut Vec<ScheduleFormError>) {
    match peer {
        SchedulePeer::Older => errors.push(ScheduleFormError::OlderPeer),
        SchedulePeer::Ready(ready) => match ready.reach {
            ScheduleReach::Forwarded => errors.push(ScheduleFormError::Reach(act)),
            ScheduleReach::Direct => {}
        },
    }
}

fn refuse_owner(info: &ScheduleInfo, errors: &mut Vec<ScheduleFormError>) {
    if matches!(info.defined_by, Creator::Agent { .. }) {
        errors.push(ScheduleFormError::Owner);
    }
}

fn refuse_state(info: &ScheduleInfo, errors: &mut Vec<ScheduleFormError>) {
    match info.state {
        ScheduleState::Active { .. } | ScheduleState::Paused { .. } => {}
        ScheduleState::Ended { .. } | ScheduleState::Unknown => {
            errors.push(ScheduleFormError::NotRunning);
        }
    }
}

fn refuse_unknown(info: &ScheduleInfo, errors: &mut Vec<ScheduleFormError>) {
    if matches!(info.definition.cadence, Cadence::Unknown) {
        errors.push(ScheduleFormError::UnknownDefinition(UnknownPiece::Cadence));
    }
    if matches!(info.definition.catch_up, CatchUp::Unknown) {
        errors.push(ScheduleFormError::UnknownDefinition(UnknownPiece::CatchUp));
    }
}
