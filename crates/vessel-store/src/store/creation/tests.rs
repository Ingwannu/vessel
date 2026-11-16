//! Observers wake after commit, even when the originating caller disappeared.

use super::*;
use crate::creation::tests::{any_candidate, open};
use vessel_proto::creation::Stage;

#[tokio::test]
async fn a_committed_ack_returns_its_own_marker_even_after_a_later_commit() {
    let (_temp, _dir, store) = open().await;
    let admitted = store
        .admit_creation(any_candidate("exact-ack"))
        .await
        .unwrap();
    let operation = admitted.snapshot().reserved.operation;
    let expected = admitted.snapshot().revision;
    let (begin_done, begin) = tokio::sync::oneshot::channel();
    store
        .send(Write::Creation {
            change: Box::new(CreationWrite::Advance {
                operation,
                expected,
                update: Advance::Begin(Stage::Workspace),
            }),
            done: begin_done,
        })
        .await
        .unwrap();
    let (finish_done, finish) = tokio::sync::oneshot::channel();
    store
        .send(Write::Creation {
            change: Box::new(CreationWrite::Advance {
                operation,
                expected: expected.next().unwrap(),
                update: Advance::Finished(Stage::Workspace),
            }),
            done: finish_done,
        })
        .await
        .unwrap();
    let finished = finish.await.unwrap().unwrap();
    assert_eq!(finished.in_flight(), None);
    let marker = begin.await.unwrap().unwrap();
    assert_eq!(marker.in_flight(), Some(Stage::Workspace));
    assert_eq!(marker.snapshot().revision, expected.next().unwrap());
    assert_eq!(
        store.creation_operation(operation).await.unwrap().unwrap(),
        finished
    );
    store.close().await;
}

#[tokio::test]
async fn dropped_ack_receiver_cannot_suppress_a_committed_creation_wakeup() {
    let (_temp, _dir, store) = open().await;
    let mut changes = store.creation_changes();
    let candidate = any_candidate("gone-caller");
    let identity = candidate.identity.clone();
    let (done, receive) = tokio::sync::oneshot::channel();
    drop(receive);
    store
        .writes
        .send(crate::writer::WriteOp::Write(Write::Creation {
            change: Box::new(CreationWrite::Admit(candidate)),
            done,
        }))
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), changes.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(store.creation(&identity).await.unwrap().is_some());
    store.close().await;
}

#[tokio::test]
async fn failed_savepoint_neither_advances_the_receipt_nor_notifies_observers() {
    let (_temp, _dir, store) = open().await;
    let admitted = store.admit_creation(any_candidate("known")).await.unwrap();
    let changes = store.creation_changes();
    let operation = admitted.snapshot().reserved.operation;
    assert!(
        store
            .advance_creation(operation, admitted.snapshot().revision, Advance::Complete)
            .await
            .is_err()
    );
    assert!(!changes.has_changed().unwrap());
    assert_eq!(
        store.creation_operation(operation).await.unwrap().unwrap(),
        admitted
    );
    store.close().await;
}

#[tokio::test]
async fn subscribe_before_snapshot_preserves_changes_while_the_snapshot_is_read() {
    let (_temp, _dir, store) = open().await;
    let admitted = store
        .admit_creation(any_candidate("subscribe"))
        .await
        .unwrap();
    let mut changes = store.creation_changes();
    let operation = admitted.snapshot().reserved.operation;
    let next = store
        .advance_creation(
            operation,
            admitted.snapshot().revision,
            Advance::Begin(Stage::Workspace),
        )
        .await
        .unwrap();
    assert!(changes.has_changed().unwrap());
    changes.changed().await.unwrap();
    assert_eq!(
        store.creation_operation(operation).await.unwrap().unwrap(),
        next
    );
    store.close().await;
}
