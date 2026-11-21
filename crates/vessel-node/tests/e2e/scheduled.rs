//! Where track D's schedules meet the rest of protocol 11 (milestone 11,
//! "Integration 3 (node)"): a scheduled session in a worktree is watched
//! for its pull request as a person's is, and put away after its first
//! turn with the service and the shell beside it, as a person's archive
//! does; a first turn that fails with the agent alive; a schedule of an
//! agent added by its command (track G) while its secret is missing and
//! once it is removed; a heartbeat does not survive its session started
//! again; a spec kept for later is bounded.
//!
//! Unix only, as `meeting` is: the shell is `/bin/sh`.

use vessel_client::NodeClient;
use vessel_forge::fake::FakeForge;
use vessel_proto::agents::RemoveAgent;
use vessel_proto::automation::{
    AfterRun, Cadence, CatchUp, EverySeconds, RunEnd, RunErrorCode, RunOutcome, ScheduleDefinition,
    ScheduleInfo, ScheduleState, ScheduleTarget,
};
use vessel_proto::conversation::ResumeSession;
use vessel_proto::scripts::{RunScript, ScriptPlace, ScriptRunOutcome};
use vessel_proto::terminals::{CreateTerminal, ServiceState, TerminalInfo, TerminalPlace};
use vessel_proto::workspace::SessionPlace;
use vessel_proto::{
    AgentEvent, ErrorCode, EventSeq, HarnessRef, PermissionOptionId, PermissionOutcome, ScheduleId,
    SessionEvent, SessionId, SessionInput, SessionMode, SessionSpec,
};

use crate::added::{add, name, quick, set_secret};
use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::forge::{info_until, project};
use crate::meeting::{nothing_watched, start, terminal, with_a_service};
use crate::support::{Fixture, read_until, status_of, step, structured_spec, within};
use crate::workspace::new_worktree;

/// An hourly schedule (never due while a test runs: runs are asked for).
fn hourly(target: ScheduleTarget, prompt: &str) -> ScheduleDefinition {
    ScheduleDefinition {
        name: Some("joined".parse().unwrap()),
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

fn new_session(spec: SessionSpec) -> ScheduleTarget {
    ScheduleTarget::NewSession {
        spec: Box::new(spec),
        after: AfterRun::Archive,
    }
}

/// The schedule once `wanted` holds of its last run's outcome.
async fn last_run_until(
    client: &NodeClient,
    schedule: ScheduleId,
    wanted: impl Fn(&RunOutcome) -> bool,
) -> ScheduleInfo {
    step("the schedule's last run", async {
        loop {
            let info = client.schedule(schedule).await.unwrap();
            if info
                .last_run
                .as_ref()
                .is_some_and(|run| wanted(&run.outcome))
            {
                return info;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
}

/// `info`'s terminal once it has ended.
async fn ended_terminal(
    owner: &NodeClient,
    info: &TerminalInfo,
    what: &'static str,
) -> TerminalInfo {
    step(what, async {
        loop {
            let now = terminal(owner, info).await;
            if !now.is_live() {
                return now;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
}

/// A scheduled session placed in a worktree is watched for its pull
/// request as its definer's own session would be; a service and a shell
/// started beside it while its first turn waits on a permission end when
/// the run puts it away after that turn, and the watch ends with them.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_scheduled_session_is_watched_and_put_away_with_what_runs_beside_it() {
    within(async {
        let forge = FakeForge::new();
        let fixture = Fixture::new();
        let node = start(&fixture, &forge).await;
        let owner = fixture.connect(&node).await;
        let (_folder, made) = project(&owner).await;
        with_a_service(&owner, &made.root, made.id).await;
        let worktree = new_worktree(&owner, made.id).await;
        let mut spec = structured_spec(&fixture, None).with_place(SessionPlace::Worktree {
            worktree: worktree.id,
        });
        spec.cwd = None;
        // Any other prompt makes the fake agent ask a permission.
        let schedule = owner
            .create_schedule(hourly(new_session(spec), "look around"), false)
            .await
            .unwrap();
        let run = owner.run_schedule(schedule.id).await.unwrap();
        let RunOutcome::Started { session } = run.outcome else {
            panic!("a new session's run starts one: {run:?}");
        };
        let _watched = info_until(&owner, session, |info| info.pull_request.is_some()).await;

        let service = match owner
            .call(RunScript {
                place: ScriptPlace::Session { session },
                script: "web".parse().unwrap(),
            })
            .await
            .unwrap()
        {
            ScriptRunOutcome::Started { terminal } => *terminal,
            other @ (ScriptRunOutcome::NeedsApproval { .. } | ScriptRunOutcome::Refused { .. }) => {
                panic!("the service did not start: {other:?}")
            }
        };
        let shell = owner
            .call(CreateTerminal {
                place: TerminalPlace::Session { session },
                size: None,
                title: None,
                profile: None,
            })
            .await
            .unwrap();

        // Nobody is asked a question, but a permission waits for a person.
        let _waiting = last_run_until(&owner, schedule.id, |outcome| {
            matches!(outcome, RunOutcome::Waiting { .. })
        })
        .await;
        let mut stream = owner.follow(session, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            matches!(
                &record.event,
                SessionEvent::Agent {
                    event: AgentEvent::PermissionRequested(_)
                }
            )
        })
        .await;
        let Some(SessionEvent::Agent {
            event: AgentEvent::PermissionRequested(asked),
        }) = seen.last().map(|record| &record.event)
        else {
            panic!("the permission was read last");
        };
        owner
            .send(
                session,
                SessionInput::AnswerPermission {
                    id: asked.id,
                    outcome: PermissionOutcome::Selected {
                        option: PermissionOptionId::new("allow-once"),
                    },
                },
            )
            .await
            .unwrap();

        // The turn ends; the run puts the session away by the service's
        // one path: archived, unwatched, and what ran beside it ended.
        let archived = info_until(&owner, session, |info| info.marks.archived_at.is_some()).await;
        assert!(archived.pull_request.is_none(), "{archived:?}");
        nothing_watched(&owner).await;
        // The archive shows before what ran beside it has ended (each
        // terminal's end is told as its process exits): wait for each.
        let service = ended_terminal(&owner, &service, "the service ends").await;
        assert!(
            matches!(service.service, Some(ServiceState::Stopped)),
            "not restarted: {service:?}"
        );
        let _shell = ended_terminal(&owner, &shell, "the shell ends").await;
        node.stop().await.unwrap();
    })
    .await;
}

/// A first turn that fails while the agent stays up finishes the run
/// `failed` with the agent's words, and the session is archived as one
/// whose turn ended is: each run would otherwise leave one more live.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_first_turn_that_fails_with_the_agent_alive_is_told_and_put_away() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let target = new_session(structured_spec(&fixture, None));
        let schedule = owner
            .create_schedule(hourly(target, "fail"), false)
            .await
            .unwrap();
        let run = owner.run_schedule(schedule.id).await.unwrap();
        let RunOutcome::Started { session } = run.outcome else {
            panic!("a new session's run starts one: {run:?}");
        };
        let after = last_run_until(&owner, schedule.id, |outcome| {
            matches!(outcome, RunOutcome::Finished { .. })
        })
        .await;
        let Some(RunOutcome::Finished {
            session: finished,
            end: RunEnd::Failed { message },
        }) = after.last_run.map(|run| run.outcome)
        else {
            panic!("the run is finished `failed`");
        };
        assert_eq!(finished, session);
        assert!(message.contains("the turn failed"), "{message}");
        let info = info_until(&owner, session, |info| info.marks.archived_at.is_some()).await;
        assert!(!info.status.is_live(), "{info:?}");
        assert_eq!(after.runs, 1, "it sent its prompt");
        node.stop().await.unwrap();
    })
    .await;
}

/// A schedule of an agent added by its command (track G) is checked as
/// `session.create` would check it: refused (`conflict`) while the
/// agent's secret is missing; once set, its runs start that agent. When
/// the agent is removed, each run fails `not_found` and the schedule goes
/// on (the agent may be added again under its name).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_schedule_of_an_added_agent_meets_its_secret_and_its_removal() {
    within(async {
        let fixture = Fixture::new();
        let node = quick(&fixture).await;
        let owner = fixture.connect(&node).await;
        let env_file = fixture.cwd().join("fake-env");
        let _added = owner.call(add("fake", &env_file)).await.unwrap();
        let spec = SessionSpec::new(
            HarnessRef::Added(name("fake")),
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        )
        .with_cwd(fixture.cwd());
        let target = new_session(spec);

        let missing = owner
            .create_schedule(hourly(target.clone(), "plain"), false)
            .await;
        assert_eq!(code(missing), ErrorCode::Conflict);

        set_secret(&owner, "s3cret").await;
        let schedule = owner
            .create_schedule(hourly(target, "plain"), false)
            .await
            .unwrap();
        let run = owner.run_schedule(schedule.id).await.unwrap();
        let RunOutcome::Started { session } = run.outcome else {
            panic!("the added agent's session starts: {run:?}");
        };
        let info = info_until(&owner, session, |info| info.marks.archived_at.is_some()).await;
        assert_eq!(info.spec.harness, HarnessRef::Added(name("fake")));

        let _removed = owner
            .call(RemoveAgent { name: name("fake") })
            .await
            .unwrap();
        let run = owner.run_schedule(schedule.id).await.unwrap();
        let RunOutcome::Failed {
            error,
            session: None,
        } = &run.outcome
        else {
            panic!("a removed agent's run fails: {run:?}");
        };
        assert_eq!(error.code, RunErrorCode::NotFound, "{error:?}");
        let after = owner.schedule(schedule.id).await.unwrap();
        assert!(
            matches!(after.state, ScheduleState::Active { .. }),
            "{after:?}"
        );
        node.stop().await.unwrap();
    })
    .await;
}

/// A heartbeat ends with its session's run: started again
/// (`session.resume`, the same id), the session has none.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_heartbeat_does_not_survive_its_session_started_again() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        let mut first = structured(&owner, &fixture).await;
        let (_program, socket, auth) = credential(&owner, &mut first).await;
        let agent = as_agent(&socket, &auth).await;
        let heartbeat = agent
            .create_schedule(
                hourly(ScheduleTarget::Session { session: first.id }, "plain"),
                false,
            )
            .await
            .unwrap();

        owner.kill(first.id).await.unwrap();
        ended(&owner, first.id).await;
        let again = owner
            .call(ResumeSession { session: first.id })
            .await
            .unwrap();
        assert_eq!(again.id, first.id);
        assert_eq!(
            code(owner.schedule(heartbeat.id).await),
            ErrorCode::NotFound
        );
        assert!(owner.schedules().await.unwrap().is_empty());
        node.stop().await.unwrap();
    })
    .await;
}

/// Read `session`'s log until its final status.
async fn ended(client: &NodeClient, session: SessionId) {
    let mut stream = client.follow(session, EventSeq::ZERO).await.unwrap();
    read_until(&mut stream, &mut Vec::new(), |record| {
        status_of(record).is_some_and(|status| !status.is_live())
    })
    .await;
}

/// A spec kept for later is bounded as the prompt beside it is: a folder
/// of 20 KiB is refused at creation (the join's code review, finding 5).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_spec_kept_for_later_is_bounded() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let spec = structured_spec(&fixture, None).with_cwd(fixture.cwd().join("x".repeat(20_000)));
        let refused = owner
            .create_schedule(hourly(new_session(spec), "plain"), false)
            .await;
        assert_eq!(code(refused), ErrorCode::InvalidRequest);
        node.stop().await.unwrap();
    })
    .await;
}
