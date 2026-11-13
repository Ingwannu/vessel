//! What the join of track D settled in the scheduler ("Integration 3
//! (node)"): a session started again takes none of the schedules its
//! agent made in its last run, even when the scheduler never heard that
//! run end; runs waiting for room go oldest due first; an asked run with
//! no room in 30 s (on the clock the test moves) is `unavailable`; a
//! catch-up starts at most eight sessions in their first turn.

use super::*;

#[tokio::test]
async fn a_session_started_again_takes_none_of_its_last_runs_schedules() {
    let clock = HandClock::at("2026-01-01T10:00:00Z");
    let (runner, _jobs) = FakeRunner::answering(prompted());
    let node = scheduler(runner, clock).await;
    let session = SessionId::random();
    let agent = Asker {
        by: Creator::Agent {
            node: NodeId::random(),
            session,
        },
        via: Via::Agent,
    };
    let heartbeat = node
        .schedules
        .create(definition(every(600, "2026-01-01T10:00:00Z")), false, agent)
        .await
        .unwrap();
    let persons = node
        .schedules
        .create(
            definition(every(600, "2026-01-01T10:00:00Z")),
            false,
            person(),
        )
        .await
        .unwrap();
    // No end was broadcast (as when the scheduler fell behind the ends'
    // channel): the start again alone removes them, before it answers.
    node.schedules.starting_again(session).await.unwrap();
    assert!(node.schedules.get(heartbeat.id).await.is_err());
    assert!(node.schedules.get(persons.id).await.is_ok());
    assert!(node.store.runs(heartbeat.id).await.unwrap().is_empty());
}

/// An every-two-minutes schedule first due `seconds` past 10:00, made at
/// 09:59 (the clock the tests below start at).
fn due_at_second(seconds: u32) -> ScheduleDefinition {
    definition(every(120, &format!("2026-01-01T09:58:{seconds:02}Z")))
}

/// A schedule starting a new session, first due `seconds` past 10:00.
fn new_session_at_second(seconds: u32) -> ScheduleDefinition {
    ScheduleDefinition {
        target: ScheduleTarget::NewSession {
            spec: Box::new(vessel_proto::SessionSpec::new(
                vessel_proto::HarnessKind::ClaudeCode,
                vessel_proto::SessionMode::Structured {
                    model: None,
                    resume: None,
                    options: Vec::new(),
                },
            )),
            after: vessel_proto::automation::AfterRun::Archive,
        },
        ..due_at_second(seconds)
    }
}

/// Runs waiting for a slot go oldest due first (the contract's order),
/// whichever task the runtime would have run first.
#[tokio::test]
async fn runs_waiting_for_a_slot_go_oldest_due_first() {
    let gate = Arc::new(Semaphore::new(0));
    let clock = HandClock::at("2026-01-01T09:59:00Z");
    let (runner, mut jobs) = FakeRunner::gated(Arc::clone(&gate));
    let node = scheduler(runner, clock.clone()).await;
    for seconds in [50, 10, 40, 0, 30, 20] {
        let _made = node
            .schedules
            .create(due_at_second(seconds), false, person())
            .await
            .unwrap();
    }
    clock.set("2026-01-01T10:01:00Z");
    let _poke = node.schedules.list().await.unwrap();
    let mut first = Vec::new();
    for _ in 0..4 {
        first.push(hang_guard("a run", jobs.recv()).await.unwrap().run.due_at);
    }
    first.sort();
    let second = |seconds: u32| at(&format!("2026-01-01T10:00:{seconds:02}Z"));
    assert_eq!(first, [second(0), second(10), second(20), second(30)]);
    // One finishes at a time: the two waiting go in due order.
    for seconds in [40, 50] {
        gate.add_permits(1);
        let job = hang_guard("a waiting run", jobs.recv()).await.unwrap();
        assert_eq!(job.run.due_at, second(seconds));
    }
    gate.add_permits(2);
}

/// `schedule.run` that finds no slot for 30 s on the scheduler's clock is
/// answered `unavailable` and recorded `failed`; nothing waits 30 s here.
#[tokio::test]
async fn a_run_asked_for_with_no_room_in_thirty_seconds_is_unavailable() {
    let gate = Arc::new(Semaphore::new(0));
    let clock = HandClock::at("2026-01-01T09:59:00Z");
    let (runner, mut jobs) = FakeRunner::gated(Arc::clone(&gate));
    let node = scheduler(runner, clock.clone()).await;
    for seconds in [0, 1, 2, 3] {
        let _made = node
            .schedules
            .create(due_at_second(seconds), false, person())
            .await
            .unwrap();
    }
    let asked = node
        .schedules
        .create(
            definition(every(3600, "2026-01-01T09:59:00Z")),
            false,
            person(),
        )
        .await
        .unwrap();
    clock.set("2026-01-01T10:00:30Z");
    let _poke = node.schedules.list().await.unwrap();
    for _ in 0..4 {
        let _held = hang_guard("a run", jobs.recv()).await.unwrap();
    }
    let schedules = node.schedules.clone();
    let answer = tokio::spawn(async move { schedules.run(asked.id, person()).await });
    let _waiting = node
        .until(asked.id, |info| {
            info.last_run
                .as_ref()
                .is_some_and(|run| matches!(run.outcome, RunOutcome::Making))
        })
        .await;
    clock.set("2026-01-01T10:00:59Z");
    let _poke = node.schedules.list().await.unwrap();
    assert!(!answer.is_finished(), "29 s: it still waits");
    clock.set("2026-01-01T10:01:00Z");
    let _poke = node.schedules.list().await.unwrap();
    let answered = hang_guard("the asked run's answer", answer).await.unwrap();
    assert!(
        matches!(
            answered,
            Err(crate::automation::error::AutomationError::Unavailable(_))
        ),
        "{answered:?}"
    );
    let run = node.last_outcome(asked.id, 1).await;
    assert!(
        matches!(&run.outcome, RunOutcome::Failed { error, session: None }
            if error.code == vessel_proto::automation::RunErrorCode::Unavailable),
        "{run:?}"
    );
    assert!(jobs.try_recv().is_err(), "it was never made");
    gate.add_permits(4);
}

/// A runner whose runs start a session at once and whose sessions stay in
/// their first turn until `turns` lets them go; `asks` makes each wait for
/// a person first.
struct TurnRunner {
    jobs: mpsc::UnboundedSender<Job>,
    turns: Arc<Semaphore>,
    asks: bool,
}

impl Runner for TurnRunner {
    async fn make(&self, job: Job) -> Made {
        let _sent = self.jobs.send(job);
        Made::outcome(RunOutcome::Started {
            session: SessionId::random(),
        })
    }

    async fn follow(&self, job: Job, session: SessionId, follow: FollowSender) {
        if self.asks {
            let run = ScheduleRun {
                outcome: RunOutcome::Waiting { session },
                ..job.run
            };
            let _sent = follow
                .send(crate::automation::schedules::Followed {
                    schedule: job.schedule,
                    run,
                })
                .await;
        }
        let _turn = self.turns.acquire().await.unwrap();
    }

    async fn settle(&self, _run: &ScheduleRun) -> Option<RunOutcome> {
        None
    }
}

async fn turn_scheduler(
    asks: bool,
    clock: HandClock,
) -> (Scheduler, mpsc::UnboundedReceiver<Job>, Arc<Semaphore>) {
    let (jobs, received) = mpsc::unbounded_channel();
    let turns = Arc::new(Semaphore::new(0));
    let runner = Arc::new(TurnRunner {
        jobs,
        turns: Arc::clone(&turns),
        asks,
    });
    let (store, data) = store().await;
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
    let node = Scheduler {
        schedules,
        store,
        ended,
        stop,
        _data: data,
    };
    (node, received, turns)
}

/// After downtime every schedule of new sessions is due at once: at most
/// eight of their sessions are in their first turn together; the ninth
/// starts when one of those turns ends (the join's review, finding 12).
#[tokio::test]
async fn a_catch_up_starts_at_most_eight_sessions_in_their_first_turn() {
    let clock = HandClock::at("2026-01-01T09:59:00Z");
    let (node, mut jobs, turns) = turn_scheduler(false, clock.clone()).await;
    for seconds in 0..9 {
        let _made = node
            .schedules
            .create(new_session_at_second(seconds), false, person())
            .await
            .unwrap();
    }
    clock.set("2026-01-01T10:01:00Z");
    let _poke = node.schedules.list().await.unwrap();
    for _ in 0..8 {
        let _started = hang_guard("a session started", jobs.recv()).await.unwrap();
    }
    let _poke = node.schedules.list().await.unwrap();
    assert!(jobs.try_recv().is_err(), "eight first turns run");
    turns.add_permits(1);
    let ninth = hang_guard("the ninth", jobs.recv()).await.unwrap();
    assert_eq!(ninth.run.due_at, at("2026-01-01T10:00:08Z"));
    turns.add_permits(8);
}

/// A first turn waiting for a person holds no room: nine sessions that
/// each ask a permission all start.
#[tokio::test]
async fn a_session_waiting_for_a_person_gives_its_first_turn_back() {
    let clock = HandClock::at("2026-01-01T09:59:00Z");
    let (node, mut jobs, turns) = turn_scheduler(true, clock.clone()).await;
    for seconds in 0..9 {
        let _made = node
            .schedules
            .create(new_session_at_second(seconds), false, person())
            .await
            .unwrap();
    }
    clock.set("2026-01-01T10:01:00Z");
    let _poke = node.schedules.list().await.unwrap();
    for _ in 0..9 {
        let _started = hang_guard("a session started", jobs.recv()).await.unwrap();
    }
    turns.add_permits(9);
}
