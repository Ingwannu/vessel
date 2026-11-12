//! The scheduler's task (ADR 0033): owns the schedules, sleeps until the
//! earliest is due, decides each due time (due, caught up, skipped),
//! records a run before making it, hands it to a run task, and keeps what
//! the run came to. The parent module's Decision Log says why one task.
//!
//! Order of a run (the contract's "How a run goes"): the run is recorded
//! `making` in the same write as the schedule's next due time, then made
//! by a task holding one of [`SLOTS`]; the task's outcome comes back here
//! ([`MadeRun`]) and is written; a `started` run's session is followed by
//! the runner, whose later outcomes come back too ([`Followed`]).

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use tokio::sync::{Semaphore, broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::automation::{
    EndReason, RunError, RunErrorCode, RunNumber, RunOutcome, RunTrigger, ScheduleDefinition,
    ScheduleInfo, ScheduleRun, ScheduleState,
};
use vessel_proto::{Creator, ScheduleId, SessionId};
use vessel_store::Store;

use super::book::Book;
use super::check::settle;
use super::rules::{AGENT_SCHEDULES, Act, Asker};
use super::{
    Clock, Command, FollowSender, Followed, Job, MAX_SLEEP, Made, RUN_NOW_WAIT, Runner, SLOTS,
};
use crate::automation::error::AutomationError;
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

/// A run's outcome, back from its task.
#[derive(Debug)]
struct MadeRun {
    schedule: ScheduleId,
    run: ScheduleRun,
    end: Option<EndReason>,
    reply: Option<oneshot::Sender<Result<ScheduleRun, AutomationError>>>,
}

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
        slots: Arc::new(Semaphore::new(SLOTS)),
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

struct Actor<R, C> {
    book: Book,
    zones: Zones,
    store: Store,
    runner: Arc<R>,
    clock: C,
    slots: Arc<Semaphore>,
    commands: mpsc::Receiver<Command>,
    made_tx: mpsc::Sender<MadeRun>,
    made_rx: mpsc::Receiver<MadeRun>,
    follow_tx: FollowSender,
    follow_rx: mpsc::Receiver<Followed>,
    tasks: TaskTracker,
    stop: CancellationToken,
    /// Sessions' ends; `None` once the sender is gone.
    ended: Option<broadcast::Receiver<SessionId>>,
    /// The clock at the last look, to notice it stepped back.
    last_now: Option<DateTime<Utc>>,
}

impl<R: Runner, C: Clock> Actor<R, C> {
    async fn run(mut self) {
        loop {
            let now = self.clock.now();
            self.fire_due(now).await;
            let sleep = self.sleep_for(now);
            // What runs came to is kept before the next command is read, so
            // a call never sees a run older than one already answered.
            tokio::select! {
                biased;
                () = self.stop.cancelled() => break,
                Some(made) = self.made_rx.recv() => self.made(made).await,
                Some(followed) = self.follow_rx.recv() => self.followed(followed).await,
                ended = next_end(self.ended.as_mut()) => match ended {
                    Ok(session) => self.session_ended(session).await,
                    // Missed ends: those agents' schedules end at their
                    // next run, which finds the agent gone.
                    Err(broadcast::error::RecvError::Lagged(missed)) => {
                        tracing::debug!(missed, "the scheduler missed some sessions' ends");
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

    /// How long to sleep: until the earliest due time, at most
    /// [`MAX_SLEEP`]; with nothing active, until something happens.
    fn sleep_for(&self, now: DateTime<Utc>) -> Option<Duration> {
        let first = self.book.first_due()?;
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
            let mut settled = None;
            if let Some(run) = &mut info.last_run
                && let Some(outcome) = self.runner.settle(run).await
            {
                run.outcome = outcome;
                settled = Some(run.clone());
            }
            let corrected = self.correct(&mut info, now);
            if settled.is_some() || corrected {
                self.persist(&info, settled).await;
            }
            self.book.put(info);
        }
        self.last_now = Some(now);
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
        let missed = time::missed(&due, next_due, now);
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

    /// A run that does nothing, recorded at once.
    async fn record(
        &mut self,
        mut info: ScheduleInfo,
        due_at: DateTime<Utc>,
        trigger: RunTrigger,
        outcome: RunOutcome,
    ) {
        let run = ScheduleRun {
            number: next_number(&info),
            due_at,
            at: self.clock.now(),
            trigger,
            outcome,
        };
        info.last_run = Some(run.clone());
        self.persist(&info, Some(run)).await;
        self.book.put(info);
    }

    /// Record a run `making` (with the schedule's state, in one write) and
    /// hand it to a task. An asked run waits [`RUN_NOW_WAIT`] at most for
    /// a slot, and is answered on `reply` once it has its outcome.
    async fn start_run(
        &mut self,
        mut info: ScheduleInfo,
        due_at: DateTime<Utc>,
        trigger: RunTrigger,
        reply: Option<oneshot::Sender<Result<ScheduleRun, AutomationError>>>,
    ) {
        let asked = reply.is_some();
        let previous = info.last_run.clone();
        let run = ScheduleRun {
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
            info.last_run = previous;
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
        let task = RunTask {
            runner: Arc::clone(&self.runner),
            slots: Arc::clone(&self.slots),
            made: self.made_tx.clone(),
            follow: self.follow_tx.clone(),
            stop: self.stop.clone(),
        };
        let wait = asked.then_some(RUN_NOW_WAIT);
        drop(self.tasks.spawn(task.make(job, wait, reply)));
    }

    /// A run task's outcome.
    async fn made(&mut self, made: MadeRun) {
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
        if end.is_some() && matches!(info.defined_by, Creator::Agent { .. }) {
            // An agent's schedule that ends goes (its definer is gone).
            self.forget(schedule).await;
        } else {
            self.persist(&info, Some(run.clone())).await;
            self.book.put(info);
        }
        if let Some(reply) = reply {
            let _gone = reply.send(Ok(run));
        }
    }

    /// A followed run's later outcome.
    async fn followed(&mut self, followed: Followed) {
        let Followed { schedule, run } = followed;
        let Some(entry) = self.book.get(schedule).cloned() else {
            return;
        };
        let mut info = entry.info;
        if info
            .last_run
            .as_ref()
            .is_some_and(|last| last.number == run.number)
        {
            info.last_run = Some(run.clone());
            self.persist(&info, Some(run)).await;
            self.book.put(info);
        } else if let Err(error) = self.store.put_run(schedule, run).await {
            tracing::warn!(%schedule, %error, "cannot record a run's outcome");
        }
    }

    async fn persist(&self, info: &ScheduleInfo, run: Option<ScheduleRun>) {
        if let Err(error) = self.store.put_schedule(info.clone(), run).await {
            tracing::warn!(schedule = %info.id, %error, "cannot record a schedule");
        }
    }

    async fn forget(&mut self, schedule: ScheduleId) {
        drop(self.book.remove(schedule));
        if let Err(error) = self.store.forget_schedule(schedule).await {
            tracing::warn!(%schedule, %error, "cannot delete a schedule");
        }
    }

    async fn handle(&mut self, command: Command) {
        match command {
            Command::List { reply } => {
                let _gone = reply.send(self.book.list());
            }
            Command::Get { schedule, reply } => {
                let found = self.found(schedule).map(|entry| entry.info.clone());
                let _gone = reply.send(found);
            }
            Command::Create {
                definition,
                paused,
                asker,
                reply,
            } => {
                let created = self.create(*definition, paused, asker).await;
                let _gone = reply.send(created);
            }
            Command::Update {
                schedule,
                revision,
                definition,
                asker,
                reply,
            } => {
                let updated = self.update(schedule, revision, *definition, &asker).await;
                let _gone = reply.send(updated);
            }
            Command::Pause {
                schedule,
                asker,
                reply,
            } => {
                let _gone = reply.send(self.pause(schedule, &asker, true).await);
            }
            Command::Resume {
                schedule,
                asker,
                reply,
            } => {
                let _gone = reply.send(self.pause(schedule, &asker, false).await);
            }
            Command::Delete {
                schedule,
                asker,
                reply,
            } => {
                let deleted = match self.found(schedule) {
                    Ok(entry) => asker.may(Act::Delete, &entry.info.defined_by),
                    Err(error) => Err(error),
                };
                if deleted.is_ok() {
                    self.forget(schedule).await;
                }
                let _gone = reply.send(deleted);
            }
            Command::Run {
                schedule,
                asker,
                reply,
            } => self.run_now(schedule, asker, reply).await,
        }
    }

    /// A session ended: the schedules its agent made go with it (ADR
    /// 0033: a session cannot use a schedule to outlive itself).
    async fn session_ended(&mut self, session: SessionId) {
        for schedule in self.book.of_agent(session) {
            self.forget(schedule).await;
        }
    }

    fn found(&self, schedule: ScheduleId) -> Result<&super::book::Entry, AutomationError> {
        self.book
            .get(schedule)
            .ok_or(AutomationError::NotFound("schedule"))
    }

    async fn create(
        &mut self,
        mut definition: ScheduleDefinition,
        paused: bool,
        asker: Asker,
    ) -> Result<ScheduleInfo, AutomationError> {
        let (people, agents) = self.book.counts();
        let own = match &asker.by {
            Creator::Agent { session, .. } => self.live_of_agent(*session),
            Creator::Owner | Creator::Device { .. } => 0,
        };
        asker.may_create(own, people, agents)?;
        let now = self.clock.now();
        let first = settle(&self.zones, &mut definition, now)?;
        let state = if paused {
            ScheduleState::Paused { since: now }
        } else {
            ScheduleState::Active { next_due: first }
        };
        let info = ScheduleInfo {
            id: ScheduleId::random(),
            definition,
            state,
            runs: 0,
            last_due: None,
            last_run: None,
            defined_by: asker.by,
            created_at: now,
            updated_at: now,
            revision: 0,
        };
        self.store.put_schedule(info.clone(), None).await?;
        self.book.put(info.clone());
        Ok(info)
    }

    /// An agent session's schedules that have not ended (what its bound
    /// counts).
    fn live_of_agent(&self, session: SessionId) -> usize {
        self.book
            .of_agent(session)
            .into_iter()
            .filter_map(|id| self.book.get(id))
            .filter(|entry| !matches!(entry.info.state, ScheduleState::Ended { .. }))
            .count()
            .min(AGENT_SCHEDULES)
    }

    async fn update(
        &mut self,
        schedule: ScheduleId,
        revision: u32,
        mut definition: ScheduleDefinition,
        asker: &Asker,
    ) -> Result<ScheduleInfo, AutomationError> {
        let mut info = self.found(schedule)?.info.clone();
        asker.may(Act::Update, &info.defined_by)?;
        ended_refused(&info)?;
        if revision != info.revision {
            return Err(AutomationError::Conflict(format!(
                "the schedule was changed meanwhile (now revision {}); edit that one",
                info.revision
            )));
        }
        let now = self.clock.now();
        let first = settle(&self.zones, &mut definition, now)?;
        let cadence_changed = definition.cadence != info.definition.cadence;
        info.state = match info.state {
            ScheduleState::Active { next_due } => {
                let next = if cadence_changed { first } else { next_due };
                advanced(&definition, Some(next), now)
            }
            other @ (ScheduleState::Paused { .. }
            | ScheduleState::Ended { .. }
            | ScheduleState::Unknown) => other,
        };
        if let Some(max) = definition.max_runs
            && info.runs >= max.get()
        {
            info.state = ScheduleState::Ended {
                at: now,
                reason: EndReason::MaxRuns,
            };
        }
        info.definition = definition;
        info.defined_by = asker.by.clone();
        info.revision = info.revision.wrapping_add(1);
        info.updated_at = now;
        self.store.put_schedule(info.clone(), None).await?;
        self.book.put(info.clone());
        Ok(info)
    }

    async fn pause(
        &mut self,
        schedule: ScheduleId,
        asker: &Asker,
        pause: bool,
    ) -> Result<ScheduleInfo, AutomationError> {
        let mut info = self.found(schedule)?.info.clone();
        asker.may(
            if pause { Act::Pause } else { Act::Resume },
            &info.defined_by,
        )?;
        ended_refused(&info)?;
        let now = self.clock.now();
        let state = match (&info.state, pause) {
            (ScheduleState::Active { .. }, false) | (ScheduleState::Paused { .. }, true) => {
                return Ok(info);
            }
            (ScheduleState::Active { .. }, true) => ScheduleState::Paused { since: now },
            (ScheduleState::Paused { .. }, false) => {
                let next = Due::of(&info.definition.cadence, info.created_at, &self.zones)
                    .map_err(|error| AutomationError::Invalid(error.to_string()))
                    .map(|due| due.next_after(now))?;
                advanced(&info.definition, next, now)
            }
            (ScheduleState::Ended { .. } | ScheduleState::Unknown, _) => {
                return Err(AutomationError::Conflict(
                    "the schedule has ended".to_owned(),
                ));
            }
        };
        info.state = state;
        info.revision = info.revision.wrapping_add(1);
        info.updated_at = now;
        self.store.put_schedule(info.clone(), None).await?;
        self.book.put(info.clone());
        Ok(info)
    }

    async fn run_now(
        &mut self,
        schedule: ScheduleId,
        asker: Asker,
        reply: oneshot::Sender<Result<ScheduleRun, AutomationError>>,
    ) {
        let entry = match self.found(schedule) {
            Ok(entry) => entry.clone(),
            Err(error) => {
                let _gone = reply.send(Err(error));
                return;
            }
        };
        let refused = asker
            .may(Act::RunNow, &entry.info.defined_by)
            .and_then(|()| ended_refused(&entry.info));
        if let Err(error) = refused {
            let _gone = reply.send(Err(error));
            return;
        }
        if entry.making {
            let _gone = reply.send(Err(AutomationError::Conflict(
                "a run of this schedule is being made".to_owned(),
            )));
            return;
        }
        let now = self.clock.now();
        let trigger = RunTrigger::Asked { by: asker.by };
        self.start_run(entry.info, now, trigger, Some(reply)).await;
    }
}

/// An ended schedule cannot change or run.
fn ended_refused(info: &ScheduleInfo) -> Result<(), AutomationError> {
    match info.state {
        ScheduleState::Ended { .. } | ScheduleState::Unknown => Err(AutomationError::Conflict(
            "the schedule has ended".to_owned(),
        )),
        ScheduleState::Active { .. } | ScheduleState::Paused { .. } => Ok(()),
    }
}

/// Active at `next`, or ended `expired` when there is none or it is past
/// `ends_at`.
fn advanced(
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
fn next_number(info: &ScheduleInfo) -> RunNumber {
    info.last_run
        .as_ref()
        .and_then(|run| run.number.next())
        .unwrap_or(RunNumber::FIRST)
}

/// What a schedule is called in its sessions: its name, or its id.
fn name_of(info: &ScheduleInfo) -> String {
    info.definition
        .name
        .as_ref()
        .map_or_else(|| info.id.to_string(), ToString::to_string)
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

/// What a run's task needs.
struct RunTask<R> {
    runner: Arc<R>,
    slots: Arc<Semaphore>,
    made: mpsc::Sender<MadeRun>,
    follow: FollowSender,
    stop: CancellationToken,
}

impl<R: Runner> RunTask<R> {
    /// Take a slot (waiting at most `wait`, when given), make the run,
    /// send its outcome back, free the slot, then follow a new session.
    async fn make(
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
                    message: "no run slot came free in time; the node is making four runs"
                        .to_owned(),
                },
                session: None,
            });
        };
        self.runner.make(job.clone()).await
    }
}
