//! Pull request feedback's record (protocol 11, schema 10, ADR 0029):
//! turned on per session, each event sent once across restarts, bounded,
//! and gone with its session.

use build::{session, signatures};
use fixtures::{at, data_dir, open, status};
use vessel_proto::{SessionId, SessionStatus};
use vessel_store::{MAX_FEEDBACK_SENT, StoreConfig, StoredFeedback};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(
    dead_code,
    reason = "builders shared with the log's tests; this file uses some"
)]
mod fixtures;

/// Builders, in a `cfg(test)` module for the lints' test allowances.
#[cfg(test)]
mod build {
    use vessel_proto::SessionId;
    use vessel_store::Store;

    use super::fixtures::new_session;

    pub(crate) async fn session(store: &Store) -> SessionId {
        let session = new_session(0);
        store.create_session(session.clone()).await.unwrap();
        session.id
    }

    pub(crate) fn signatures(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }
}

#[tokio::test]
async fn feedback_is_off_until_turned_on_and_survives_a_restart() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let id = session(&store).await;
    assert_eq!(store.feedback(id).await.unwrap(), StoredFeedback::default());

    store.set_feedback(id, true).await.unwrap();
    store.set_feedback(id, true).await.unwrap();
    store
        .feedback_sent(id, signatures(&["check:a@1", "review:b@1"]), at(1))
        .await
        .unwrap();
    store
        .feedback_sent(id, signatures(&["review:b@1"]), at(2))
        .await
        .unwrap();
    store.close().await;
    drop(store);

    let store = open(&dir, StoreConfig::default()).await;
    assert_eq!(
        store.feedback(id).await.unwrap(),
        StoredFeedback {
            on: true,
            sent: signatures(&["check:a@1", "review:b@1"]),
        }
    );
    // Off keeps what was sent: on again sends nothing twice.
    store.set_feedback(id, false).await.unwrap();
    let off = store.feedback(id).await.unwrap();
    assert!(!off.on);
    assert_eq!(off.sent.len(), 2);
    store
        .forget_feedback_sent(id, signatures(&["check:a@1", "never-sent"]))
        .await
        .unwrap();
    assert_eq!(
        store.feedback(id).await.unwrap().sent,
        signatures(&["review:b@1"])
    );
}

#[tokio::test]
async fn at_most_the_newest_signatures_are_kept() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let id = session(&store).await;
    let first: Vec<String> = (0..MAX_FEEDBACK_SENT)
        .map(|n| format!("old:{n:05}"))
        .collect();
    store.feedback_sent(id, first, at(1)).await.unwrap();
    store
        .feedback_sent(id, signatures(&["new:a", "new:b"]), at(2))
        .await
        .unwrap();
    let kept = store.feedback(id).await.unwrap().sent;
    assert_eq!(kept.len(), MAX_FEEDBACK_SENT);
    assert!(kept.iter().any(|signature| signature == "new:a"));
    assert!(kept.iter().any(|signature| signature == "new:b"));
}

#[tokio::test]
async fn feedback_goes_with_its_session() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let id = session(&store).await;
    store.set_feedback(id, true).await.unwrap();
    store
        .feedback_sent(id, signatures(&["check:a@1"]), at(1))
        .await
        .unwrap();
    store
        .append(
            id,
            vec![
                status(0, SessionStatus::Starting),
                status(1, SessionStatus::Exited { code: Some(0) }),
            ],
        )
        .await
        .unwrap();
    store.delete_session(id).await.unwrap();
    assert_eq!(store.feedback(id).await.unwrap(), StoredFeedback::default());
    // A session the store does not have takes no feedback.
    assert!(store.set_feedback(SessionId::random(), true).await.is_err());
}
