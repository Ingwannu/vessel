//! A run's own task (ADR 0033): takes one of the scheduler's slots, makes
//! the run through the runner, sends its outcome back to the scheduler,
//! frees the slot, then follows a session it started. Must not: write the
//! store (the scheduler records every outcome it sends back).

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Semaphore, mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use vessel_proto::ScheduleId;
use vessel_proto::automation::{EndReason, RunError, RunErrorCode, RunOutcome, ScheduleRun};

use super::{FollowSender, Job, Made, Runner};
use crate::automation::error::AutomationError;

/// A run's outcome, back from its task.
#[derive(Debug)]
pub(super) struct MadeRun {
    pub(super) schedule: ScheduleId,
    pub(super) run: ScheduleRun,
    pub(super) end: Option<EndReason>,
    pub(super) reply: Option<oneshot::Sender<Result<ScheduleRun, AutomationError>>>,
}

/// What `schedule.run` answers: the run, or `unavailable` when it never
/// had a slot (the contract's answer; the run is recorded `failed` all the
/// same).
pub(super) fn answer(run: ScheduleRun) -> Result<ScheduleRun, AutomationError> {
    match &run.outcome {
        RunOutcome::Failed {
            error,
            session: None,
        } if error.code == RunErrorCode::Unavailable && error.message == NO_SLOT => {
            Err(AutomationError::Unavailable(NO_SLOT.to_owned()))
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
const NO_SLOT: &str = "no run slot came free in time; the node is making four runs";

/// What a run's task needs.
pub(super) struct RunTask<R> {
    pub(super) runner: Arc<R>,
    pub(super) slots: Arc<Semaphore>,
    pub(super) made: mpsc::Sender<MadeRun>,
    pub(super) follow: FollowSender,
    pub(super) stop: CancellationToken,
}

impl<R: Runner> RunTask<R> {
    /// Take a slot (waiting at most `wait`, when given), make the run,
    /// send its outcome back, free the slot, then follow a new session.
    pub(super) async fn make(
        self,
        job: Job,
        wait: Option<Duration>,
        reply: Option<oneshot::Sender<Result<ScheduleRun, AutomationError>>>,
    ) {
        let made = tokio::select! {
            () = self.stop.cancelled() => return,
            made = self.slot_then_make(&job, wait) => made,
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
        let back = MadeRun {
            schedule: job.schedule,
            run: run.clone(),
            end: made.end,
            reply,
        };
        if self.made.send(back).await.is_err() {
            return;
        }
        if let Some(session) = session {
            let job = Job { run, ..job };
            tokio::select! {
                () = self.stop.cancelled() => {}
                () = self.runner.follow(job, session, self.follow) => {}
            }
        }
    }

    async fn slot_then_make(&self, job: &Job, wait: Option<Duration>) -> Made {
        let slot = match wait {
            None => Arc::clone(&self.slots).acquire_owned().await.ok(),
            Some(wait) => tokio::time::timeout(wait, Arc::clone(&self.slots).acquire_owned())
                .await
                .ok()
                .and_then(Result::ok),
        };
        let Some(_slot) = slot else {
            return Made::outcome(RunOutcome::Failed {
                error: RunError {
                    code: RunErrorCode::Unavailable,
                    message: NO_SLOT.to_owned(),
                },
                session: None,
            });
        };
        self.runner.make(job.clone()).await
    }
}
