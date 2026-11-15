//! Ended metadata uses the real writer without creating or resuming an actor.
use super::{
    CheckpointWrite,
    paths::Phase,
    records::Operation,
    writer_tests::{info, plan, publish_capture, read_operation},
};
use crate::{DataDir, Store, StoreConfig};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use vessel_proto::{
    AgentEvent, EventRecord, EventSeq, HarnessSessionId, PromptQueue, QueueEntryId, QueueHold,
    QueuedPrompt, SessionEvent, SessionStatus,
    checkpoint::{CheckpointEvent, PathCount, RestoreId, RestoreOutcome},
};

async fn completed(store: &Store) -> Operation {
    let info = info();
    store.create_session(info.clone()).await.unwrap();
    let plan = plan(info.id);
    publish_capture(store, &plan).await;
    store
        .checkpoint_write(CheckpointWrite::Allocate {
            key: plan.key,
            allocation: super::Allocation::Preview { plan: plan.id },
            owner: plan.owner,
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::Plan {
            plan: Box::new(plan.clone()),
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
    let restore = RestoreId::random();
    store
        .checkpoint_write(CheckpointWrite::Admit {
            plan: Box::new(plan.clone()),
            restore,
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
    let operation = read_operation(store, &plan, restore).await;
    store
        .checkpoint_write(CheckpointWrite::Staged {
            operation: Box::new(operation.clone()),
            paths: super::tests::staged(&plan),
        })
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::Advance {
            operation: Box::new(operation.clone()),
            path: serde_json::from_str("\"file\"").unwrap(),
            from: Phase::Pending,
            to: Phase::Applied,
        })
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::Outcome {
            operation: Box::new(operation),
            result: RestoreOutcome::Restored {
                changed: PathCount::new(1).unwrap(),
            },
        })
        .await
        .unwrap();
    read_operation(store, &plan, restore).await
}

async fn ended(store: &Store, operation: &Operation) -> DateTime<Utc> {
    let at = DateTime::from_timestamp(100, 0).unwrap();
    let events = [
        SessionEvent::QueueChanged {
            queue: PromptQueue {
                entries: vec![QueuedPrompt {
                    id: QueueEntryId::random(),
                    content: Vec::new(),
                    queued_at: at,
                    revision: 4,
                    by: Default::default(),
                }],
                held: Some(QueueHold::Ended),
            },
        },
        SessionEvent::Agent {
            event: AgentEvent::Ready {
                harness_session: HarnessSessionId::new("original-conversation"),
                abilities: None,
            },
        },
        SessionEvent::Status {
            status: SessionStatus::Exited { code: Some(7) },
        },
    ];
    store
        .append(
            operation.session,
            events
                .into_iter()
                .enumerate()
                .map(|(index, event)| {
                    Arc::new(EventRecord {
                        seq: EventSeq::new(u64::try_from(index).unwrap()),
                        at,
                        event,
                    })
                })
                .collect(),
        )
        .await
        .unwrap();
    at
}

async fn recorded_seq(store: &Store, operation: &Operation) -> Option<i64> {
    let id = operation.id.to_string();
    store
        .read(move |reader| {
            reader.with(|connection| {
                Ok(connection.query_row(
                    "SELECT event_seq FROM checkpoint_restores WHERE id=?1",
                    [id],
                    |row| row.get(0),
                )?)
            })
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn ended_restore_preserves_projections_and_history_after_status_then_deduplicates_on_restart()
{
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let operation = completed(&store).await;
    let ended_at = ended(&store, &operation).await;
    let before = store.session(operation.session).await.unwrap().unwrap();
    let conversation = store.conversation(operation.session).await.unwrap();
    for _ in 0..2 {
        store
            .checkpoint_write(CheckpointWrite::EndedMetadata {
                operation: Box::new(operation.clone()),
                at: DateTime::from_timestamp(200, 0).unwrap(),
            })
            .await
            .unwrap();
    }
    let after = store.session(operation.session).await.unwrap().unwrap();
    assert_eq!(after.status, before.status);
    assert_eq!(after.ended_at, Some(ended_at));
    assert_eq!(after.harness_session, before.harness_session);
    assert_eq!(
        store.conversation(operation.session).await.unwrap(),
        conversation
    );
    assert_eq!(recorded_seq(&store, &operation).await, Some(3));
    let history = store
        .read_events(operation.session, EventSeq::ZERO, 10)
        .await
        .unwrap();
    assert_eq!(history.records.len(), 4);
    assert!(matches!(
        history.records[2].event,
        SessionEvent::Status { .. }
    ));
    assert!(
        matches!(history.records[3].event, SessionEvent::Checkpoint {
        event: CheckpointEvent::FilesRestored { restore, .. }
    } if restore == operation.id)
    );
    store.close().await;
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    store
        .checkpoint_write(CheckpointWrite::EndedMetadata {
            operation: Box::new(operation.clone()),
            at: DateTime::from_timestamp(300, 0).unwrap(),
        })
        .await
        .unwrap();
    assert_eq!(recorded_seq(&store, &operation).await, Some(3));
    let after = store.session(operation.session).await.unwrap().unwrap();
    assert_eq!(after.status, before.status);
    assert_eq!(after.ended_at, Some(ended_at));
    assert_eq!(
        store
            .read_events(operation.session, EventSeq::ZERO, 10)
            .await
            .unwrap(),
        history
    );
    store.close().await;
}

#[tokio::test]
async fn live_target_requires_actor_append_and_projects_the_same_restore_identity() {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let operation = completed(&store).await;
    assert!(
        store
            .checkpoint_write(CheckpointWrite::EndedMetadata {
                operation: Box::new(operation.clone()),
                at: DateTime::UNIX_EPOCH,
            })
            .await
            .is_err()
    );
    assert_eq!(recorded_seq(&store, &operation).await, None);
    let id = operation.id;
    let session = operation.session;
    let event = store.read(move |reader| reader.with(|connection| {
        let raw: String = connection.query_row(
            "SELECT c.checkpoint FROM checkpoint_captures c JOIN checkpoint_plans p ON p.capture=c.id JOIN checkpoint_restores r ON r.plan=p.id WHERE r.id=?1",
            [id.to_string()], |row| row.get(0),
        )?;
        Ok(CheckpointEvent::FilesRestored {
            restore: id, checkpoint: serde_json::from_str(&serde_json::to_string(&raw).unwrap()).unwrap(),
            paths: PathCount::new(1).unwrap(),
        })
    })).await.unwrap();
    let record = Arc::new(EventRecord {
        seq: EventSeq::ZERO,
        at: DateTime::UNIX_EPOCH,
        event: SessionEvent::Checkpoint { event },
    });
    store.append(session, vec![record.clone()]).await.unwrap();
    assert_eq!(recorded_seq(&store, &operation).await, Some(0));
    let duplicate = Arc::new(EventRecord {
        seq: EventSeq::new(1),
        ..(*record).clone()
    });
    assert!(store.append(session, vec![duplicate]).await.is_err());
    assert_eq!(
        store
            .read_events(session, EventSeq::ZERO, 10)
            .await
            .unwrap()
            .records
            .len(),
        1
    );
    store.close().await;
}

#[tokio::test]
async fn failed_event_marker_update_rolls_back_the_event_and_allows_one_retry() {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let operation = completed(&store).await;
    let _ended_at = ended(&store, &operation).await;
    let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    connection.execute_batch("CREATE TRIGGER deny_checkpoint_marker BEFORE UPDATE OF event_seq ON checkpoint_restores BEGIN SELECT RAISE(ABORT,'fixture marker failure'); END;").unwrap();
    drop(connection);
    assert!(
        store
            .checkpoint_write(CheckpointWrite::EndedMetadata {
                operation: Box::new(operation.clone()),
                at: DateTime::from_timestamp(200, 0).unwrap(),
            })
            .await
            .is_err()
    );
    assert_eq!(recorded_seq(&store, &operation).await, None);
    assert_eq!(
        store
            .read_events(operation.session, EventSeq::ZERO, 10)
            .await
            .unwrap()
            .records
            .len(),
        3
    );
    store.close().await;
    assert!(
        Store::open(&dir, StoreConfig::default()).await.is_err(),
        "startup metadata failure must not admit a partially projected node"
    );
    let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT count(*) FROM events WHERE session_id=?1",
            [operation.session.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        count, 3,
        "failed FULL-sync startup replay rolled back its event too"
    );
    let marker: Option<i64> = connection
        .query_row(
            "SELECT event_seq FROM checkpoint_restores WHERE id=?1",
            [operation.id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(marker.is_none());
    connection
        .execute_batch("DROP TRIGGER deny_checkpoint_marker")
        .unwrap();
    drop(connection);
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    store
        .checkpoint_write(CheckpointWrite::EndedMetadata {
            operation: Box::new(operation.clone()),
            at: DateTime::from_timestamp(300, 0).unwrap(),
        })
        .await
        .unwrap();
    assert_eq!(recorded_seq(&store, &operation).await, Some(3));
    assert_eq!(
        store
            .read_events(operation.session, EventSeq::ZERO, 10)
            .await
            .unwrap()
            .records
            .len(),
        4
    );
    store.close().await;
}
