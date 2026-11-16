//! Force both SQLite read/write orders through the actual Store recheck.
//! The temporary view observes only the authority SELECT on one private pooled
//! reader; it changes no production callback, SQL policy or writer behavior.
use super::*;
use rusqlite::functions::FunctionFlags;
use std::sync::{Mutex, mpsc};
use std::time::Duration;
use tokio::sync::oneshot;

const SNAPSHOT_DEADLINE: Duration = Duration::from_secs(5);

async fn pause_authority_read(store: &Store) -> (oneshot::Receiver<()>, mpsc::SyncSender<()>) {
    let (entered, observed) = oneshot::channel();
    let (release, released) = mpsc::sync_channel(1);
    store.read(move |reader| reader.with(move |connection| {
        let pause = Mutex::new(Some((entered, released)));
        connection.create_scalar_function("a5_observe_scopes", 1, FunctionFlags::SQLITE_UTF8, move |context| {
            let signals = pause.lock().unwrap().take();
            if let Some((entered, released)) = signals {
                entered.send(()).map_err(|()| rusqlite::Error::UserFunctionError(Box::new(std::io::Error::other("authority observation abandoned"))))?;
                released.recv_timeout(SNAPSHOT_DEADLINE).map_err(|error| rusqlite::Error::UserFunctionError(Box::new(error)))?;
            }
            context.get::<String>(0)
        })?;
        // Shadow only this reader's devices name. Writes still use the real
        // canonical table on the existing writer, with actual WAL commits.
        connection.execute_batch("CREATE TEMP VIEW devices AS SELECT id,revoked_at,a5_observe_scopes(scopes) AS scopes FROM main.devices")?;
        Ok(())
    })).await.unwrap();
    (observed, release)
}

async fn seen_source() -> (Fixture, crate::PushAttention, StoredPush) {
    let fixture = Fixture::new().await;
    let source = fixture.finished(Creator::Owner).await;
    let lease = fixture.lease().await;
    assert!(
        fixture
            .store
            .notification_current(NotificationAuthority::Device(lease.device), source)
            .await
            .unwrap()
    );
    assert!(
        fixture
            .store
            .notification_current(NotificationAuthority::Owner, source)
            .await
            .unwrap()
    );
    fixture
        .store
        .mark_inbox(MarkInbox {
            key: source.key,
            read: InboxRead::Read,
        })
        .await
        .unwrap();
    assert!(
        !fixture
            .store
            .notification_current(NotificationAuthority::Device(lease.device), source)
            .await
            .unwrap()
    );
    (fixture, source, lease)
}

async fn revoke_then_unread(fixture: &Fixture, lease: &StoredPush, source: crate::PushAttention) {
    // There is never a jointly eligible state: while Read authority is live
    // the source is Seen; it becomes Unread only after revoke has committed.
    fixture
        .store
        .revoke_device(lease.device, fixture.now)
        .await
        .unwrap();
    fixture
        .store
        .mark_inbox(MarkInbox {
            key: source.key,
            read: InboxRead::Unread,
        })
        .await
        .unwrap();
}

async fn assert_fresh_controls(
    fixture: &Fixture,
    lease: &StoredPush,
    source: crate::PushAttention,
) {
    assert!(
        !fixture
            .store
            .notification_current(NotificationAuthority::Device(lease.device), source)
            .await
            .unwrap()
    );
    assert!(
        fixture
            .store
            .notification_current(NotificationAuthority::Owner, source)
            .await
            .unwrap()
    );
    assert!(matches!(
        fixture
            .store
            .claim_notification(NotificationAuthority::Device(lease.device), source)
            .await,
        Err(StoreError::NotificationDenied)
    ));
}

#[tokio::test]
async fn a_local_recheck_cannot_mix_pre_revoke_authority_with_post_revoke_unread() {
    let (fixture, source, lease) = seen_source().await;
    let (observed, release) = pause_authority_read(&fixture.store).await;
    let store = fixture.store.clone();
    let authority = NotificationAuthority::Device(lease.device);
    let checking = tokio::spawn(async move { store.notification_current(authority, source).await });
    tokio::time::timeout(SNAPSHOT_DEADLINE, observed)
        .await
        .expect("the actual authority SELECT is paused")
        .unwrap();
    revoke_then_unread(&fixture, &lease, source).await;
    release.try_send(()).unwrap();
    assert!(
        !tokio::time::timeout(SNAPSHOT_DEADLINE, checking)
            .await
            .expect("the original read snapshot settles")
            .unwrap()
            .unwrap()
    );
    assert_fresh_controls(&fixture, &lease, source).await;
    fixture.store.close().await;
}

#[tokio::test]
async fn a_recheck_started_after_revoke_uses_the_new_view_and_owner_stays_independent() {
    let (fixture, source, lease) = seen_source().await;
    revoke_then_unread(&fixture, &lease, source).await;
    assert_fresh_controls(&fixture, &lease, source).await;
    fixture.store.close().await;
}
