//! The plan is a durable bounded projection, not an accidental part of a
//! recent event page. Exercise pruning and recovery against real `SQLite`.

use std::sync::Arc;

use chrono::DateTime;
use vessel_proto::{
    AgentEvent, ContentBlock, EventRecord, EventSeq, HarnessKind, PlanEntry, PlanPriority,
    PlanStatus, SessionEvent, SessionId, SessionInfo, SessionMode, SessionSpec,
};
use vessel_store::{ByteCount, DataDir, Retention, Store, StoreConfig, StoreError};

fn record(seq: u64, event: AgentEvent) -> Arc<EventRecord> {
    Arc::new(EventRecord {
        seq: EventSeq::new(seq),
        at: DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent { event },
    })
}

#[tokio::test]
async fn latest_plan_survives_a_tool_heavy_tail_retention_and_restart() {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let config = StoreConfig {
        retention: Retention {
            max_session_bytes: ByteCount::new(1024),
            ..Retention::default()
        },
        ..StoreConfig::default()
    };
    let store = Store::open(&dir, config).await.unwrap().store;
    let session = SessionInfo::new(
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
    store.create_session(session.clone()).await.unwrap();
    let plan = vec![PlanEntry {
        content: "Verify the retained plan".to_owned(),
        priority: PlanPriority::High,
        status: PlanStatus::InProgress,
    }];
    store
        .append(
            session.id,
            vec![record(
                0,
                AgentEvent::Plan {
                    entries: plan.clone(),
                },
            )],
        )
        .await
        .unwrap();
    let tail = (1..=300)
        .map(|seq| {
            record(
                seq,
                AgentEvent::MessageChunk {
                    content: ContentBlock::Text {
                        text: format!("output {seq}"),
                    },
                },
            )
        })
        .collect();
    store.append(session.id, tail).await.unwrap();
    let context = store
        .side_context(session.id, EventSeq::new(301))
        .await
        .unwrap();
    assert_eq!(context.plan.unwrap().entries, plan);
    assert!(context.records.len() <= 128);
    assert_eq!(context.records.first().unwrap().seq, EventSeq::new(300));
    assert!(
        store
            .read_events(session.id, EventSeq::ZERO, 1)
            .await
            .unwrap()
            .records
            .first()
            .unwrap()
            .seq
            > EventSeq::ZERO
    );
    store.close().await;
    let store = Store::open(&dir, config).await.unwrap().store;
    assert_eq!(
        store
            .side_context(session.id, EventSeq::new(302))
            .await
            .unwrap()
            .plan
            .unwrap()
            .entries,
        plan
    );
    store.delete_session(session.id).await.unwrap();
    assert!(
        matches!(store.side_context(session.id, EventSeq::new(302)).await, Err(StoreError::UnknownSession(id)) if id == session.id)
    );
    store.close().await;
}

#[tokio::test]
async fn a_rewind_clears_the_plan_and_a_later_plan_replaces_it_in_the_same_append() {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let info = SessionInfo::new(
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
    store.create_session(info.clone()).await.unwrap();
    let entries = vec![PlanEntry {
        content: "obsolete plan".to_owned(),
        priority: PlanPriority::High,
        status: PlanStatus::InProgress,
    }];
    store
        .append(
            info.id,
            vec![record(
                0,
                AgentEvent::Plan {
                    entries: entries.clone(),
                },
            )],
        )
        .await
        .unwrap();
    let rewind = |seq| {
        Arc::new(EventRecord {
            seq: EventSeq::new(seq),
            at: DateTime::UNIX_EPOCH,
            event: SessionEvent::Rewound {
                after: EventSeq::ZERO,
                files_changed: 0,
                harness_session: vessel_proto::HarnessSessionId::new("rewound-conversation"),
            },
        })
    };
    store.append(info.id, vec![rewind(1)]).await.unwrap();
    let context = store.side_context(info.id, EventSeq::new(2)).await.unwrap();
    assert_eq!(context.plan, None);
    assert!(context.records.is_empty());
    store
        .append(
            info.id,
            vec![
                rewind(2),
                record(
                    3,
                    AgentEvent::Plan {
                        entries: entries.clone(),
                    },
                ),
            ],
        )
        .await
        .unwrap();
    assert_eq!(
        store
            .side_context(info.id, EventSeq::new(4))
            .await
            .unwrap()
            .plan
            .unwrap()
            .entries,
        entries
    );
    store.close().await;
}
