//! Two real clients racing use one store transaction and one elected recipient.
use super::super::{PresenceId, Push};
use super::candidates::{finished, session};
use vessel_proto::inbox::ListInbox;
use vessel_proto::notifications::NotificationClaim;
use vessel_proto::push::{PushActivity, PushFocus, ReportPushPresence};
use vessel_store::{DataDir, NotificationAuthority, PushAttention, Store, StoreConfig};

#[tokio::test]
async fn only_elected_connection_can_consume_a_source_and_consumption_is_once() {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let target = session(&store).await;
    finished(&store, target, 0).await;
    let entry = store
        .inbox(ListInbox::default())
        .await
        .unwrap()
        .entries
        .remove(0);
    let source = PushAttention {
        key: entry.key,
        cursor: entry.cursor,
    };
    let push = Push::new(store.clone(), dir);
    let first = PresenceId::new();
    let second = PresenceId::new();
    for caller in [first, second] {
        let _watch = push
            .present(
                caller,
                ReportPushPresence {
                    idle_age: None,
                    focus: PushFocus::Elsewhere,
                    activity: PushActivity::Active,
                },
            )
            .unwrap();
    }
    let elected = if push.recipient(first, target) == NotificationClaim::Admitted {
        first
    } else {
        second
    };
    let other = if elected == first { second } else { first };
    assert_eq!(
        push.claim_session(other, NotificationAuthority::Owner, source)
            .await
            .unwrap(),
        NotificationClaim::NotRecipient
    );
    let (a, b) = tokio::join!(
        push.claim_session(elected, NotificationAuthority::Owner, source),
        push.claim_session(elected, NotificationAuthority::Owner, source)
    );
    let results = [a.unwrap(), b.unwrap()];
    assert_eq!(
        results
            .iter()
            .filter(|result| **result == NotificationClaim::Admitted)
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| **result == NotificationClaim::AlreadyClaimed)
            .count(),
        1
    );
    push.forget(elected);
    assert_eq!(
        push.claim_session(other, NotificationAuthority::Owner, source)
            .await
            .unwrap(),
        NotificationClaim::AlreadyClaimed
    );
    store.close().await;
}
#[tokio::test]
async fn visible_source_suppresses_claim_without_spending_the_opportunity() {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let target = session(&store).await;
    finished(&store, target, 0).await;
    let entry = store
        .inbox(ListInbox::default())
        .await
        .unwrap()
        .entries
        .remove(0);
    let source = PushAttention {
        key: entry.key,
        cursor: entry.cursor,
    };
    let push = Push::new(store.clone(), dir);
    let caller = PresenceId::new();
    assert_eq!(
        push.claim_session(caller, NotificationAuthority::Owner, source)
            .await
            .unwrap(),
        NotificationClaim::NoPresentClient
    );
    let _watch = push
        .present(
            caller,
            ReportPushPresence {
                idle_age: None,
                focus: PushFocus::Session { session: target },
                activity: PushActivity::Active,
            },
        )
        .unwrap();
    assert_eq!(
        push.claim_session(caller, NotificationAuthority::Owner, source)
            .await
            .unwrap(),
        NotificationClaim::Suppressed
    );
    let _watch = push
        .present(
            caller,
            ReportPushPresence {
                idle_age: None,
                focus: PushFocus::Elsewhere,
                activity: PushActivity::Unchanged,
            },
        )
        .unwrap();
    assert_eq!(
        push.claim_session(caller, NotificationAuthority::Owner, source)
            .await
            .unwrap(),
        NotificationClaim::Admitted
    );
    store.close().await;
}
