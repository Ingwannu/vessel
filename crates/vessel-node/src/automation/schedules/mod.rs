//! Schedules (protocol 11, ADR 0033): one task owns every schedule, sleeps
//! until the earliest is due, and makes runs through a [`Runner`].
//!
//! [Decision Log]
//! - Purpose: a prompt sent on a cadence, to a new session or through an
//!   existing session's queue, as the person or agent that defined it, with
//!   every run recorded and none made twice.
//! - Constraint: AGENTS.md section 5: a long-lived stateful thing is a task
//!   owning its state, others hold a handle. A node at rest must stay at
//!   rest (3 s of CPU in six hours was measured): no ticker, no scan.
//! - Alternatives: the references' tickers (agent-orchestrator 15 s, GOAT
//!   5 s, Paseo 1 s reading every schedule's file); a timer task per
//!   schedule (256 tasks asleep, and their order on a step of the clock is
//!   anyone's guess).
//! - Choice: one actor ([`actor`]) holding the schedules and an index of
//!   due times ([`book`]), sleeping until the first, at most 10 minutes at a
//!   time, and woken by every command. Runs are made by tasks of their own,
//!   at most [`SLOTS`] at once, through a [`Runner`]: the node's own
//!   (`node_runner`) acts through the service's checks; the tests' fake
//!   records what it was asked. Who may change what is pure ([`rules`]).
//! - Cost: the runner is a generic parameter, so the actor's tests run
//!   without a node and on a clock they move ([`Clock`]); one more trait,
//!   with the node's and the tests' implementations.

mod actor;
mod book;
mod check;
pub(crate) mod node_runner;
pub(crate) mod rules;

use std::future::Future;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::{mpsc, oneshot};
use vessel_proto::automation::{
    EndReason, RunOutcome, ScheduleDefinition, ScheduleInfo, ScheduleRun,
};
use vessel_proto::{Creator, ScheduleId, SessionId};

use crate::automation::error::AutomationError;
use rules::Asker;

pub(crate) use actor::{Parts, start};

/// Runs made at once on a node.
pub(crate) const SLOTS: usize = 4;
/// How long `schedule.run` waits for a slot.
pub(crate) const RUN_NOW_WAIT: Duration = Duration::from_secs(30);
/// The longest the scheduler sleeps before reading the clock again: a
/// clock stepped forward is noticed within this.
pub(crate) const MAX_SLEEP: Duration = Duration::from_secs(600);

/// The wall clock. The node's reads the system's; the tests' moves when
/// they say (and with tokio's paused time).
pub(crate) trait Clock: Send + Sync + 'static {
    /// Now.
    fn now(&self) -> DateTime<Utc>;
}

/// The system's clock.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// One run to make.
#[derive(Debug, Clone)]
pub(crate) struct Job {
    pub(crate) schedule: ScheduleId,
    /// The schedule's name, for the session's notice and title.
    pub(crate) name: String,
    pub(crate) run: ScheduleRun,
    pub(crate) definition: ScheduleDefinition,
    pub(crate) defined_by: Creator,
    /// The schedule's run before this one, for the overlap rules.
    pub(crate) previous: Option<ScheduleRun>,
}

/// What making a run came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Made {
    pub(crate) outcome: RunOutcome,
    /// The schedule ends with it: its definer or its target is gone.
    pub(crate) end: Option<EndReason>,
}

impl Made {
    pub(crate) const fn outcome(outcome: RunOutcome) -> Self {
        Self { outcome, end: None }
    }
}

/// A later outcome of a run already made (`started` became `waiting` or
/// `finished`), sent by a runner following its session: the run as it is
/// now.
#[derive(Debug, Clone)]
pub(crate) struct Followed {
    pub(crate) schedule: ScheduleId,
    pub(crate) run: ScheduleRun,
}

/// Where a runner sends what it follows.
pub(crate) type FollowSender = mpsc::Sender<Followed>;

/// What makes runs: the node's, or a test's.
pub(crate) trait Runner: Send + Sync + 'static {
    /// Make `job`'s run.
    fn make(&self, job: Job) -> impl Future<Output = Made> + Send;

    /// Follow a `started` run's session until its first turn ends (or the
    /// session does), sending each later outcome to `follow`.
    fn follow(
        &self,
        job: Job,
        session: SessionId,
        follow: FollowSender,
    ) -> impl Future<Output = ()> + Send;

    /// At the node's start: what an unfinished run of the last node's life
    /// came to (`making` was interrupted; a session followed ended with the
    /// node). `None` keeps the record as it is.
    fn settle(&self, run: &ScheduleRun) -> impl Future<Output = Option<RunOutcome>> + Send;
}

/// A command to the scheduler (only its handle makes them).
#[derive(Debug)]
pub(crate) enum Command {
    List {
        reply: oneshot::Sender<Vec<ScheduleInfo>>,
    },
    Get {
        schedule: ScheduleId,
        reply: oneshot::Sender<Result<ScheduleInfo, AutomationError>>,
    },
    Create {
        definition: Box<ScheduleDefinition>,
        paused: bool,
        asker: Asker,
        reply: oneshot::Sender<Result<ScheduleInfo, AutomationError>>,
    },
    Update {
        schedule: ScheduleId,
        revision: u32,
        definition: Box<ScheduleDefinition>,
        asker: Asker,
        reply: oneshot::Sender<Result<ScheduleInfo, AutomationError>>,
    },
    Pause {
        schedule: ScheduleId,
        asker: Asker,
        reply: oneshot::Sender<Result<ScheduleInfo, AutomationError>>,
    },
    Resume {
        schedule: ScheduleId,
        asker: Asker,
        reply: oneshot::Sender<Result<ScheduleInfo, AutomationError>>,
    },
    Delete {
        schedule: ScheduleId,
        asker: Asker,
        reply: oneshot::Sender<Result<(), AutomationError>>,
    },
    Run {
        schedule: ScheduleId,
        asker: Asker,
        reply: oneshot::Sender<Result<ScheduleRun, AutomationError>>,
    },
}

/// The handle every call holds: commands to the scheduler's task.
#[derive(Debug, Clone)]
pub(crate) struct Schedules {
    commands: mpsc::Sender<Command>,
}

/// Commands that may wait for the scheduler; a call finding no room waits.
const COMMANDS: usize = 64;

fn gone() -> AutomationError {
    AutomationError::Unavailable("the node is stopping".to_owned())
}

impl Schedules {
    /// A handle and the receiver its scheduler takes ([`start`]).
    pub(crate) fn channel() -> (Self, mpsc::Receiver<Command>) {
        let (commands, receiver) = mpsc::channel(COMMANDS);
        (Self { commands }, receiver)
    }

    async fn ask<T>(
        &self,
        command: impl FnOnce(oneshot::Sender<Result<T, AutomationError>>) -> Command,
    ) -> Result<T, AutomationError> {
        let (reply, answer) = oneshot::channel();
        self.commands
            .send(command(reply))
            .await
            .map_err(|_closed| gone())?;
        answer.await.map_err(|_dropped| gone())?
    }

    pub(crate) async fn list(&self) -> Result<Vec<ScheduleInfo>, AutomationError> {
        let (reply, answer) = oneshot::channel();
        self.commands
            .send(Command::List { reply })
            .await
            .map_err(|_closed| gone())?;
        answer.await.map_err(|_dropped| gone())
    }

    pub(crate) async fn get(&self, schedule: ScheduleId) -> Result<ScheduleInfo, AutomationError> {
        self.ask(|reply| Command::Get { schedule, reply }).await
    }

    pub(crate) async fn create(
        &self,
        definition: ScheduleDefinition,
        paused: bool,
        asker: Asker,
    ) -> Result<ScheduleInfo, AutomationError> {
        self.ask(|reply| Command::Create {
            definition: Box::new(definition),
            paused,
            asker,
            reply,
        })
        .await
    }

    pub(crate) async fn update(
        &self,
        schedule: ScheduleId,
        revision: u32,
        definition: ScheduleDefinition,
        asker: Asker,
    ) -> Result<ScheduleInfo, AutomationError> {
        self.ask(|reply| Command::Update {
            schedule,
            revision,
            definition: Box::new(definition),
            asker,
            reply,
        })
        .await
    }

    pub(crate) async fn pause(
        &self,
        schedule: ScheduleId,
        asker: Asker,
    ) -> Result<ScheduleInfo, AutomationError> {
        self.ask(|reply| Command::Pause {
            schedule,
            asker,
            reply,
        })
        .await
    }

    pub(crate) async fn resume(
        &self,
        schedule: ScheduleId,
        asker: Asker,
    ) -> Result<ScheduleInfo, AutomationError> {
        self.ask(|reply| Command::Resume {
            schedule,
            asker,
            reply,
        })
        .await
    }

    pub(crate) async fn delete(
        &self,
        schedule: ScheduleId,
        asker: Asker,
    ) -> Result<(), AutomationError> {
        self.ask(|reply| Command::Delete {
            schedule,
            asker,
            reply,
        })
        .await
    }

    pub(crate) async fn run(
        &self,
        schedule: ScheduleId,
        asker: Asker,
    ) -> Result<ScheduleRun, AutomationError> {
        self.ask(|reply| Command::Run {
            schedule,
            asker,
            reply,
        })
        .await
    }
}

#[cfg(test)]
mod tests;
