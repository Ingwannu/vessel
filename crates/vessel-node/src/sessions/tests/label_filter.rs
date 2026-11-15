//! The staged filter uses the real merged list, current columns and fake runtime.

use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use vessel_proto::{EventSeq, SessionId, SessionInfo, SessionLabels, SessionStatus};
use vessel_session::LaunchOptions;

use super::Fixture;

const SCENARIO_BUDGET: Duration = Duration::from_secs(60);

fn labels(pairs: &[(&str, &str)]) -> SessionLabels {
    let mut labels = SessionLabels::new();
    for (key, value) in pairs {
        labels
            .insert(key.parse().unwrap(), value.parse().unwrap())
            .unwrap();
    }
    labels
}

async fn stored(
    fixture: &Fixture,
    template: &SessionInfo,
    current: SessionLabels,
    created_at: DateTime<Utc>,
) -> SessionId {
    let mut info = template.clone();
    info.id = SessionId::random();
    info.spec.labels = labels(&[("source", "creation")]);
    info.labels = current;
    info.created_at = created_at;
    info.status = SessionStatus::Exited { code: Some(0) };
    info.ended_at = Some(created_at);
    info.next_seq = EventSeq::ZERO;
    let id = info.id;
    fixture.store.create_session(info).await.unwrap();
    id
}

fn ids(sessions: Vec<SessionInfo>) -> Vec<SessionId> {
    sessions.into_iter().map(|info| info.id).collect()
}

async fn stop(fixture: Fixture) {
    fixture.runtime.shutdown().await;
    fixture.settle().await;
    fixture.stop().await;
}

struct Population {
    required: SessionLabels,
    live: SessionInfo,
    older: SessionId,
    other: SessionId,
}

impl Population {
    async fn new(fixture: &Fixture) -> Self {
        let required = labels(&[("team", "prod"), ("role", "worker")]);
        let mut spec = fixture.spec();
        spec.labels = required.clone();
        let live = fixture
            .sessions
            .create(spec, LaunchOptions::default(), None)
            .await
            .unwrap();
        let older = stored(
            fixture,
            &live,
            required.clone(),
            live.created_at - TimeDelta::seconds(2),
        )
        .await;
        let other = stored(
            fixture,
            &live,
            labels(&[("team", "prod"), ("role", "other")]),
            live.created_at - TimeDelta::seconds(1),
        )
        .await;
        Self {
            required,
            live,
            older,
            other,
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn current_labels_are_anded_across_live_and_stored_sessions_in_existing_order() {
    vessel_testkit::within(
        SCENARIO_BUDGET,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let population = Population::new(&fixture).await;
            let matching = fixture
                .sessions
                .matching_labels(&population.required)
                .await
                .unwrap();
            assert_eq!(ids(matching), [population.live.id, population.older]);
            let all = fixture
                .sessions
                .matching_labels(&SessionLabels::new())
                .await
                .unwrap();
            assert_eq!(
                ids(all),
                [population.live.id, population.other, population.older]
            );
            let creation = fixture
                .sessions
                .get(population.older)
                .await
                .unwrap()
                .spec
                .labels;
            assert!(!creation.covers(&population.required));
            assert_eq!(
                fixture.sessions.get(population.other).await.unwrap().id,
                population.other
            );
            assert!(
                fixture
                    .sessions
                    .matching_labels(&labels(&[("team", "absent")]))
                    .await
                    .unwrap()
                    .is_empty()
            );
            stop(fixture).await;
        }),
    )
    .await
    .expect("the label discovery scenario did not finish");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn label_discovery_does_not_expose_a_runtime_session_before_its_node_entry() {
    vessel_testkit::within(
        SCENARIO_BUDGET,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let required = labels(&[("team", "prod")]);
            let mut spec = fixture.spec();
            spec.labels = required.clone();
            let created = fixture.runtime.create(spec).await.unwrap();
            assert!(
                fixture
                    .sessions
                    .matching_labels(&required)
                    .await
                    .unwrap()
                    .is_empty()
            );
            created.handle.kill();
            drop(created.durable);
            let _ended = created.handle.ended().await;
            stop(fixture).await;
        }),
    )
    .await
    .expect("the label discovery visibility scenario did not finish");
}
