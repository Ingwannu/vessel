//! The scheduler's task (ADR 0033): owns the schedules, sleeps until the
//! earliest is due, decides each due time (due, caught up, skipped),
//! records a run before making it, hands it to a run task, and keeps what
//! the run came to. The parent module's Decision Log says why one task.
//!
//! Order of a run (the contract's "How a run goes"): the run is recorded
//! `making` in the same write as the schedule's next due time, waits for
//! room ([`Room`]: one of [`SLOTS`], and a first turn for a run that
//! starts a session), oldest due first, then is made by a task; the
//! task's outcome comes back here ([`MadeRun`]) and is written; a
//! `started` run's session is followed by the runner, whose later
//! outcomes come back too ([`Followed`]).

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::automation::{
    RunNumber, RunOutcome, RunTrigger, ScheduleInfo, ScheduleRun, ScheduleState,
};
use vessel_proto::{Creator, ScheduleId, SessionId};
use vessel_store::Store;

use super::book::Book;
use super::room::{Room, TURNS};
use super::runs::Pending;
use super::state::advanced;
use super::task::Back;
use super::{Clock, Command, FollowSender, Followed, MAX_SLEEP, Runner, SLOTS};
use crate::automation::time::{self, Decision, Due, Zones};

/// A clock read this far behind the last one was stepped back.
fn stepped_back() -> TimeDelta {
    TimeDelta::seconds(60)
}

/// A last due time this far ahead of the clock means the clock was wrong
/// when the run was made: the next is computed from now.
fn wrong_ahead() -> TimeDelta {
    TimeDelta::hours(24)
}

/// Outcomes coming back from run tasks; a task finding it full waits.
const MADE: usize = 64;

/// What the scheduler is started with.
#[derive(Debug)]
pub(crate) struct Parts<R, C> {
    pub(crate) runner: Arc<R>,
    pub(crate) clock: C,
    pub(crate) store: Store,
    pub(crate) tasks: TaskTracker,
    pub(crate) stop: CancellationToken,
    /// Each session's end: an agent's schedules go with its session.
    pub(crate) ended: broadcast::Receiver<SessionId>,
}

/// Read the schedules the store keeps, settle what the last run of the
/// node left unfinished, and start the scheduler's task.
pub(crate) async fn start<R: Runner, C: Clock>(
    commands: mpsc::Receiver<Command>,
    parts: Parts<R, C>,
) {
    let Parts {
        runner,
        clock,
        store,
        tasks,
        stop,
        ended,
    } = parts;
    let (made_tx, made_rx) = mpsc::channel(MADE);
    let (follow_tx, follow_rx) = mpsc::channel(MADE);
    let mut actor = Actor {
        book: Book::default(),
        zones: Zones::default(),
        store,
        runner,
        clock,
        room: Room::new(SLOTS, TURNS),
        turns: HashSet::new(),
        commands,
        made_tx,
        made_rx,
        follow_tx,
        follow_rx,
        tasks: tasks.clone(),
        stop,
        ended: Some(ended),
        last_now: None,
    };
    actor.load().await;
    drop(tasks.spawn(actor.run()));
}

pub(super) struct Actor<R, C> {
    pub(super) book: Book,
    pub(super) zones: Zones,
    pub(super) store: Store,
    pub(super) runner: Arc<R>,
    pub(super) clock: C,
    /// Runs being made and waiting (`room`).
    pub(super) room: Room<Pending>,
    /// The runs whose session holds a first turn, by schedule and run.
    pub(super) turns: HashSet<(ScheduleId, RunNumber)>,
    pub(super) commands: mpsc::Receiver<Command>,
    pub(super) made_tx: mpsc::Sender<Back>,
    pub(super) made_rx: mpsc::Receiver<Back>,
    pub(super) follow_tx: FollowSender,
    pub(super) follow_rx: mpsc::Receiver<Followed>,
    pub(super) tasks: TaskTracker,
    pub(super) stop: CancellationToken,
    /// Sessions' ends; `None` once the sender is gone.
    pub(super) ended: Option<broadcast::Receiver<SessionId>>,
    /// The clock at the last look, to notice it stepped back.
    pub(super) last_now: Option<DateTime<Utc>>,
}

impl<R: Runner, C: Clock> Actor<R, C> {
    async fn run(mut self) {
        loop {
            let now = self.clock.now();
            self.expire(now).await;
            self.fire_due(now).await;
            self.dispatch();
            let sleep = self.sleep_for(now);
            // What runs came to is kept before the next command is read, so
            // a call never sees a run older than one already answered.
            tokio::select! {
                biased;
                () = self.stop.cancelled() => break,
                Some(back) = self.made_rx.recv() => self.back(back).await,
                Some(followed) = self.follow_rx.recv() => self.followed(followed).await,
                ended = next_end(self.ended.as_mut()) => match ended {
                    Ok(session) => self.session_ended(session).await,
                    // Missed ends: every agent's schedule is checked now.
                    Err(broadcast::error::RecvError::Lagged(missed)) => {
                        tracing::info!(missed, "the scheduler missed some sessions' ends; agents' schedules checked");
                        self.sweep_agents().await;
                    }
                    Err(broadcast::error::RecvError::Closed) => self.ended = None,
                },
                command = self.commands.recv() => match command {
                    Some(command) => self.handle(command).await,
                    None => break,
                },
                () = sleep_or_wait(sleep) => {}
            }
        }
    }

    /// How long to sleep: until the earliest due time or an asked run's
    /// deadline, at most [`MAX_SLEEP`]; with neither, until something
    /// happens.
    fn sleep_for(&self, now: DateTime<Utc>) -> Option<Duration> {
        let first = match (self.book.first_due(), self.room.next_deadline()) {
            (Some(due), Some(deadline)) => due.min(deadline),
            (Some(first), None) | (None, Some(first)) => first,
            (None, None) => return None,
        };
        let wait = first
            .signed_duration_since(now)
            .to_std()
            .unwrap_or(Duration::ZERO);
        Some(wait.min(MAX_SLEEP))
    }

    async fn load(&mut self) {
        let stored = match self.store.automation().await {
            Ok(stored) => stored,
            Err(error) => {
                tracing::error!(%error, "cannot read the schedules; none will run until a restart reads them");
                return;
            }
        };
        let now = self.clock.now();
        for mut info in stored.schedules {
            // No session survives a restart, so neither does a schedule an
            // agent's session made (ADR 0033).
            if matches!(info.defined_by, Creator::Agent { .. }) {
                if let Err(error) = self.store.forget_schedule(info.id).await {
                    tracing::warn!(schedule = %info.id, %error, "cannot delete an agent's schedule");
                }
                continue;
            }
            // The run to settle is the newest that did something: skips
            // recorded since sit above it.
            let mut current = self.current_of(&info).await;
            let mut settled = None;
            if let Some(run) = &mut current
                && let Some(outcome) = self.runner.settle(run).await
            {
                run.outcome = outcome;
                settled = Some(run.clone());
                if info
                    .last_run
                    .as_ref()
                    .is_some_and(|last| last.number == run.number)
                {
                    info.last_run = Some(run.clone());
                }
            }
            let corrected = self.correct(&mut info, now);
            if settled.is_some() || corrected {
                self.persist(&info, settled).await;
            }
            let id = info.id;
            self.book.put(info);
            self.book.set_current(id, current);
        }
        self.last_now = Some(now);
    }

    /// A schedule's newest stored run that did something (not a skip).
    async fn current_of(&self, info: &ScheduleInfo) -> Option<ScheduleRun> {
        let not_skipped = |run: &ScheduleRun| !matches!(run.outcome, RunOutcome::Skipped { .. });
        match self.store.runs(info.id).await {
            Ok(runs) => runs.into_iter().find(not_skipped),
            Err(error) => {
                tracing::warn!(schedule = %info.id, %error, "cannot read a schedule's runs; its last one stands for them");
                info.last_run.clone().filter(not_skipped)
            }
        }
    }

    /// A last due time far ahead of `now`: the clock was wrong when it
    /// ran, and the next due time comes from now. Whether it changed.
    fn correct(&self, info: &mut ScheduleInfo, now: DateTime<Utc>) -> bool {
        let ahead = info
            .last_due
            .is_some_and(|last| last.signed_duration_since(now) > wrong_ahead());
        if !ahead || !matches!(info.state, ScheduleState::Active { .. }) {
            return false;
        }
        let next = Due::of(&info.definition.cadence, info.created_at, &self.zones)
            .ok()
            .and_then(|due| due.next_after(now));
        info.state = advanced(&info.definition, next, now);
        tracing::warn!(schedule = %info.id, "the clock was behind a schedule's last run by more than a day; its next due time is counted from now");
        true
    }

    async fn fire_due(&mut self, now: DateTime<Utc>) {
        if let Some(last) = self.last_now
            && now < last.checked_sub_signed(stepped_back()).unwrap_or(last)
        {
            self.clock_stepped_back(now).await;
        }
        self.last_now = Some(now);
        while let Some(id) = self.book.next_due_by(now) {
            self.fire(id, now).await;
        }
    }

    /// The clock went back: a pass over every schedule (not at a wake,
    /// only on such a step) for runs the clock was wrong about.
    async fn clock_stepped_back(&mut self, now: DateTime<Utc>) {
        let infos: Vec<ScheduleInfo> = self.book.all().map(|entry| entry.info.clone()).collect();
        for mut info in infos {
            if self.correct(&mut info, now) {
                self.persist(&info, None).await;
                self.book.put(info);
            }
        }
    }

    /// `id` is due at or before `now`: decide, record, make. Every path
    /// moves its due time past `now` or ends it, so `fire_due` ends.
    async fn fire(&mut self, id: ScheduleId, now: DateTime<Utc>) {
        let Some(entry) = self.book.get(id).cloned() else {
            return;
        };
        let mut info = entry.info;
        let ScheduleState::Active { next_due } = info.state else {
            return;
        };
        let due = match Due::of(&info.definition.cadence, info.created_at, &self.zones) {
            Ok(due) => due,
            Err(error) => {
                // Checked when it was kept: a zone gone from a newer
                // database. Paused, so a person sees it.
                tracing::warn!(schedule = %id, %error, "a schedule's due times cannot be computed; paused");
                info.state = ScheduleState::Paused { since: now };
                self.persist(&info, None).await;
                self.book.put(info);
                return;
            }
        };
        // Due times after `ends_at` never come, also when they were missed
        // while the node was stopped; how old the latest is still counts
        // from now.
        let until = info
            .definition
            .ends_at
            .map_or(now, |ends_at| ends_at.min(now));
        let mut missed = time::missed(&due, next_due, until);
        if until < now {
            missed.recent = missed
                .recent
                .filter(|recent| now.signed_duration_since(*recent) < time::too_old());
        }
        let decision = missed.decide(info.definition.catch_up, now);
        info.state = advanced(&info.definition, missed.next, now);
        match decision {
            Decision::Run { due_at, trigger } if !entry.making => {
                info.last_due = Some(due_at);
                self.start_run(info, due_at, trigger, None).await;
            }
            Decision::Run { due_at, .. } => {
                let skipped = RunOutcome::Skipped {
                    reason: vessel_proto::automation::SkipReason::PreviousRunning,
                };
                self.record(info, due_at, RunTrigger::Due, skipped).await;
            }
            Decision::Skip { due_at, reason } => {
                let skipped = RunOutcome::Skipped { reason };
                self.record(info, due_at, RunTrigger::Due, skipped).await;
            }
        }
    }

    pub(super) async fn persist(&self, info: &ScheduleInfo, run: Option<ScheduleRun>) {
        if let Err(error) = self.store.put_schedule(info.clone(), run).await {
            tracing::warn!(schedule = %info.id, %error, "cannot record a schedule");
        }
    }

    pub(super) async fn forget(&mut self, schedule: ScheduleId) {
        drop(self.book.remove(schedule));
        if let Err(error) = self.store.forget_schedule(schedule).await {
            tracing::warn!(%schedule, %error, "cannot delete a schedule");
        }
    }
}

/// The next session end, or never once there are none to hear.
async fn next_end(
    ended: Option<&mut broadcast::Receiver<SessionId>>,
) -> Result<SessionId, broadcast::error::RecvError> {
    match ended {
        Some(ended) => ended.recv().await,
        None => std::future::pending().await,
    }
}

async fn sleep_or_wait(sleep: Option<Duration>) {
    match sleep {
        Some(sleep) => tokio::time::sleep(sleep).await,
        None => std::future::pending().await,
    }
}
