//! The scheduler on a clock the tests move and with a runner that records
//! what it is asked (ADR 0033's edges: a stopped node, a clock stepped
//! either way, two due at once, an edit while due, a run still being
//! made, a definer gone, the bound of runs at once). Nothing sleeps: the
//! clock is moved by hand and the scheduler is woken by a call; the
//! timeouts only turn a hang into a failure.

use std::sync::{Arc, PoisonError};

use tokio::sync::{Semaphore, broadcast, mpsc};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::automation::{
    Cadence, CatchUp, EndReason, EverySeconds, MaxRuns, RunNumber, RunOutcome, RunTrigger,
    ScheduleDefinition, ScheduleInfo, ScheduleRun, ScheduleState, ScheduleTarget, SkipReason,
};
use vessel_proto::{Creator, NodeId, ScheduleId, SessionId};

use super::rules::{Asker, Via};
use super::{FollowSender, Job, Made, Parts, Runner, Schedules, start};

mod harness;

use harness::*;

#[tokio::test]
async fn a_due_schedule_runs_once_and_moves_on() {
    let clock = HandClock::at("2026-01-01T10:00:00Z");
    let (runner, mut jobs) = FakeRunner::answering(prompted());
    let node = scheduler(Arc::clone(&runner), clock.clone()).await;
    let info = node
        .schedules
        .create(
            definition(every(300, "2026-01-01T10:00:00Z")),
            false,
            person(),
        )
        .await
        .unwrap();
    assert_eq!(
        info.state,
        ScheduleState::Active {
            next_due: at("2026-01-01T10:05:00Z")
        }
    );
    // Not yet due: nothing runs.
    clock.set("2026-01-01T10:04:59Z");
    let quiet = node.schedules.get(info.id).await.unwrap();
    assert_eq!(quiet.last_run, None);

    clock.set("2026-01-01T10:05:00.010Z");
    let job = hang_guard("the run", jobs.recv()).await.unwrap();
    assert_eq!(job.run.trigger, RunTrigger::Due);
    assert_eq!(job.run.due_at, at("2026-01-01T10:05:00Z"));
    let run = node.last_outcome(info.id, 1).await;
    assert!(matches!(run.outcome, RunOutcome::Prompted { .. }));
    let after = node.schedules.get(info.id).await.unwrap();
    assert_eq!(after.runs, 1);
    assert_eq!(after.last_due, Some(at("2026-01-01T10:05:00Z")));
    assert_eq!(
        after.state,
        ScheduleState::Active {
            next_due: at("2026-01-01T10:10:00Z")
        }
    );
    // The run is kept by the store too.
    let runs = node.store.runs(info.id).await.unwrap();
    assert_eq!(runs.len(), 1);
}

#[tokio::test]
async fn two_due_at_once_both_run() {
    let clock = HandClock::at("2026-01-01T10:00:00Z");
    let (runner, mut jobs) = FakeRunner::answering(prompted());
    let node = scheduler(runner, clock.clone()).await;
    let every = node
        .schedules
        .create(
            definition(every(180, "2026-01-01T10:00:00Z")),
            false,
            person(),
        )
        .await
        .unwrap();
    let cron = Cadence::Cron {
        expr: "3 10 * * *".parse().unwrap(),
        zone: "UTC".parse().unwrap(),
    };
    let daily = node
        .schedules
        .create(definition(cron), false, person())
        .await
        .unwrap();
    // Both are due at 10:03:00. Their runs are made by tasks of their own,
    // so they arrive in either order; the order they were taken in is the
    // book's (its own test).
    clock.set("2026-01-01T10:03:00Z");
    let _poke = node.schedules.list().await.unwrap();
    let mut ran = Vec::new();
    for _ in 0..2 {
        let job = hang_guard("a run", jobs.recv()).await.unwrap();
        assert_eq!(job.run.trigger, RunTrigger::Due);
        assert_eq!(job.run.due_at, at("2026-01-01T10:03:00Z"));
        ran.push(job.schedule);
    }
    ran.sort();
    let mut both = vec![every.id, daily.id];
    both.sort();
    assert_eq!(ran, both);
}

#[tokio::test]
async fn due_times_missed_while_stopped_make_one_run_or_one_record() {
    let (store, data) = store().await;
    // A schedule kept by a node that stopped at 10:00 and starts at 13:02.
    let mut once = ScheduleInfo {
        id: ScheduleId::random(),
        definition: definition(every(300, "2026-01-01T00:00:00Z")),
        state: ScheduleState::Active {
            next_due: at("2026-01-01T10:00:00Z"),
        },
        runs: 0,
        last_due: None,
        last_run: None,
        defined_by: Creator::Owner,
        created_at: at("2026-01-01T00:00:00Z"),
        updated_at: at("2026-01-01T00:00:00Z"),
        revision: 0,
    };
    let mut skip = ScheduleInfo {
        id: ScheduleId::random(),
        ..once.clone()
    };
    skip.definition.catch_up = CatchUp::Skip;
    store.put_schedule(once.clone(), None).await.unwrap();
    store.put_schedule(skip.clone(), None).await.unwrap();
    let clock = HandClock::at("2026-01-01T13:02:00Z");
    let (runner, mut jobs) = FakeRunner::answering(prompted());
    let node = scheduler_on(store, data, runner, clock).await;
    let job = hang_guard("the catch-up", jobs.recv()).await.unwrap();
    assert_eq!(job.schedule, once.id);
    assert_eq!(job.run.trigger, RunTrigger::CaughtUp { missed: 37 });
    assert_eq!(job.run.due_at, at("2026-01-01T13:00:00Z"));
    once = node.until(once.id, |info| info.runs == 1).await;
    assert_eq!(
        once.state,
        ScheduleState::Active {
            next_due: at("2026-01-01T13:05:00Z")
        }
    );
    skip = node.schedules.get(skip.id).await.unwrap();
    let record = skip.last_run.unwrap();
    assert_eq!(
        record.outcome,
        RunOutcome::Skipped {
            reason: SkipReason::Missed { count: 37 }
        }
    );
    assert_eq!(skip.runs, 0);
    // Nothing else ran.
    assert!(jobs.try_recv().is_err());
}

#[tokio::test]
async fn a_clock_stepped_back_never_runs_a_due_time_twice() {
    let clock = HandClock::at("2026-01-01T09:59:00Z");
    let (runner, mut jobs) = FakeRunner::answering(prompted());
    let node = scheduler(runner, clock.clone()).await;
    let hourly = Cadence::Cron {
        expr: "0 * * * *".parse().unwrap(),
        zone: "UTC".parse().unwrap(),
    };
    let info = node
        .schedules
        .create(definition(hourly), false, person())
        .await
        .unwrap();
    clock.set("2026-01-01T10:00:01Z");
    let _poke = node.schedules.list().await.unwrap();
    let _ten = hang_guard("10:00's run", jobs.recv()).await.unwrap();
    let _done = node.last_outcome(info.id, 1).await;
    // Back an hour: 10:00 is not due again, and 11:00 waits for the clock.
    clock.set("2026-01-01T09:05:00Z");
    let back = node.schedules.get(info.id).await.unwrap();
    assert_eq!(
        back.state,
        ScheduleState::Active {
            next_due: at("2026-01-01T11:00:00Z")
        }
    );
    clock.set("2026-01-01T10:30:00Z");
    let _poke = node.schedules.list().await.unwrap();
    let _again = node.schedules.list().await.unwrap();
    assert!(jobs.try_recv().is_err());
    clock.set("2026-01-01T11:00:00Z");
    let _poke = node.schedules.list().await.unwrap();
    let eleven = hang_guard("11:00's run", jobs.recv()).await.unwrap();
    assert_eq!(eleven.run.due_at, at("2026-01-01T11:00:00Z"));
    assert_eq!(eleven.run.trigger, RunTrigger::Due);
}

#[tokio::test]
async fn a_run_made_while_the_clock_was_days_ahead_does_not_silence_the_schedule() {
    let clock = HandClock::at("2030-06-01T10:00:00Z");
    let (runner, mut jobs) = FakeRunner::answering(prompted());
    let node = scheduler(runner, clock.clone()).await;
    let info = node
        .schedules
        .create(
            definition(every(3600, "2026-01-01T00:00:00Z")),
            false,
            person(),
        )
        .await
        .unwrap();
    clock.set("2030-06-01T11:00:00Z");
    let _poke = node.schedules.list().await.unwrap();
    let _ran = hang_guard("the run in 2030", jobs.recv()).await.unwrap();
    let _done = node.last_outcome(info.id, 1).await;
    // The clock is corrected back to 2026: the next due time comes from
    // now, not from 2030.
    clock.set("2026-01-01T10:30:00Z");
    let corrected = node
        .until(info.id, |info| {
            info.state
                == ScheduleState::Active {
                    next_due: at("2026-01-01T11:00:00Z"),
                }
        })
        .await;
    assert_eq!(corrected.runs, 1);
}

#[tokio::test]
async fn a_clock_stepped_forward_counts_what_it_skipped_as_missed() {
    let clock = HandClock::at("2026-01-01T10:00:00Z");
    let (runner, mut jobs) = FakeRunner::answering(prompted());
    let node = scheduler(runner, clock.clone()).await;
    let info = node
        .schedules
        .create(
            definition(every(600, "2026-01-01T10:00:00Z")),
            false,
            person(),
        )
        .await
        .unwrap();
    clock.set("2026-01-01T11:05:00Z");
    let _poke = node.schedules.list().await.unwrap();
    let job = hang_guard("the catch-up", jobs.recv()).await.unwrap();
    assert_eq!(job.schedule, info.id);
    assert_eq!(job.run.trigger, RunTrigger::CaughtUp { missed: 6 });
    assert_eq!(job.run.due_at, at("2026-01-01T11:00:00Z"));
}

#[tokio::test]
async fn an_edit_of_the_cadence_counts_from_the_edit() {
    let clock = HandClock::at("2026-01-01T10:00:00Z");
    let (runner, mut jobs) = FakeRunner::answering(prompted());
    let node = scheduler(runner, clock.clone()).await;
    let info = node
        .schedules
        .create(
            definition(every(600, "2026-01-01T10:00:00Z")),
            false,
            person(),
        )
        .await
        .unwrap();
    // Edited a moment before it was due: the new cadence counts from now,
    // and the old due time is not run.
    clock.set("2026-01-01T10:09:59Z");
    let mut changed = info.definition.clone();
    changed.cadence = Cadence::Every {
        seconds: EverySeconds::try_from(3600).unwrap(),
        anchor: None,
    };
    let edited = node
        .schedules
        .update(info.id, info.revision, changed.clone(), person())
        .await
        .unwrap();
    assert_eq!(
        edited.state,
        ScheduleState::Active {
            next_due: at("2026-01-01T11:09:59Z")
        }
    );
    assert_eq!(edited.revision, 1);
    // A second edit from the old revision is refused.
    let stale = node
        .schedules
        .update(info.id, info.revision, changed, person())
        .await
        .unwrap_err();
    assert!(stale.to_string().contains("changed meanwhile"), "{stale}");
    clock.set("2026-01-01T10:10:30Z");
    let _poke = node.schedules.list().await.unwrap();
    let _again = node.schedules.list().await.unwrap();
    assert!(jobs.try_recv().is_err());
}

#[tokio::test]
async fn a_due_time_while_a_run_is_being_made_is_skipped() {
    let gate = Arc::new(Semaphore::new(0));
    let clock = HandClock::at("2026-01-01T10:00:00Z");
    let (runner, mut jobs) = FakeRunner::gated(Arc::clone(&gate));
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
    let _held = hang_guard("the first run", jobs.recv()).await.unwrap();
    // Due again while the first is still being made.
    clock.set("2026-01-01T10:02:00Z");
    let skipped = node
        .until(info.id, |info| {
            info.last_run
                .as_ref()
                .is_some_and(|run| run.number.get() == 2)
        })
        .await;
    assert_eq!(
        skipped.last_run.unwrap().outcome,
        RunOutcome::Skipped {
            reason: SkipReason::PreviousRunning
        }
    );
    // Asked to run now meanwhile: refused.
    let busy = node.schedules.run(info.id, person()).await.unwrap_err();
    assert!(busy.to_string().contains("being made"), "{busy}");
    gate.add_permits(1);
    let first = hang_guard("the first run's record", async {
        loop {
            let runs = node.store.runs(info.id).await.unwrap();
            if let Some(run) = runs.iter().find(|run| run.number == RunNumber::FIRST)
                && !matches!(run.outcome, RunOutcome::Making)
            {
                return run.clone();
            }
            let _poke = node.schedules.list().await.unwrap();
        }
    })
    .await;
    assert!(matches!(first.outcome, RunOutcome::Prompted { .. }));
}

#[tokio::test]
async fn run_now_answers_with_its_outcome_and_keeps_the_cadence() {
    let clock = HandClock::at("2026-01-01T10:00:00Z");
    let (runner, _jobs) = FakeRunner::answering(prompted());
    let node = scheduler(runner, clock.clone()).await;
    let mut paused_one = definition(every(3600, "2026-01-01T10:00:00Z"));
    paused_one.max_runs = Some(MaxRuns::try_from(2).unwrap());
    let info = node
        .schedules
        .create(paused_one, true, person())
        .await
        .unwrap();
    let run = node.schedules.run(info.id, person()).await.unwrap();
    assert!(matches!(run.outcome, RunOutcome::Prompted { .. }));
    assert_eq!(run.trigger, RunTrigger::Asked { by: Creator::Owner });
    let after = node.schedules.get(info.id).await.unwrap();
    assert!(matches!(after.state, ScheduleState::Paused { .. }));
    assert_eq!(after.runs, 1);
    // The second run reaches `max_runs`: the schedule ends.
    let _second = node.schedules.run(info.id, person()).await.unwrap();
    let ended = node.schedules.get(info.id).await.unwrap();
    assert!(matches!(
        ended.state,
        ScheduleState::Ended {
            reason: EndReason::MaxRuns,
            ..
        }
    ));
    let refused = node.schedules.run(info.id, person()).await.unwrap_err();
    assert!(refused.to_string().contains("ended"), "{refused}");
}

#[tokio::test]
async fn ends_at_ends_it_and_resume_counts_from_now() {
    let clock = HandClock::at("2026-01-01T10:00:00Z");
    let (runner, _jobs) = FakeRunner::answering(prompted());
    let node = scheduler(runner, clock.clone()).await;
    let mut ending = definition(every(600, "2026-01-01T10:00:00Z"));
    ending.ends_at = Some(at("2026-01-01T10:25:00Z"));
    let info = node
        .schedules
        .create(ending, false, person())
        .await
        .unwrap();
    let paused = node.schedules.pause(info.id, person()).await.unwrap();
    assert!(matches!(paused.state, ScheduleState::Paused { .. }));
    clock.set("2026-01-01T10:12:00Z");
    let resumed = node.schedules.resume(info.id, person()).await.unwrap();
    // The anchor's phase is kept: 10:20, not 10:22.
    assert_eq!(
        resumed.state,
        ScheduleState::Active {
            next_due: at("2026-01-01T10:20:00Z")
        }
    );
    clock.set("2026-01-01T10:20:00Z");
    let ended = node
        .until(info.id, |info| {
            matches!(info.state, ScheduleState::Ended { .. })
        })
        .await;
    assert!(matches!(
        ended.state,
        ScheduleState::Ended {
            reason: EndReason::Expired,
            ..
        }
    ));
}

#[tokio::test]
async fn a_definer_gone_ends_the_schedule_and_an_agents_goes_with_its_session() {
    let clock = HandClock::at("2026-01-01T10:00:00Z");
    let gone = Made {
        outcome: RunOutcome::Interrupted,
        end: Some(EndReason::CreatorGone),
    };
    let (runner, _jobs) = FakeRunner::answering(gone);
    let node = scheduler(runner, clock.clone()).await;
    let info = node
        .schedules
        .create(
            definition(every(600, "2026-01-01T10:00:00Z")),
            false,
            person(),
        )
        .await
        .unwrap();
    let _run = node.schedules.run(info.id, person()).await.unwrap();
    let ended = node.schedules.get(info.id).await.unwrap();
    assert!(matches!(
        ended.state,
        ScheduleState::Ended {
            reason: EndReason::CreatorGone,
            ..
        }
    ));

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
    let _sent = node.ended.send(session).unwrap();
    hang_guard("the agent's schedule to go", async {
        while node.schedules.get(heartbeat.id).await.is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(node.store.runs(heartbeat.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn at_start_a_run_being_made_is_interrupted_and_agents_schedules_go() {
    let (store, data) = store().await;
    let mut making = ScheduleInfo {
        id: ScheduleId::random(),
        definition: definition(every(600, "2026-01-01T10:00:00Z")),
        state: ScheduleState::Active {
            next_due: at("2026-01-01T10:20:00Z"),
        },
        runs: 0,
        last_due: Some(at("2026-01-01T10:10:00Z")),
        last_run: Some(ScheduleRun {
            number: RunNumber::FIRST,
            due_at: at("2026-01-01T10:10:00Z"),
            at: at("2026-01-01T10:10:00Z"),
            trigger: RunTrigger::Due,
            outcome: RunOutcome::Making,
        }),
        defined_by: Creator::Owner,
        created_at: at("2026-01-01T10:00:00Z"),
        updated_at: at("2026-01-01T10:00:00Z"),
        revision: 0,
    };
    let agents = ScheduleInfo {
        id: ScheduleId::random(),
        defined_by: Creator::Agent {
            node: NodeId::random(),
            session: SessionId::random(),
        },
        last_run: None,
        ..making.clone()
    };
    store
        .put_schedule(making.clone(), making.last_run.clone())
        .await
        .unwrap();
    store.put_schedule(agents.clone(), None).await.unwrap();
    let clock = HandClock::at("2026-01-01T10:15:00Z");
    let (runner, mut jobs) = FakeRunner::answering(prompted());
    let node = scheduler_on(store, data, runner, clock).await;
    making = node.schedules.get(making.id).await.unwrap();
    assert_eq!(making.last_run.unwrap().outcome, RunOutcome::Interrupted);
    assert!(node.schedules.get(agents.id).await.is_err());
    // Not made again.
    let _poke = node.schedules.list().await.unwrap();
    assert!(jobs.try_recv().is_err());
}

#[tokio::test]
async fn at_most_four_runs_are_made_at_once() {
    let gate = Arc::new(Semaphore::new(0));
    let clock = HandClock::at("2026-01-01T10:00:00Z");
    let (runner, mut jobs) = FakeRunner::gated(Arc::clone(&gate));
    let node = scheduler(Arc::clone(&runner), clock.clone()).await;
    let mut ids = Vec::new();
    for _ in 0..6 {
        let info = node
            .schedules
            .create(
                definition(every(60, "2026-01-01T10:00:00Z")),
                false,
                person(),
            )
            .await
            .unwrap();
        ids.push(info.id);
    }
    clock.set("2026-01-01T10:01:00Z");
    let _poke = node.schedules.list().await.unwrap();
    for _ in 0..4 {
        let _job = hang_guard("a run", jobs.recv()).await.unwrap();
    }
    // Four are held; the other two wait for a slot.
    let _poke = node.schedules.list().await.unwrap();
    assert!(jobs.try_recv().is_err());
    gate.add_permits(6);
    for _ in 0..2 {
        let _job = hang_guard("a waiting run", jobs.recv()).await.unwrap();
    }
    assert_eq!(runner.most_at_once(), 4);
}

#[cfg(test)]
mod after_review;
#[cfg(test)]
mod joined;

mod ended_edit;
