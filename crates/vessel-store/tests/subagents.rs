//! Durable detach never replaces a managed child's identity or history.

use chrono::Utc;
use vessel_proto::{
    Creator, EventSeq, Mark, NodeId, SessionId, SessionRef, SessionStatus,
    subagents::{DetachDisposition, DetachReceipt, Parentage},
};
use vessel_store::{StoreConfig, StoreError};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(dead_code, reason = "shared fixtures also serve log tests")]
mod fixtures;
use fixtures::{at, data_dir, new_session, open, status};

#[tokio::test]
async fn first_repeat_and_restart_keep_the_original_receipt_and_log() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let parent = SessionRef {
        node: NodeId::random(),
        session: SessionId::random(),
    };
    let info = new_session(0).with_creator(Creator::Agent {
        node: parent.node,
        session: parent.session,
    });
    store.create_session(info.clone()).await.unwrap();
    store
        .append(info.id, vec![status(0, SessionStatus::Running)])
        .await
        .unwrap();
    let first = store.detach(info.id, at(1)).await.unwrap();
    assert_eq!(first.disposition, DetachDisposition::Detached);
    assert_eq!(first.receipt.former_parent(), parent);
    assert_eq!(first.receipt.detached_at(), at(1));
    assert_eq!(first.session.id, info.id);
    assert_eq!(first.session.spec, info.spec);
    assert_eq!(first.session.created_by, info.created_by);
    assert_eq!(first.session.next_seq, EventSeq::new(1));
    let repeated = store.detach(info.id, at(2)).await.unwrap();
    assert_eq!(repeated.disposition, DetachDisposition::AlreadyDetached);
    assert_eq!(repeated.receipt, first.receipt);
    store
        .append(
            info.id,
            vec![status(1, SessionStatus::Exited { code: Some(0) })],
        )
        .await
        .unwrap();
    let _marks = store.mark(info.id, Mark::Archive, at(3)).await.unwrap();
    let _marks = store.reopen(info.id, EventSeq::new(2)).await.unwrap();
    assert_eq!(
        store
            .session(info.id)
            .await
            .unwrap()
            .unwrap()
            .detach_receipt,
        Some(first.receipt)
    );
    store.close().await;
    let store = open(&dir, StoreConfig::default()).await;
    let after_restart = store.detach(info.id, at(4)).await.unwrap();
    assert_eq!(after_restart.receipt, first.receipt);
    assert_eq!(
        after_restart.disposition,
        DetachDisposition::AlreadyDetached
    );
    assert_eq!(after_restart.session.created_by, info.created_by);
    assert_eq!(after_restart.session.spec, info.spec);
    assert_eq!(
        after_restart.session.parentage().unwrap(),
        Parentage::Detached(first.receipt)
    );
}

#[tokio::test]
async fn concurrent_writers_commit_exactly_one_timestamp() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let info = new_session(0).with_creator(Creator::Agent {
        node: NodeId::random(),
        session: SessionId::random(),
    });
    store.create_session(info.clone()).await.unwrap();
    let (a, b) = tokio::join!(store.detach(info.id, at(1)), store.detach(info.id, at(2)));
    let a = a.unwrap();
    let b = b.unwrap();
    assert_eq!(a.receipt, b.receipt);
    assert_ne!(a.disposition, b.disposition);
    assert_eq!(
        store
            .session(info.id)
            .await
            .unwrap()
            .unwrap()
            .detach_receipt,
        Some(a.receipt)
    );
}

#[tokio::test]
async fn never_child_is_a_definite_refusal_without_a_receipt() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let info = new_session(0);
    store.create_session(info.clone()).await.unwrap();
    assert!(
        matches!(store.detach(info.id, Utc::now()).await,Err(StoreError::NotChild(id)) if id==info.id)
    );
    assert_eq!(
        store
            .session(info.id)
            .await
            .unwrap()
            .unwrap()
            .detach_receipt,
        None
    );
}

#[tokio::test]
async fn a_receipt_for_another_parent_is_corrupt_and_never_overwritten() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let info = new_session(0).with_creator(Creator::Agent {
        node: NodeId::random(),
        session: SessionId::random(),
    });
    store.create_session(info.clone()).await.unwrap();
    let receipt = DetachReceipt::new(
        SessionRef {
            node: NodeId::random(),
            session: SessionId::random(),
        },
        at(1),
    );
    let db = rusqlite::Connection::open(dir.database_path()).unwrap();
    let changed = db
        .execute(
            "UPDATE sessions SET detach_receipt=?2 WHERE id=?1",
            rusqlite::params![
                info.id.to_string(),
                serde_json::to_string(&receipt).unwrap()
            ],
        )
        .unwrap();
    assert_eq!(changed, 1);
    assert!(
        matches!(store.session(info.id).await,Err(StoreError::InvalidParentage(id)) if id==info.id)
    );
    assert!(
        matches!(store.detach(info.id, at(2)).await,Err(StoreError::InvalidParentage(id)) if id==info.id)
    );
    let kept: String = db
        .query_row(
            "SELECT detach_receipt FROM sessions WHERE id=?1",
            [info.id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<DetachReceipt>(&kept).unwrap(),
        receipt
    );
}
