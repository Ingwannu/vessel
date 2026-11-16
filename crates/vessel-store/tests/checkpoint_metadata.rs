//! Files-only metadata must not replace the conversation or discard its plan.
use chrono::DateTime;
use std::sync::Arc;
use vessel_proto::{
    AgentEvent, EventRecord, EventSeq, HarnessKind, HarnessSessionId, PlanEntry, PlanPriority,
    PlanStatus, SessionEvent, SessionId, SessionInfo, SessionMode, SessionSpec,
    checkpoint::CheckpointEvent,
};
use vessel_store::{DataDir, Store, StoreConfig};
async fn conversation_fixture(store: &Store) -> (SessionInfo, Vec<PlanEntry>) {
    let mut session = SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        DateTime::UNIX_EPOCH,
    );
    session.harness_session = Some(HarnessSessionId::new("same-conversation"));
    store.create_session(session.clone()).await.unwrap();
    let plan = vec![PlanEntry {
        content: "Preserve the conversation".to_owned(),
        priority: PlanPriority::High,
        status: PlanStatus::InProgress,
    }];
    store
        .append(
            session.id,
            vec![Arc::new(EventRecord {
                seq: EventSeq::ZERO,
                at: DateTime::UNIX_EPOCH,
                event: SessionEvent::Agent {
                    event: AgentEvent::Plan {
                        entries: plan.clone(),
                    },
                },
            })],
        )
        .await
        .unwrap();
    (session, plan)
}

#[tokio::test]
async fn checkpoint_metadata_preserves_conversation_identity_and_the_retained_plan() {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let (session, plan) = conversation_fixture(&store).await;
    let before = store.conversation(session.id).await.unwrap();
    let events = [
        CheckpointEvent::PromptAccepted,
        CheckpointEvent::TurnAssociated {
            prompt: EventSeq::new(1),
            turn: EventSeq::ZERO,
        },
        CheckpointEvent::PromptAccepted,
    ];
    let records = events
        .into_iter()
        .enumerate()
        .map(|(index, event)| {
            Arc::new(EventRecord {
                seq: EventSeq::new(u64::try_from(index).unwrap().checked_add(1).unwrap()),
                at: DateTime::UNIX_EPOCH,
                event: SessionEvent::Checkpoint { event },
            })
        })
        .collect();
    store.append(session.id, records).await.unwrap();
    assert_eq!(store.conversation(session.id).await.unwrap(), before);
    assert_eq!(
        store
            .session(session.id)
            .await
            .unwrap()
            .unwrap()
            .harness_session,
        session.harness_session
    );
    assert_eq!(
        store
            .side_context(session.id, EventSeq::new(4))
            .await
            .unwrap()
            .plan
            .unwrap()
            .entries,
        plan
    );
    store.close().await;
    let reopened = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    assert_eq!(
        reopened
            .session(session.id)
            .await
            .unwrap()
            .unwrap()
            .harness_session,
        session.harness_session
    );
    assert_eq!(
        reopened
            .side_context(session.id, EventSeq::new(5))
            .await
            .unwrap()
            .plan
            .unwrap()
            .entries,
        plan
    );
    reopened.close().await;
}
