//! Completed-definition editing preserves status, authority and revision fences.
use super::*;
use crate::automation::error::AutomationError;
struct Fixture {
    node: Scheduler,
    info: ScheduleInfo,
    jobs: mpsc::UnboundedReceiver<Job>,
}
async fn seeded(state: ScheduleState) -> Fixture {
    let (store, data) = store().await;
    let info = ScheduleInfo {
        id: ScheduleId::random(),
        definition: definition(every(300, "2026-01-01T10:00:00Z")),
        state,
        runs: 4,
        last_due: None,
        last_run: None,
        defined_by: Creator::Owner,
        created_at: at("2026-01-01T09:00:00Z"),
        updated_at: at("2026-01-01T09:00:00Z"),
        revision: 0,
    };
    store.put_schedule(info.clone(), None).await.unwrap();
    let (runner, jobs) = FakeRunner::answering(prompted());
    let node = scheduler_on(store, data, runner, HandClock::at("2026-01-01T10:00:00Z")).await;
    Fixture { node, info, jobs }
}
#[tokio::test]
async fn ended_definition_edit_keeps_end_reason_time_authority_and_revision_fences() {
    hang_guard("ended definition edit and persisted receipt", async {
        let mut fixture = seeded(ScheduleState::Ended {
            at: at("2026-01-01T09:30:00Z"),
            reason: EndReason::Expired,
        })
        .await;
        let original = &fixture.info;
        let mut definition = original.definition.clone();
        definition.prompt = "Updated completed prompt".parse().unwrap();
        definition.max_runs = Some(MaxRuns::try_from(1).unwrap());
        let foreign = Asker {
            by: Creator::Agent {
                node: NodeId::random(),
                session: SessionId::random(),
            },
            via: Via::Agent,
        };
        let denied = fixture
            .node
            .schedules
            .update(original.id, original.revision, definition.clone(), foreign)
            .await;
        assert!(matches!(denied, Err(AutomationError::Forbidden(_))));
        let forwarded = Asker {
            by: Creator::Owner,
            via: Via::Delegated { agent: false },
        };
        let denied = fixture
            .node
            .schedules
            .update(
                original.id,
                original.revision,
                definition.clone(),
                forwarded,
            )
            .await;
        assert!(matches!(denied, Err(AutomationError::Forbidden(_))));
        let changed = fixture
            .node
            .schedules
            .update(original.id, original.revision, definition.clone(), person())
            .await
            .unwrap();
        assert_eq!(changed.state, original.state);
        assert_eq!(changed.definition, definition);
        assert_eq!(changed.runs, original.runs);
        assert_eq!(changed.last_run, original.last_run);
        assert_eq!(changed.defined_by, original.defined_by);
        assert_eq!(changed.revision, 1);
        let replay = fixture
            .node
            .schedules
            .update(original.id, original.revision, definition, person())
            .await;
        assert!(matches!(replay, Err(AutomationError::Conflict(_))));
        assert_eq!(
            fixture.node.schedules.get(original.id).await.unwrap(),
            changed
        );
        let saved = fixture.node.store.automation().await.unwrap();
        assert_eq!(
            saved.schedules.iter().find(|info| info.id == original.id),
            Some(&changed)
        );
        assert!(matches!(
            fixture.jobs.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    })
    .await;
}
#[tokio::test]
async fn unknown_definition_edit_is_refused_without_touching_the_record() {
    hang_guard("unknown definition refusal", async {
        for state in [
            ScheduleState::Unknown,
            ScheduleState::Ended {
                at: at("2026-01-01T09:30:00Z"),
                reason: EndReason::Unknown,
            },
        ] {
            let mut fixture = seeded(state).await;
            let mut definition = fixture.info.definition.clone();
            definition.prompt = "Must not be stored".parse().unwrap();
            let refused = fixture
                .node
                .schedules
                .update(fixture.info.id, fixture.info.revision, definition, person())
                .await;
            assert!(matches!(refused, Err(AutomationError::Conflict(_))));
            assert_eq!(
                fixture.node.schedules.get(fixture.info.id).await.unwrap(),
                fixture.info
            );
            let stored = fixture.node.store.automation().await.unwrap();
            assert_eq!(
                stored
                    .schedules
                    .iter()
                    .find(|info| info.id == fixture.info.id),
                Some(&fixture.info)
            );
            assert!(matches!(
                fixture.jobs.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
        }
    })
    .await;
}
#[tokio::test]
async fn active_definition_edit_still_keeps_its_due_time_when_cadence_is_unchanged() {
    hang_guard("active definition edit retains next due", async {
        let mut fixture = seeded(ScheduleState::Active {
            next_due: at("2026-01-01T10:05:00Z"),
        })
        .await;
        let mut definition = fixture.info.definition.clone();
        definition.prompt = "Changed without resetting cadence".parse().unwrap();
        let changed = fixture
            .node
            .schedules
            .update(
                fixture.info.id,
                fixture.info.revision,
                definition.clone(),
                person(),
            )
            .await
            .unwrap();
        assert_eq!(changed.state, fixture.info.state);
        assert_eq!(changed.definition, definition);
        assert_eq!(changed.revision, 1);
        assert_eq!(changed.runs, fixture.info.runs);
        assert_eq!(
            fixture.node.schedules.get(fixture.info.id).await.unwrap(),
            changed
        );
        assert!(matches!(
            fixture.jobs.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    })
    .await;
}
