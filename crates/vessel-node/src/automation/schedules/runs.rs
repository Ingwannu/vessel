//! A run's way through the scheduler (ADR 0033): recorded, waiting for
//! room in due order (`room`), handed to a task, and what it came to kept;
//! asked runs that waited past their deadline answered `unavailable`.
//! The parent module's actor owns the state; this is its part about runs,
//! as `calls` is its part about calls. Must not: decide when a schedule
//! is due (`actor::fire` does, with `time`).

use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use tokio::sync::oneshot;
use vessel_proto::ScheduleId;
use vessel_proto::automation::{
    EndReason, RunNumber, RunOutcome, RunTrigger, ScheduleInfo, ScheduleRun, ScheduleState,
    ScheduleTarget,
};

use super::actor::Actor;
use super::room::Let;
use super::state::{ends_an_agents, name_of, next_number};
use super::task::{Back, MadeRun, RunTask, answer, no_room};
use super::{Clock, Followed, Job, RUN_NOW_WAIT, Runner};
use crate::automation::error::AutomationError;

/// How long an asked run waits for room, on the scheduler's clock.
fn run_now_wait() -> TimeDelta {
    TimeDelta::from_std(RUN_NOW_WAIT).unwrap_or(TimeDelta::MAX)
}

/// A recorded run waiting for room, and who asked for it.
#[derive(Debug)]
pub(super) struct Pending {
    job: Job,
    reply: Option<oneshot::Sender<Result<ScheduleRun, AutomationError>>>,
}

impl<R: Runner, C: Clock> Actor<R, C> {
    /// A run that does nothing, recorded at once.
    pub(super) async fn record(
        &mut self,
        mut info: ScheduleInfo,
        due_at: DateTime<Utc>,
        trigger: RunTrigger,
        outcome: RunOutcome,
    ) {
        let run = ScheduleRun {
            archive: None,
            number: next_number(&info),
            due_at,
            at: self.clock.now(),
            trigger,
            outcome,
        };
        info.last_run = Some(run.clone());
        if ends_an_agents(&info) {
            self.forget(info.id).await;
            return;
        }
        self.persist(&info, Some(run)).await;
        self.book.put(info);
    }

    /// Record a run `making` (with the schedule's state, in one write) and
    /// let it wait for room. An asked run waits [`RUN_NOW_WAIT`] at most,
    /// and is answered on `reply` once it has its outcome.
    pub(super) async fn start_run(
        &mut self,
        mut info: ScheduleInfo,
        due_at: DateTime<Utc>,
        trigger: RunTrigger,
        reply: Option<oneshot::Sender<Result<ScheduleRun, AutomationError>>>,
    ) {
        let asked = reply.is_some();
        let last = info.last_run.clone();
        let previous = self
            .book
            .get(info.id)
            .and_then(|entry| entry.current.clone());
        let run = ScheduleRun {
            archive: None,
            number: next_number(&info),
            due_at,
            at: self.clock.now(),
            trigger,
            outcome: RunOutcome::Making,
        };
        info.last_run = Some(run.clone());
        if let Err(error) = self
            .store
            .put_schedule(info.clone(), Some(run.clone()))
            .await
        {
            // Not recorded, so not made: a run made unrecorded could be
            // made again after a restart.
            tracing::error!(schedule = %info.id, %error, "cannot record a run; it is not made");
            info.last_run = last;
            self.book.put(info);
            if let Some(reply) = reply {
                let _gone = reply.send(Err(error.into()));
            }
            return;
        }
        let job = Job {
            schedule: info.id,
            name: name_of(&info),
            run,
            definition: info.definition.clone(),
            defined_by: info.defined_by.clone(),
            previous,
        };
        let id = info.id;
        self.book.put(info);
        self.book.set_making(id, true);
        let starts = matches!(job.definition.target, ScheduleTarget::NewSession { .. });
        let deadline = asked
            .then(|| self.clock.now().checked_add_signed(run_now_wait()))
            .flatten();
        self.room
            .wait(due_at, id, Pending { job, reply }, starts, deadline);
        self.dispatch();
    }

    /// Hand every run that has room to a task of its own, in the room's
    /// order.
    pub(super) fn dispatch(&mut self) {
        while let Some(Let { item, turn }) = self.room.next() {
            let Pending { job, reply } = item;
            if turn {
                let _new = self.turns.insert((job.schedule, job.run.number));
            }
            let task = RunTask {
                runner: Arc::clone(&self.runner),
                back: self.made_tx.clone(),
                follow: self.follow_tx.clone(),
                stop: self.stop.clone(),
            };
            drop(self.tasks.spawn(task.make(job, turn, reply)));
        }
    }

    /// Asked runs that waited for room past their deadline: not made,
    /// recorded `failed` (`unavailable`) and answered so.
    pub(super) async fn expire(&mut self, now: DateTime<Utc>) {
        for Pending { job, reply } in self.room.expired(now) {
            let run = ScheduleRun {
                outcome: no_room(),
                ..job.run
            };
            let made = MadeRun {
                schedule: job.schedule,
                run,
                end: None,
                reply,
            };
            self.made(made).await;
        }
    }

    /// What a run's task sent back.
    pub(super) async fn back(&mut self, back: Back) {
        match back {
            Back::Made(made) => {
                self.room.slot_freed();
                if !matches!(made.run.outcome, RunOutcome::Started { .. }) {
                    self.turn_over(made.schedule, made.run.number);
                }
                self.made(made).await;
            }
            Back::TurnOver { schedule, number } => self.turn_over(schedule, number),
        }
        self.dispatch();
    }

    /// A run's first turn no longer holds room, if it held one.
    pub(super) fn turn_over(&mut self, schedule: ScheduleId, number: RunNumber) {
        if self.turns.remove(&(schedule, number)) {
            self.room.turn_freed();
        }
    }

    /// A run's outcome: from its task, or never made.
    pub(super) async fn made(&mut self, made: MadeRun) {
        let MadeRun {
            schedule,
            run,
            end,
            reply,
        } = made;
        self.book.set_making(schedule, false);
        let Some(entry) = self.book.get(schedule).cloned() else {
            // Deleted while it was made: the run happened all the same.
            if let Some(reply) = reply {
                let _gone = reply.send(Ok(run));
            }
            return;
        };
        let mut info = entry.info;
        if run.outcome.counts() {
            info.runs = info.runs.saturating_add(1);
        }
        if info
            .last_run
            .as_ref()
            .is_some_and(|last| last.number == run.number)
        {
            info.last_run = Some(run.clone());
        }
        let now = self.clock.now();
        if let Some(reason) = end {
            info.state = ScheduleState::Ended { at: now, reason };
        } else if let Some(max) = info.definition.max_runs
            && info.runs >= max.get()
            && !matches!(info.state, ScheduleState::Ended { .. })
        {
            info.state = ScheduleState::Ended {
                at: now,
                reason: EndReason::MaxRuns,
            };
        }
        if ends_an_agents(&info) {
            self.forget(schedule).await;
        } else {
            self.persist(&info, Some(run.clone())).await;
            self.book.put(info);
            self.book.noted(schedule, &run);
        }
        if let Some(reply) = reply {
            let _gone = reply.send(answer(run));
        }
    }

    /// A followed run's later outcome. A session waiting for a person, or
    /// done with its first turn, gives its first turn's room back.
    pub(super) async fn followed(&mut self, followed: Followed) {
        let Followed {
            schedule,
            run,
            committed,
        } = followed;
        if matches!(
            run.outcome,
            RunOutcome::Waiting { .. } | RunOutcome::Finished { .. }
        ) {
            self.turn_over(schedule, run.number);
            self.dispatch();
        }
        let Some(entry) = self.book.get(schedule).cloned() else {
            // Deletion already discarded history, but never discards a pin.
            if let Some(committed) = committed {
                let _gone = committed.send(true);
            }
            return;
        };
        let mut info = entry.info;
        let last = info
            .last_run
            .as_ref()
            .is_some_and(|last| last.number == run.number);
        let result = if last {
            info.last_run = Some(run.clone());
            let result = self
                .store
                .put_schedule(info.clone(), Some(run.clone()))
                .await;
            self.book.put(info);
            result
        } else {
            self.store.put_run(schedule, run.clone()).await
        };
        if let Err(error) = &result {
            tracing::warn!(%schedule, %error, "cannot record a run's outcome");
        }
        if let Some(committed) = committed {
            let _gone = committed.send(result.is_ok());
        }
        self.book.noted(schedule, &run);
    }
    /// Keep the first archive disposition alongside the original turn.
    /// A late release must not replace a newer run or re-create trimmed history.
    pub(super) async fn note_archive(
        &mut self,
        schedule: ScheduleId,
        number: RunNumber,
        disposition: vessel_proto::automation::RunArchive,
    ) -> Result<(), AutomationError> {
        let Some(entry) = self.book.get(schedule) else {
            return Ok(());
        };
        let mut info = entry.info.clone();
        let Some(mut run) = self
            .store
            .runs(schedule)
            .await?
            .into_iter()
            .find(|run| run.number == number)
        else {
            return Ok(());
        };
        if run.archive.is_some() {
            return Ok(());
        }
        run.archive = Some(disposition);
        if info
            .last_run
            .as_ref()
            .is_some_and(|last| last.number == number)
        {
            info.last_run = Some(run.clone());
            self.store
                .put_schedule(info.clone(), Some(run.clone()))
                .await?;
            self.book.put(info);
        } else {
            self.store.put_run(schedule, run.clone()).await?;
        }
        self.book.noted(schedule, &run);
        Ok(())
    }
}
