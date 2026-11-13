//! A run's own task (ADR 0033): makes the run through the runner once the
//! scheduler has given it room (`room`), sends its outcome back to the
//! scheduler, then follows a session it started and says when that
//! session's first turn no longer holds room. Must not: write the store
//! (the scheduler records every outcome it sends back), nor wait for room
//! itself (the scheduler orders who goes first).

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use vessel_proto::ScheduleId;
use vessel_proto::automation::{
    EndReason, RunError, RunErrorCode, RunNumber, RunOutcome, ScheduleRun,
};

use super::{FollowSender, Job, Runner};
use crate::automation::error::AutomationError;

/// A run's outcome, back from its task (or, never made, from the
/// scheduler's own wait).
#[derive(Debug)]
pub(super) struct MadeRun {
    pub(super) schedule: ScheduleId,
    pub(super) run: ScheduleRun,
    pub(super) end: Option<EndReason>,
    pub(super) reply: Option<oneshot::Sender<Result<ScheduleRun, AutomationError>>>,
}

/// What a run's task sends back.
#[derive(Debug)]
pub(super) enum Back {
    /// The run has its outcome: its slot is free.
    Made(MadeRun),
    /// The session the run started is no longer followed (its first turn
    /// ended, or the session did): the first turn it held is free.
    TurnOver {
        schedule: ScheduleId,
        number: RunNumber,
    },
}

/// What `schedule.run` answers: the run, or `unavailable` when it never
/// had room (the contract's answer; the run is recorded `failed` all the
/// same).
pub(super) fn answer(run: ScheduleRun) -> Result<ScheduleRun, AutomationError> {
    match &run.outcome {
        RunOutcome::Failed {
            error,
            session: None,
        } if error.code == RunErrorCode::Unavailable && error.message == NO_ROOM => {
            Err(AutomationError::Unavailable(NO_ROOM.to_owned()))
        }
        RunOutcome::Making
        | RunOutcome::Started { .. }
        | RunOutcome::Waiting { .. }
        | RunOutcome::Finished { .. }
        | RunOutcome::Prompted { .. }
        | RunOutcome::Skipped { .. }
        | RunOutcome::Failed { .. }
        | RunOutcome::Interrupted
        | RunOutcome::Unknown => Ok(run),
    }
}

/// Why an asked run was not made.
const NO_ROOM: &str = "no room came free in time: the node is making four runs, or eight scheduled sessions are in \
     their first turn";

/// The outcome of an asked run that waited for room past its deadline.
pub(super) fn no_room() -> RunOutcome {
    RunOutcome::Failed {
        error: RunError {
            code: RunErrorCode::Unavailable,
            message: NO_ROOM.to_owned(),
        },
        session: None,
    }
}

/// What a run's task needs.
pub(super) struct RunTask<R> {
    pub(super) runner: Arc<R>,
    pub(super) back: mpsc::Sender<Back>,
    pub(super) follow: FollowSender,
    pub(super) stop: CancellationToken,
}

impl<R: Runner> RunTask<R> {
    /// Make the run (room was given), send its outcome back, then follow
    /// a new session; `turn` when it holds a first turn, given back once
    /// the follow ends.
    pub(super) async fn make(
        self,
        job: Job,
        turn: bool,
        reply: Option<oneshot::Sender<Result<ScheduleRun, AutomationError>>>,
    ) {
        let made = tokio::select! {
            () = self.stop.cancelled() => return,
            made = self.runner.make(job.clone()) => made,
        };
        let run = ScheduleRun {
            outcome: made.outcome,
            ..job.run.clone()
        };
        let session = match &run.outcome {
            RunOutcome::Started { session } => Some(*session),
            RunOutcome::Making
            | RunOutcome::Waiting { .. }
            | RunOutcome::Finished { .. }
            | RunOutcome::Prompted { .. }
            | RunOutcome::Skipped { .. }
            | RunOutcome::Failed { .. }
            | RunOutcome::Interrupted
            | RunOutcome::Unknown => None,
        };
        let (schedule, number) = (job.schedule, run.number);
        let back = MadeRun {
            schedule,
            run: run.clone(),
            end: made.end,
            reply,
        };
        if self.back.send(Back::Made(back)).await.is_err() {
            return;
        }
        let Some(session) = session else {
            return;
        };
        let job = Job { run, ..job };
        tokio::select! {
            () = self.stop.cancelled() => return,
            () = self.runner.follow(job, session, self.follow) => {}
        }
        if turn {
            let _stopping = self.back.send(Back::TurnOver { schedule, number }).await;
        }
    }
}
