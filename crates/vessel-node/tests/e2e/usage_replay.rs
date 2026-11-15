//! Cumulative cost during the real runtime's hidden Ready boundaries.
//! Only fake ACP vendor-shaped reports and temporary agent homes are used.
use crate::support::{Fixture, ask, read_until, status_of, structured_spec, within};
use vessel_client::{NodeClient, SessionStream};
use vessel_harness::fakes::claude_project;
use vessel_proto::conversation::{FiniteNumber, UsageObservation};
use vessel_proto::forking::RewindSession;
use vessel_proto::parking::{ParkSession, WakeSession};
use vessel_proto::usage::{QueryUsage, UsagePeriod, UsageScope};
use vessel_proto::{AgentEvent, EventRecord, EventSeq, SessionEvent, SessionId, SessionStatus};

#[derive(Clone, Copy)]
enum Restart {
    Wake,
    Rewind,
    Profile,
}
struct Running {
    id: SessionId,
    stream: SessionStream,
    seen: Vec<EventRecord>,
    anchor: EventSeq,
}

fn query(id: SessionId) -> QueryUsage {
    QueryUsage {
        scope: UsageScope::Session { session: id },
        period: UsagePeriod::Session,
        after: None,
    }
}
fn profiles(fixture: &Fixture) {
    let home = fixture.cwd().join("agent-home");
    let text = ["source", "target"].map(|name| format!(
        "[profiles.{name}]\nharness = 'claude'\nenv = {{ CLAUDE_CONFIG_DIR = '{}', FAKE_ACP_LOAD_USAGE = 'higher' }}\n", home.display()
    )).join("\n");
    std::fs::create_dir_all(fixture.data_path()).unwrap();
    std::fs::write(fixture.data_dir().profiles_path(), text).unwrap();
}
async fn start(fixture: &Fixture, client: &NodeClient) -> Running {
    let mut spec = structured_spec(fixture, None);
    spec.profile = Some("source".parse().unwrap());
    let id = client.create_session(spec).await.unwrap().id;
    let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
    let mut seen = Vec::new();
    read_until(&mut stream, &mut seen, |r| {
        status_of(r) == Some(&SessionStatus::Running)
    })
    .await;
    assert_eq!(
        ask(client, &mut stream, &mut seen, id, "plain").await,
        "echo: plain"
    );
    let anchor = seen
        .iter()
        .rev()
        .find_map(|r| {
            matches!(
                &r.event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded { point: Some(_), .. }
                }
            )
            .then_some(r.seq)
        })
        .unwrap();
    assert_eq!(
        ask(client, &mut stream, &mut seen, id, "usage").await,
        "used"
    );
    Running {
        id,
        stream,
        seen,
        anchor,
    }
}
async fn restart(case: Restart, fixture: &Fixture, client: &NodeClient, run: &Running) {
    match case {
        Restart::Wake => {
            let _parked = client.call(ParkSession { session: run.id }).await.unwrap();
            let _awake = client.call(WakeSession { session: run.id }).await.unwrap();
        }
        Restart::Rewind => {
            let _rewound = client
                .call(RewindSession {
                    session: run.id,
                    after: run.anchor,
                })
                .await
                .unwrap();
        }
        Restart::Profile => {
            let native = client
                .session(run.id)
                .await
                .unwrap()
                .harness_session
                .unwrap();
            let project =
                claude_project(&fixture.cwd().join("agent-home"), &fixture.cwd()).unwrap();
            std::fs::create_dir_all(&project).unwrap();
            std::fs::write(project.join(format!("{native}.jsonl")), "{}\n").unwrap();
            let _moved = client
                .move_profile(run.id, Some("target".parse().unwrap()), false)
                .await
                .unwrap();
        }
    }
}
async fn scenario(case: Restart) {
    let fixture = Fixture::new();
    profiles(&fixture);
    let node = fixture.start().await;
    let client = fixture.connect(&node).await;
    let mut run = start(&fixture, &client).await;
    restart(case, &fixture, &client, &run).await;
    for prompt in ["usage-after-load", "usage-later"] {
        assert_eq!(
            ask(&client, &mut run.stream, &mut run.seen, run.id, prompt).await,
            "used"
        );
    }
    assert!(run.seen.iter().any(|r| matches!(&r.event,
        SessionEvent::UsageChanged { usage }
            if usage.observation == UsageObservation::Replayed
                && usage.cost.as_ref().is_some_and(|c| c.amount == FiniteNumber::new(10.0).unwrap())
    )));
    let report = client.usage(query(run.id)).await.unwrap();
    let (total, gaps) = match case {
        Restart::Wake => (4.5, 1),
        Restart::Rewind | Restart::Profile => (2.5, 2),
    };
    assert_eq!(
        report.totals.costs[0].amount,
        FiniteNumber::new(total).unwrap()
    );
    assert_eq!(report.totals.unaccounted_costs.get(), gaps);
    assert_eq!(
        report.current_costs[0].amount,
        FiniteNumber::new(14.0).unwrap()
    );
    if let Restart::Profile = case {
        let source = report
            .agents
            .iter()
            .find(|g| g.profile.as_ref().is_some_and(|p| p.as_str() == "source"))
            .unwrap();
        let target = report
            .agents
            .iter()
            .find(|g| g.profile.as_ref().is_some_and(|p| p.as_str() == "target"))
            .unwrap();
        assert_eq!(
            source.totals.costs[0].amount,
            FiniteNumber::new(0.5).unwrap()
        );
        assert_eq!(
            target.totals.costs[0].amount,
            FiniteNumber::new(2.0).unwrap()
        );
    }
    node.stop().await.unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_wake_marks_the_load_observation_and_only_counts_later_increases() {
    within(scenario(Restart::Wake)).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_rewind_cannot_spend_a_replayed_increase_before_its_new_native_identity() {
    within(scenario(Restart::Rewind)).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_profile_move_cannot_spend_the_new_accounts_replay_under_the_old_profile() {
    within(scenario(Restart::Profile)).await;
}
