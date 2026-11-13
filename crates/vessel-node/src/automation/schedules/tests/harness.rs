//! The scheduler's tests' harness: a wall clock moved by hand, a runner
//! that records what it is asked and answers as told, and a scheduler over
//! a fresh store (divided from `tests` when the join grew it).

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::{Semaphore, broadcast, mpsc};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::automation::{
    Cadence, CatchUp, EverySeconds, RunOutcome, ScheduleDefinition, ScheduleInfo, ScheduleRun,
    ScheduleTarget,
};
use vessel_proto::{Creator, Delivered, ScheduleId, SessionId};
use vessel_store::{DataDir, Store, StoreConfig};

use crate::automation::schedules::rules::{Asker, Via};
use crate::automation::schedules::{
    Clock, FollowSender, Job, Made, Parts, Runner, Schedules, start,
};

/// A wall clock moved by hand.
#[derive(Debug, Clone)]
pub(super) struct HandClock(Arc<Mutex<DateTime<Utc>>>);

impl HandClock {
    pub(super) fn at(text: &str) -> Self {
        Self(Arc::new(Mutex::new(at(text))))
    }

    pub(super) fn set(&self, text: &str) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = at(text);
    }
}

impl Clock for HandClock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub(super) fn at(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .unwrap()
        .with_timezone(&Utc)
}

/// A runner that records each job and answers as told; `gate`, when set,
/// holds every run until the test lets it go.
#[derive(Debug)]
pub(super) struct FakeRunner {
    pub(super) jobs: mpsc::UnboundedSender<Job>,
    pub(super) answer: Mutex<Made>,
    pub(super) gate: Option<Arc<Semaphore>>,
    pub(super) running: Mutex<(usize, usize)>,
    /// Agents' sessions that have ended.
    pub(super) gone: Mutex<Vec<SessionId>>,
}

impl FakeRunner {
    pub(super) fn answering(made: Made) -> (Arc<Self>, mpsc::UnboundedReceiver<Job>) {
        let (jobs, received) = mpsc::unbounded_channel();
        let runner = Self {
            jobs,
            answer: Mutex::new(made),
            gate: None,
            running: Mutex::new((0, 0)),
            gone: Mutex::new(Vec::new()),
        };
        (Arc::new(runner), received)
    }

    pub(super) fn gated(gate: Arc<Semaphore>) -> (Arc<Self>, mpsc::UnboundedReceiver<Job>) {
        let (jobs, received) = mpsc::unbounded_channel();
        let runner = Self {
            jobs,
            answer: Mutex::new(prompted()),
            gate: Some(gate),
            running: Mutex::new((0, 0)),
            gone: Mutex::new(Vec::new()),
        };
        (Arc::new(runner), received)
    }

    /// The most runs that were made at once.
    pub(super) fn most_at_once(&self) -> usize {
        self.running
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .1
    }
}

impl Runner for FakeRunner {
    async fn make(&self, job: Job) -> Made {
        {
            let mut running = self.running.lock().unwrap_or_else(PoisonError::into_inner);
            running.0 = running.0.saturating_add(1);
            running.1 = running.1.max(running.0);
        }
        let _sent = self.jobs.send(job);
        if let Some(gate) = &self.gate {
            let _permit = gate.acquire().await.unwrap();
        }
        {
            let mut running = self.running.lock().unwrap_or_else(PoisonError::into_inner);
            running.0 = running.0.saturating_sub(1);
        }
        self.answer
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    async fn follow(&self, _job: Job, _session: SessionId, _follow: FollowSender) {}

    async fn settle(&self, run: &ScheduleRun) -> Option<RunOutcome> {
        matches!(run.outcome, RunOutcome::Making).then_some(RunOutcome::Interrupted)
    }

    fn agent_alive(&self, session: SessionId) -> bool {
        !self
            .gone
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(&session)
    }
}

pub(super) fn prompted() -> Made {
    Made::outcome(RunOutcome::Prompted {
        session: SessionId::random(),
        delivered: Delivered::Started,
    })
}

pub(super) fn every(seconds: u32, anchor: &str) -> Cadence {
    Cadence::Every {
        seconds: EverySeconds::try_from(seconds).unwrap(),
        anchor: Some(at(anchor)),
    }
}

pub(super) fn definition(cadence: Cadence) -> ScheduleDefinition {
    ScheduleDefinition {
        name: Some("check".parse().unwrap()),
        cadence,
        target: ScheduleTarget::Session {
            session: SessionId::random(),
        },
        prompt: "Check the build.".parse().unwrap(),
        catch_up: CatchUp::Once,
        max_runs: None,
        ends_at: None,
    }
}

pub(super) fn person() -> Asker {
    Asker {
        by: Creator::Owner,
        via: Via::Person,
    }
}

/// A scheduler over a fresh store.
pub(super) struct Scheduler {
    pub(super) schedules: Schedules,
    pub(super) store: Store,
    pub(super) ended: broadcast::Sender<SessionId>,
    pub(super) stop: CancellationToken,
    pub(super) _data: tempfile::TempDir,
}

pub(super) async fn store() -> (Store, tempfile::TempDir) {
    let temp = tempfile::tempdir().unwrap();
    let data = DataDir::new(temp.path());
    data.prepare().unwrap();
    let store = Store::open(&data, StoreConfig::default())
        .await
        .unwrap()
        .store;
    (store, temp)
}

pub(super) async fn scheduler_on(
    store: Store,
    data: tempfile::TempDir,
    runner: Arc<FakeRunner>,
    clock: HandClock,
) -> Scheduler {
    let (schedules, commands) = Schedules::channel();
    let (ended, ends) = broadcast::channel(16);
    let stop = CancellationToken::new();
    start(
        commands,
        Parts {
            runner,
            clock,
            store: store.clone(),
            tasks: TaskTracker::new(),
            stop: stop.clone(),
            ended: ends,
        },
    )
    .await;
    Scheduler {
        schedules,
        store,
        ended,
        stop,
        _data: data,
    }
}

pub(super) async fn scheduler(runner: Arc<FakeRunner>, clock: HandClock) -> Scheduler {
    let (store, data) = store().await;
    scheduler_on(store, data, runner, clock).await
}

/// A deadline that only turns a hang into a failure.
pub(super) async fn hang_guard<F: Future>(what: &str, future: F) -> F::Output {
    tokio::time::timeout(Duration::from_secs(20), future)
        .await
        .unwrap_or_else(|_hung| panic!("timed out waiting for {what}"))
}

impl Scheduler {
    /// The schedule once `until` holds of it, the scheduler asked again
    /// each time (every call wakes it, so it looks at the clock).
    pub(super) async fn until(
        &self,
        id: ScheduleId,
        until: impl Fn(&ScheduleInfo) -> bool,
    ) -> ScheduleInfo {
        hang_guard("a schedule's state", async {
            loop {
                let info = self.schedules.get(id).await.unwrap();
                if until(&info) {
                    return info;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
    }

    pub(super) async fn last_outcome(&self, id: ScheduleId, number: u32) -> ScheduleRun {
        self.until(id, |info| {
            info.last_run.as_ref().is_some_and(|run| {
                run.number.get() == number && !matches!(run.outcome, RunOutcome::Making)
            })
        })
        .await
        .last_run
        .unwrap()
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}
