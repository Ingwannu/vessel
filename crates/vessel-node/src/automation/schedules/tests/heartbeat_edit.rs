//! A cadence edit keeps the identity a future run acts as.

use super::*;
use crate::automation::error::AutomationError;
use vessel_proto::{DeviceId, HarnessKind, SessionMode, SessionSpec, automation::AfterRun};

const NOW: &str = "2026-01-01T10:00:00Z";

fn daily() -> Cadence {
    Cadence::Cron {
        expr: "0 11 * * *".parse().unwrap(),
        zone: "UTC".parse().unwrap(),
    }
}

#[tokio::test]
async fn cadence_edit_preserves_person_and_agent_ownership() {
    let session = SessionId::random();
    let agent = Asker {
        by: Creator::Agent {
            node: NodeId::random(),
            session,
        },
        via: Via::Agent,
    };
    let device = Asker {
        by: Creator::Device {
            id: DeviceId::random(),
        },
        via: Via::Person,
    };
    let (runner, _jobs) = FakeRunner::answering(prompted());
    let node = scheduler(runner, HandClock::at(NOW)).await;
    let mut definition = definition(every(300, NOW));
    definition.target = ScheduleTarget::Session { session };
    definition.max_runs = Some(MaxRuns::try_from(4).unwrap());
    definition.ends_at = Some(at("2026-01-03T00:00:00Z"));
    let mut edited = Vec::new();
    for definer in [agent.clone(), device] {
        let old = node
            .schedules
            .create(definition.clone(), false, definer)
            .await
            .unwrap();
        let changed = node
            .schedules
            .update_cadence(old.id, old.revision, daily(), person())
            .await
            .unwrap();
        let mut expected = old.clone();
        expected.definition.cadence = daily();
        expected.state = ScheduleState::Active {
            next_due: at("2026-01-01T11:00:00Z"),
        };
        expected.revision = 1;
        assert_eq!(changed, expected);
        let saved = node.store.automation().await.unwrap();
        assert_eq!(
            saved.schedules.iter().find(|info| info.id == old.id),
            Some(&changed)
        );
        edited.push(changed.id);
    }
    node.schedules.starting_again(session).await.unwrap();
    assert!(node.schedules.get(edited[0]).await.is_err());
    assert!(node.schedules.get(edited[1]).await.is_ok());
}

#[tokio::test]
async fn cadence_edit_requires_current_revision_direct_authority_and_heartbeat_target() {
    let (runner, _jobs) = FakeRunner::answering(prompted());
    let node = scheduler(runner, HandClock::at(NOW)).await;
    let old = node
        .schedules
        .create(definition(every(300, NOW)), true, person())
        .await
        .unwrap();
    let foreign = Asker {
        by: Creator::Agent {
            node: NodeId::random(),
            session: SessionId::random(),
        },
        via: Via::Agent,
    };
    let forwarded = Asker {
        by: Creator::Owner,
        via: Via::Delegated { agent: false },
    };
    for asker in [foreign, forwarded] {
        let result = node
            .schedules
            .update_cadence(old.id, old.revision, daily(), asker)
            .await;
        assert!(matches!(result, Err(AutomationError::Forbidden(_))));
    }
    let mut definition = old.definition;
    definition.target = ScheduleTarget::NewSession {
        isolation: None,
        spec: Box::new(SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        )),
        after: AfterRun::Archive,
    };
    let changed = node
        .schedules
        .update(old.id, old.revision, definition, person())
        .await
        .unwrap();
    let stale = node
        .schedules
        .update_cadence(old.id, old.revision, daily(), person())
        .await;
    assert!(matches!(stale, Err(AutomationError::Conflict(_))));
    let new_session = node
        .schedules
        .update_cadence(old.id, changed.revision, daily(), person())
        .await;
    assert!(matches!(new_session, Err(AutomationError::Invalid(_))));
    assert_eq!(node.schedules.get(old.id).await.unwrap(), changed);
}
