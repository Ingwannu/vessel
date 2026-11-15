//! Starting an ended session again (protocol 10's `session.resume`, ADR
//! 0026): its row is reopened before its next run writes, retention leaves
//! it alone from then on, and its log continues after its final status.

use std::sync::Arc;

use fixtures::{at, data_dir, new_session, open, record, status};
use vessel_proto::{
    ContentBlock, Creator, EventRecord, EventSeq, Mark, PromptQueue, QueueEntryId, QueueHold,
    QueuedPrompt, SessionEvent, SessionId, SessionStatus,
};
use vessel_store::{Retention, Store, StoreConfig, StoreError};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(
    dead_code,
    reason = "builders shared with the log's tests; this file uses some"
)]
mod fixtures;

/// Builders for this file. A `cfg(test)` module so the lints' test
/// allowances apply.
#[cfg(test)]
mod support {
    use super::*;

    /// A stored session that ran and ended: records 0 to 2, so its log
    /// continues at 3.
    pub(crate) async fn ended(store: &Store, created: i64) -> SessionId {
        let session = new_session(created);
        store.create_session(session.clone()).await.unwrap();
        store
            .append(
                session.id,
                vec![
                    status(0, SessionStatus::Starting),
                    status(1, SessionStatus::Running),
                    status(2, SessionStatus::Exited { code: Some(0) }),
                ],
            )
            .await
            .unwrap();
        session.id
    }

    pub(crate) fn queue() -> PromptQueue {
        PromptQueue {
            entries: vec![QueuedPrompt {
                provenance: None,
                id: QueueEntryId::random(),
                content: vec![ContentBlock::Text {
                    text: "later".to_owned(),
                }],
                queued_at: at(1),
                revision: 0,
                by: Creator::Owner,

                client_message_id: None,
            }],
            held: Some(QueueHold::Ended),
        }
    }
}

use support::{ended, queue};

const NEXT: EventSeq = EventSeq::new(3);

/// The row is reopened live, without an end and out of the archive, its
/// marks otherwise kept; the log continues at its next record and ends
/// again with the run's final status.
#[tokio::test]
async fn a_reopened_session_continues_its_log() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = new_session(0);
    let id = session.id;
    store.create_session(session).await.unwrap();
    let queued = queue();
    store
        .append(
            id,
            vec![
                status(0, SessionStatus::Starting),
                status(1, SessionStatus::Running),
                record(
                    2,
                    SessionEvent::QueueChanged {
                        queue: queued.clone(),
                    },
                ),
                status(3, SessionStatus::Exited { code: Some(0) }),
            ],
        )
        .await
        .unwrap();
    let _kept = store.mark(id, Mark::Pin, at(5)).await.unwrap();
    let _kept = store.mark(id, Mark::Archive, at(6)).await.unwrap();

    let stored = store.ended_session(id).await.unwrap().unwrap();
    assert_eq!(stored.info.next_seq, EventSeq::new(4));
    assert_eq!(stored.queue, queued, "the queue as the log left it");
    assert!(!stored.info.status.is_live());
    assert!(stored.info.marks.archived_at.is_some());

    let reopened = store.reopen(id, stored.info.next_seq).await.unwrap();
    assert!(reopened.pinned, "kept");
    assert_eq!(reopened.archived_at, None, "out of the archive");
    let info = store.session(id).await.unwrap().unwrap();
    assert_eq!(info.status, SessionStatus::Starting);
    assert_eq!(info.ended_at, None);
    assert_eq!(info.board, None, "the node places a live session");

    store
        .append(
            id,
            vec![
                status(4, SessionStatus::Starting),
                status(5, SessionStatus::Running),
                Arc::new(EventRecord {
                    seq: EventSeq::new(6),
                    at: at(20),
                    event: SessionEvent::Status {
                        status: SessionStatus::Exited { code: Some(0) },
                    },
                }),
            ],
        )
        .await
        .unwrap();
    let info = store.session(id).await.unwrap().unwrap();
    assert_eq!(info.next_seq, EventSeq::new(7));
    assert_eq!(info.ended_at, Some(at(20)), "the second run's end");
    let page = store.read_events(id, EventSeq::ZERO, 100).await.unwrap();
    let statuses: Vec<_> = page
        .records
        .iter()
        .filter_map(|record| {
            if let SessionEvent::Status { status } = &record.event {
                Some(status.clone())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        statuses,
        [
            SessionStatus::Starting,
            SessionStatus::Running,
            SessionStatus::Exited { code: Some(0) },
            SessionStatus::Starting,
            SessionStatus::Running,
            SessionStatus::Exited { code: Some(0) },
        ],
        "a final status ends a run, not the log"
    );
}

/// A row that retention or `session.delete` removed is not reopened, nor
/// one still live, nor one whose log does not end where the caller read
/// it; a refusal changes nothing.
#[tokio::test]
async fn only_an_ended_row_where_it_was_read_is_reopened() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let deleted = ended(&store, 0).await;
    store.delete_session(deleted).await.unwrap();
    assert!(matches!(
        store.reopen(deleted, NEXT).await,
        Err(StoreError::UnknownSession(id)) if id == deleted
    ));
    assert!(matches!(
        store.reopen(SessionId::random(), NEXT).await,
        Err(StoreError::UnknownSession(_))
    ));

    let moved = ended(&store, 1).await;
    assert!(matches!(
        store.reopen(moved, EventSeq::new(2)).await,
        Err(StoreError::OutOfOrder { expected, got, .. })
            if expected == NEXT && got == EventSeq::new(2)
    ));
    assert!(
        !store
            .session(moved)
            .await
            .unwrap()
            .unwrap()
            .status
            .is_live()
    );

    let _kept = store.reopen(moved, NEXT).await.unwrap();
    assert!(matches!(
        store.reopen(moved, NEXT).await,
        Err(StoreError::SessionLive(id)) if id == moved
    ));
    assert!(matches!(
        store.delete_session(moved).await,
        Err(StoreError::SessionLive(_))
    ));
}

/// Retention keeps a reopened session however many others end after it:
/// it prunes only ended rows.
#[tokio::test]
async fn retention_leaves_a_session_starting_again() {
    let (_temp, dir) = data_dir();
    let config = StoreConfig {
        retention: Retention {
            max_ended_sessions: 1,
            ..Retention::default()
        },
        ..StoreConfig::default()
    };
    let store = open(&dir, config).await;
    let again = ended(&store, 0).await;
    let _kept = store.reopen(again, NEXT).await.unwrap();
    let first = ended(&store, 1).await;
    let second = ended(&store, 2).await;
    assert!(
        store.session(first).await.unwrap().is_none(),
        "one ended session kept"
    );
    assert!(store.session(second).await.unwrap().is_some());
    assert!(
        store.session(again).await.unwrap().is_some(),
        "the one starting again is not pruned"
    );
    let stored = store.ended_session(again).await.unwrap().unwrap();
    assert_eq!(stored.info.next_seq, NEXT);
    assert!(stored.info.status.is_live());
}
