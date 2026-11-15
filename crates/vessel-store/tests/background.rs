//! Discovery is durable metadata, independent of the creation spec and run state.

use fixtures::{data_dir, new_session, open};
use vessel_proto::{SessionListing, SessionStatus};
use vessel_store::{Column, StoreConfig, StoreError};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(
    dead_code,
    reason = "only shared session and store fixture builders are needed"
)]
mod fixtures;

#[tokio::test]
async fn listing_survives_reopening_without_becoming_a_state_or_spec_override() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let mut background = new_session(0);
    background.listing = SessionListing::Background;
    let shown = new_session(1);
    for info in [&background, &shown] {
        store.create_session(info.clone()).await.unwrap();
    }
    store.close().await;
    drop(store);
    let store = open(&dir, StoreConfig::default()).await;
    let found = store.session(background.id).await.unwrap().unwrap();
    assert_eq!(found.listing, SessionListing::Background);
    assert_eq!(found.spec.listing, None);
    assert_eq!(found.status, SessionStatus::Interrupted);
    assert_eq!(
        store.session(shown.id).await.unwrap().unwrap().listing,
        SessionListing::Shown
    );
    assert_eq!(store.sessions().await.unwrap().len(), 2);
    store.close().await;
    drop(store);
    let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    let changed = connection
        .execute(
            "UPDATE sessions SET listing='invalid' WHERE id=?1",
            [background.id.to_string()],
        )
        .unwrap();
    assert_eq!(changed, 1);
    drop(connection);
    let store = open(&dir, StoreConfig::default()).await;
    let error = store.session(background.id).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Corrupt { session, column: Column::Listing, .. } if session == background.id)
    );
}
