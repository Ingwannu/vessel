//! Current labels are a projection distinct from the creation spec, and
//! an invalid stored map must be corruption rather than missing metadata.

use vessel_proto::SessionLabels;
use vessel_store::{Column, StoreConfig, StoreError};

#[cfg(test)]
#[path = "support/fixtures.rs"]
mod fixtures;

fn labels(text: &str) -> SessionLabels {
    serde_json::from_str(text).unwrap()
}

#[tokio::test]
async fn current_labels_are_stored_independently_from_creation_labels() {
    let (_temp, dir) = fixtures::data_dir();
    let store = fixtures::open(&dir, StoreConfig::default()).await;
    let mut session = fixtures::new_session(0);
    session.spec.labels = labels(r#"{"env":"initial"}"#);
    let current = labels(r#"{"env":"updated","empty":""}"#);
    session.labels = current.clone();
    store.create_session(session.clone()).await.unwrap();
    store.close().await;
    let store = fixtures::open(&dir, StoreConfig::default()).await;
    let restored = store.session(session.id).await.unwrap().unwrap();
    assert_eq!(restored.labels, current);
    assert_eq!(restored.spec.labels, session.spec.labels);
    store.close().await;
}

#[tokio::test]
async fn an_invalid_label_column_is_not_accepted_as_an_empty_map() {
    let (_temp, dir) = fixtures::data_dir();
    let store = fixtures::open(&dir, StoreConfig::default()).await;
    let session = fixtures::new_session(0);
    store.create_session(session.clone()).await.unwrap();
    let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    connection
        .execute(
            "UPDATE sessions SET labels = ?1 WHERE id = ?2",
            rusqlite::params![r#"{"vessel.reserved":"value"}"#, session.id.to_string()],
        )
        .unwrap();
    assert!(matches!(
        store.session(session.id).await,
        Err(StoreError::Corrupt {
            column: Column::Labels,
            ..
        })
    ));
    store.close().await;
}
