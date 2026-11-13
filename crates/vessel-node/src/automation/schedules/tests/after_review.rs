//! What the code review of track D found, each held by a test: a skip
//! recorded above a run that still runs, at the next due time and at a
//! restart; `ends_at` bounding a catch-up; an agent's schedule that ends
//! for any reason going.

use super::*;

/// A runner whose runs start a session (each run's `previous` is what it
/// checks for overlap; the fake only records it).
fn started() -> Made {
    Made::outcome(RunOutcome::Started {
        session: SessionId::random(),
    })
}

#[tokio::test]
async fn a_skip_never_hides_the_run_that_still_runs() {
    let gate = Arc::new(Semaphore::new(0));
    let clock = HandClock::at("2026-01-01T10:00:00Z");
    let (runner, mut jobs) = FakeRunner::gated(Arc::clone(&gate));
    *runner.answer.lock().unwrap_or_else(PoisonError::into_inner) = started();
    let node = scheduler(runner, clock.clone()).await;
    let info = node
        .schedules
        .create(
            definition(every(60, "2026-01-01T10:00:00Z")),
            false,
            person(),
        )
        .await
        .unwrap();
    clock.set("2026-01-01T10:01:00Z");
    let _poke = node.schedules.list().await.unwrap();
    let first = hang_guard("the first run", jobs.recv()).await.unwrap();
    assert!(first.previous.is_none());
    // Due again while it is made: a skip, recorded as the newest run.
    clock.set("2026-01-01T10:02:00Z");
    let _skipped = node
        .until(info.id, |info| {
            info.last_run
                .as_ref()
                .is_some_and(|run| run.number.get() == 2)
        })
        .await;
    gate.add_permits(1);
    let _made = node.last_outcome(info.id, 2).await;
    hang_guard("the first run's outcome", async {
        loop {
            let runs = node.store.runs(info.id).await.unwrap();
            if runs
                .iter()
                .any(|run| run.number == RunNumber::FIRST && run.outcome != RunOutcome::Making)
            {
                return;
            }
            let _poke = node.schedules.list().await.unwrap();
        }
    })
    .await;
    // The next run checks the run that started a session, not the skip.
    clock.set("2026-01-01T10:03:00Z");
    gate.add_permits(1);
    let _poke = node.schedules.list().await.unwrap();
    let third = hang_guard("the third run", jobs.recv()).await.unwrap();
    let previous = third.previous.unwrap();
    assert_eq!(previous.number, RunNumber::FIRST);
    assert!(matches!(previous.outcome, RunOutcome::Started { .. }));
}

#[tokio::test]
async fn at_start_the_run_under_a_skip_is_settled() {
    let (store, data) = store().await;
    let making = ScheduleRun {
        number: RunNumber::FIRST,
        due_at: at("2026-01-01T10:10:00Z"),
        at: at("2026-01-01T10:10:00Z"),
        trigger: RunTrigger::Due,
        outcome: RunOutcome::Making,
    };
    let skip = ScheduleRun {
        number: RunNumber::FIRST.next().unwrap(),
        due_at: at("2026-01-01T10:20:00Z"),
        at: at("2026-01-01T10:20:00Z"),
        trigger: RunTrigger::Due,
        outcome: RunOutcome::Skipped {
            reason: SkipReason::PreviousRunning,
        },
    };
    let info = ScheduleInfo {
        id: ScheduleId::random(),
        definition: definition(every(600, "2026-01-01T10:00:00Z")),
        state: ScheduleState::Active {
            next_due: at("2026-01-01T10:30:00Z"),
        },
        runs: 0,
        last_due: Some(at("2026-01-01T10:20:00Z")),
        last_run: Some(skip.clone()),
        defined_by: Creator::Owner,
        created_at: at("2026-01-01T10:00:00Z"),
        updated_at: at("2026-01-01T10:00:00Z"),
        revision: 0,
    };
    store
        .put_schedule(info.clone(), Some(making))
        .await
        .unwrap();
    store.put_run(info.id, skip.clone()).await.unwrap();
    let clock = HandClock::at("2026-01-01T10:25:00Z");
    let (runner, mut jobs) = FakeRunner::answering(prompted());
    let node = scheduler_on(store, data, runner, clock).await;
    let runs = node.store.runs(info.id).await.unwrap();
    let first = runs
        .iter()
        .find(|run| run.number == RunNumber::FIRST)
        .unwrap();
    assert_eq!(first.outcome, RunOutcome::Interrupted);
    // The skip stays the newest record; nothing was made.
    let kept = node.schedules.get(info.id).await.unwrap();
    assert_eq!(kept.last_run, Some(skip));
    assert!(jobs.try_recv().is_err());
}

#[tokio::test]
async fn ends_at_bounds_what_a_stopped_node_catches_up() {
    let (store, data) = store().await;
    let mut ending = definition(every(1800, "2026-01-01T00:00:00Z"));
    ending.ends_at = Some(at("2026-01-01T10:00:00Z"));
    let info = ScheduleInfo {
        id: ScheduleId::random(),
        definition: ending,
        state: ScheduleState::Active {
            next_due: at("2026-01-01T08:00:00Z"),
        },
        runs: 0,
        last_due: None,
        last_run: None,
        defined_by: Creator::Owner,
        created_at: at("2026-01-01T00:00:00Z"),
        updated_at: at("2026-01-01T00:00:00Z"),
        revision: 0,
    };
    store.put_schedule(info.clone(), None).await.unwrap();
    // Stopped from before 08:00 until 12:00.
    let clock = HandClock::at("2026-01-01T12:00:00Z");
    let (runner, mut jobs) = FakeRunner::answering(prompted());
    let node = scheduler_on(store, data, runner, clock).await;
    let job = hang_guard("the catch-up", jobs.recv()).await.unwrap();
    // 08:00 to 10:00 every half hour: five, the last at `ends_at`.
    assert_eq!(job.run.due_at, at("2026-01-01T10:00:00Z"));
    assert_eq!(job.run.trigger, RunTrigger::CaughtUp { missed: 5 });
    let ended = node.until(info.id, |info| info.runs == 1).await;
    assert!(matches!(
        ended.state,
        ScheduleState::Ended {
            reason: EndReason::Expired,
            ..
        }
    ));
    assert!(jobs.try_recv().is_err());
}

#[tokio::test]
async fn an_agents_schedule_that_ends_by_its_runs_goes() {
    let clock = HandClock::at("2026-01-01T10:00:00Z");
    let (runner, mut jobs) = FakeRunner::answering(prompted());
    let node = scheduler(runner, clock.clone()).await;
    let session = SessionId::random();
    let agent = Asker {
        by: Creator::Agent {
            node: NodeId::random(),
            session,
        },
        via: Via::Agent,
    };
    let mut once = definition(every(60, "2026-01-01T10:00:00Z"));
    once.target = ScheduleTarget::Session { session };
    once.max_runs = Some(MaxRuns::try_from(1).unwrap());
    let info = node.schedules.create(once, false, agent).await.unwrap();
    clock.set("2026-01-01T10:01:00Z");
    let _poke = node.schedules.list().await.unwrap();
    let _made = hang_guard("its one run", jobs.recv()).await.unwrap();
    hang_guard("the ended schedule to go", async {
        while node.schedules.get(info.id).await.is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(node.store.runs(info.id).await.unwrap().is_empty());
}
