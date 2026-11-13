//! Protocol 11's schedules through a real node (milestone 11, track D,
//! ADR 0033): a run that starts a session, follows its first turn and
//! archives it; a run through an existing session's queue, never piling up
//! and never starting a session a person stopped; a target gone; who may
//! make and run what; schedules kept across a restart. Runs are asked for
//! (`schedule.run`): the scheduler's timing is tested on a clock the tests
//! move (`automation::schedules::tests`), not by waiting here.

use vessel_client::NodeClient;
use vessel_proto::automation::{
    AfterRun, Cadence, CatchUp, EndReason, EverySeconds, RunEnd, RunOutcome, RunTrigger,
    ScheduleDefinition, ScheduleInfo, ScheduleState, ScheduleTarget, SkipReason,
};
use vessel_proto::{
    AgentEvent, ContentBlock, Creator, Delivered, Delivery, ErrorCode, EventSeq, QueueChange,
    ScheduleId, SessionEvent, SessionId, SessionInput, StopReason,
};

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{Fixture, read_until, status_of, structured_spec, within};

fn definition(target: ScheduleTarget, prompt: &str) -> ScheduleDefinition {
    ScheduleDefinition {
        name: Some("nightly check".parse().unwrap()),
        // An hour: it is never due while a test runs.
        cadence: Cadence::Every {
            seconds: EverySeconds::try_from(3600).unwrap(),
            anchor: None,
        },
        target,
        prompt: prompt.parse().unwrap(),
        catch_up: CatchUp::Once,
        max_runs: None,
        ends_at: None,
    }
}

fn text(prompt: &str) -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        text: prompt.to_owned(),
    }]
}

/// The schedule once its last run is no longer `started` or `waiting`:
/// the session it follows has ended, so the runner has sent its last word
/// (the scheduler keeps what runs came to before it answers a call).
async fn finished(client: &NodeClient, schedule: ScheduleId) -> ScheduleInfo {
    client.schedule(schedule).await.unwrap()
}

/// Read `session`'s log from the start until its final status.
async fn until_ended(client: &NodeClient, session: SessionId) -> Vec<vessel_proto::EventRecord> {
    let mut stream = client.follow(session, EventSeq::ZERO).await.unwrap();
    let mut seen = Vec::new();
    read_until(&mut stream, &mut seen, |record| {
        status_of(record).is_some_and(|status| !status.is_live())
    })
    .await;
    seen
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_starts_a_session_follows_its_turn_and_archives_it() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let target = ScheduleTarget::NewSession {
            spec: Box::new(structured_spec(&fixture, None)),
            after: AfterRun::Archive,
        };
        let schedule = client
            .create_schedule(definition(target, "plain"), false)
            .await
            .unwrap();
        assert_eq!(schedule.defined_by, Creator::Owner);
        let run = client.run_schedule(schedule.id).await.unwrap();
        assert_eq!(run.trigger, RunTrigger::Asked { by: Creator::Owner });
        let RunOutcome::Started { session } = run.outcome else {
            panic!("a new session's run starts one: {run:?}");
        };
        let seen = until_ended(&client, session).await;
        let notice = seen.iter().any(|record| {
            matches!(&record.event, SessionEvent::Agent { event: AgentEvent::HarnessLog { line } }
                if line.contains("started by schedule \"nightly check\" (run 1)"))
        });
        assert!(notice, "the log says which schedule started it");
        let info = client.session(session).await.unwrap();
        assert!(info.marks.archived_at.is_some(), "archived after its turn");
        assert_eq!(
            info.title.as_ref().map(vessel_proto::SessionTitle::as_str),
            Some("nightly check")
        );
        let after = finished(&client, schedule.id).await;
        assert_eq!(after.runs, 1);
        assert_eq!(
            after.last_run.unwrap().outcome,
            RunOutcome::Finished {
                session,
                end: RunEnd::Turn {
                    reason: StopReason::EndTurn
                }
            }
        );
        let runs = client.schedule_runs(schedule.id).await.unwrap();
        assert_eq!(runs.len(), 1);
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_waits_its_turn_in_a_queue_and_never_piles_up() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let busy = structured(&client, &fixture).await;
        // A turn that runs until it is told to stop.
        let _started = client
            .prompt(busy.id, text("wait"), Delivery::Start)
            .await
            .unwrap();
        let target = ScheduleTarget::Session { session: busy.id };
        let schedule = client
            .create_schedule(definition(target, "plain"), true)
            .await
            .unwrap();
        let queued = client.run_schedule(schedule.id).await.unwrap();
        let RunOutcome::Prompted {
            delivered: Delivered::Queued { entry },
            ..
        } = queued.outcome
        else {
            panic!("a busy session queues the prompt: {queued:?}");
        };
        // The previous prompt still waits: nothing more is queued.
        let again = client.run_schedule(schedule.id).await.unwrap();
        assert_eq!(
            again.outcome,
            RunOutcome::Skipped {
                reason: SkipReason::PreviousWaiting
            }
        );
        let conversation = client.conversation(busy.id).await.unwrap();
        assert_eq!(conversation.queue.entries.len(), 1);
        assert_eq!(
            conversation.queue.entries.first().map(|waiting| waiting.id),
            Some(entry)
        );
        assert_eq!(
            conversation
                .queue
                .entries
                .first()
                .map(|waiting| waiting.by.clone()),
            Some(Creator::Owner)
        );

        // A person stops the turn and clears the queue: nothing automatic
        // starts the session again.
        client
            .send(busy.id, SessionInput::CancelTurn { turn: None })
            .await
            .unwrap();
        let _cleared = client
            .change_queue(busy.id, QueueChange::Clear)
            .await
            .unwrap();
        let mut stream = client.follow(busy.id, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            matches!(
                &record.event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded {
                        reason: StopReason::Cancelled,
                        ..
                    }
                }
            )
        })
        .await;
        let stopped = client.run_schedule(schedule.id).await.unwrap();
        assert_eq!(
            stopped.outcome,
            RunOutcome::Skipped {
                reason: SkipReason::Stopped
            }
        );
        // Once someone sends it something, it is reached again.
        let _sent = client
            .prompt(busy.id, text("plain"), Delivery::Start)
            .await
            .unwrap();
        read_until(&mut stream, &mut seen, |record| {
            matches!(
                &record.event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded {
                        reason: StopReason::EndTurn,
                        ..
                    }
                }
            )
        })
        .await;
        let reached = client.run_schedule(schedule.id).await.unwrap();
        assert!(
            matches!(reached.outcome, RunOutcome::Prompted { .. }),
            "{reached:?}"
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_deleted_target_ends_the_schedule() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let target = structured(&client, &fixture).await;
        let schedule = client
            .create_schedule(
                definition(ScheduleTarget::Session { session: target.id }, "plain"),
                false,
            )
            .await
            .unwrap();
        client.kill(target.id).await.unwrap();
        let _ended = until_ended(&client, target.id).await;
        // Ended but kept: the run fails and the schedule goes on.
        let ended = client.run_schedule(schedule.id).await.unwrap();
        let RunOutcome::Failed { error, .. } = &ended.outcome else {
            panic!("an ended session cannot be prompted: {ended:?}");
        };
        assert!(error.message.contains("ended"), "{error:?}");
        // Deleted: the schedule ends.
        let _deleted = loop {
            match client.delete_session(target.id).await {
                Ok(deleted) => break deleted,
                Err(vessel_rpc::CallError::Remote(error)) if error.code == ErrorCode::Conflict => {
                    tokio::task::yield_now().await;
                }
                Err(other) => panic!("cannot delete: {other:?}"),
            }
        };
        let _gone = client.run_schedule(schedule.id).await.unwrap();
        let after = client.schedule(schedule.id).await.unwrap();
        assert!(
            matches!(
                after.state,
                ScheduleState::Ended {
                    reason: EndReason::TargetGone,
                    ..
                }
            ),
            "{after:?}"
        );
        assert_eq!(
            code(client.run_schedule(schedule.id).await),
            ErrorCode::Conflict
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn what_is_refused_at_creation() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let session = ScheduleTarget::Session {
            session: SessionId::random(),
        };
        assert_eq!(
            code(
                client
                    .create_schedule(definition(session, "x"), false)
                    .await
            ),
            ErrorCode::NotFound
        );
        let mut resuming = structured_spec(
            &fixture,
            Some(vessel_proto::HarnessSessionId::new("resume-me")),
        );
        resuming.title = None;
        let resume = ScheduleTarget::NewSession {
            spec: Box::new(resuming),
            after: AfterRun::Archive,
        };
        assert_eq!(
            code(client.create_schedule(definition(resume, "x"), false).await),
            ErrorCode::InvalidRequest
        );
        let fine = ScheduleTarget::NewSession {
            spec: Box::new(structured_spec(&fixture, None)),
            after: AfterRun::Keep,
        };
        let mut never = definition(fine.clone(), "x");
        never.cadence = Cadence::Cron {
            expr: "0 0 30 2 *".parse().unwrap(),
            zone: "UTC".parse().unwrap(),
        };
        assert_eq!(
            code(client.create_schedule(never, false).await),
            ErrorCode::InvalidRequest
        );
        let mut mars = definition(fine, "x");
        mars.cadence = Cadence::Cron {
            expr: "0 9 * * *".parse().unwrap(),
            zone: "Mars/Olympus".parse().unwrap(),
        };
        assert_eq!(
            code(client.create_schedule(mars, false).await),
            ErrorCode::InvalidRequest
        );
        assert_eq!(
            code(client.schedule(ScheduleId::random()).await),
            ErrorCode::NotFound
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_keeps_a_heartbeat_that_ends_with_its_session() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        let mut first = structured(&owner, &fixture).await;
        let (_program, socket, auth) = credential(&owner, &mut first).await;
        let agent = as_agent(&socket, &auth).await;

        // Its own session: a heartbeat.
        let heartbeat = agent
            .create_schedule(
                definition(ScheduleTarget::Session { session: first.id }, "plain"),
                false,
            )
            .await
            .unwrap();
        assert!(
            matches!(heartbeat.defined_by, Creator::Agent { session, .. } if session == first.id)
        );
        // Not someone else's session, and never run now.
        let theirs = structured(&owner, &fixture).await;
        assert_eq!(
            code(
                agent
                    .create_schedule(
                        definition(ScheduleTarget::Session { session: theirs.id }, "x"),
                        false
                    )
                    .await
            ),
            ErrorCode::Forbidden
        );
        assert_eq!(
            code(agent.run_schedule(heartbeat.id).await),
            ErrorCode::Forbidden
        );
        // A person's schedule is not the agent's to change.
        let persons = owner
            .create_schedule(
                definition(ScheduleTarget::Session { session: theirs.id }, "x"),
                false,
            )
            .await
            .unwrap();
        assert_eq!(
            code(agent.pause_schedule(persons.id).await),
            ErrorCode::Forbidden
        );
        let paused = agent.pause_schedule(heartbeat.id).await.unwrap();
        assert!(matches!(paused.state, ScheduleState::Paused { .. }));
        // The owner may run it now: it acts as the agent, on its own
        // session's queue.
        let run = owner.run_schedule(heartbeat.id).await.unwrap();
        assert!(
            matches!(run.outcome, RunOutcome::Prompted { session, .. } if session == first.id),
            "{run:?}"
        );

        // The session ends: its schedules go with it.
        owner.kill(first.id).await.unwrap();
        let _ended = until_ended(&owner, first.id).await;
        let gone = loop {
            match owner.schedule(heartbeat.id).await {
                Err(vessel_rpc::CallError::Remote(error)) => break error.code,
                Ok(_still) => tokio::task::yield_now().await,
                Err(other) => panic!("{other:?}"),
            }
        };
        assert_eq!(gone, ErrorCode::NotFound);
        assert!(owner.schedule(persons.id).await.is_ok());
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schedules_are_kept_across_a_restart() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let target = ScheduleTarget::NewSession {
            spec: Box::new(structured_spec(&fixture, None)),
            after: AfterRun::Keep,
        };
        let made = client
            .create_schedule(definition(target, "plain"), true)
            .await
            .unwrap();
        node.stop().await.unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let kept = client.schedule(made.id).await.unwrap();
        assert_eq!(kept, made);
        let resumed = client.resume_schedule(made.id).await.unwrap();
        assert!(matches!(resumed.state, ScheduleState::Active { .. }));
        assert_eq!(resumed.revision, made.revision.wrapping_add(1));
        client.delete_schedule(made.id).await.unwrap();
        assert!(client.schedules().await.unwrap().is_empty());
        node.stop().await.unwrap();
    })
    .await;
}
