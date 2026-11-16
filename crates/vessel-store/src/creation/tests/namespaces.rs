//! Agent origins cannot collide with person keys or another parent's keys.

use super::*;
use sha2::{Digest as _, Sha256};
use vessel_proto::NodeId;

fn agent_candidate(identity: Identity) -> Admission {
    let receipt = Receipt::admitted(
        Kind::Agent,
        Reservation {
            operation: WorkspaceOperationId::random(),
            workspace: WorkspaceId::random(),
            workspace_claim: WorkspaceClaim::Existing,
            resource: Some(Resource::Agent {
                session: SessionId::random(),
            }),
            first_input: FirstInput::Absent,
        },
        Fingerprint::new(BlobHash::from_bytes([7; 32])),
    )
    .unwrap();
    Admission { identity, receipt }
}
#[tokio::test]
async fn identical_keys_in_person_and_distinct_origin_namespaces_reserve_distinct_operations() {
    let (_temp, dir, store) = open().await;
    let node = NodeId::random();
    let parent = SessionId::random();
    let key = Key::parse("same-key").unwrap();
    let identities = [
        Identity::new(Kind::Agent, key.clone()),
        Identity::for_agent(node, parent, key.clone()),
        Identity::for_agent(NodeId::random(), parent, key.clone()),
        Identity::for_agent(node, SessionId::random(), key),
    ];
    let mut operations = std::collections::HashSet::new();
    for identity in &identities {
        let receipt = store
            .admit_creation(agent_candidate(identity.clone()))
            .await
            .unwrap();
        assert!(operations.insert(receipt.snapshot().reserved.operation));
        let retry = store
            .admit_creation(agent_candidate(identity.clone()))
            .await
            .unwrap();
        assert_eq!(receipt.snapshot().reserved, retry.snapshot().reserved);
    }
    store.close().await;
    drop(store);
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    for identity in identities {
        assert!(store.creation(&identity).await.unwrap().is_some());
    }
    store.close().await;
}
#[tokio::test]
async fn old_person_key_hash_and_receipt_survive_the_owner_column_migration() {
    let (_temp, dir, store) = open().await;
    let admission = any_candidate("legacy-person-key");
    let identity = admission.identity.clone();
    let mut hash = Sha256::new();
    hash.update(b"workspace\0legacy-person-key");
    assert_eq!(
        identity.digest(),
        BlobHash::from_bytes(hash.finalize().into())
    );
    let original = store.admit_creation(admission).await.unwrap();
    store.close().await;
    drop(store);
    let db = rusqlite::Connection::open(dir.database_path()).unwrap();
    let latest: i64 = db
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    db.execute_batch("ALTER TABLE creation_receipts DROP COLUMN owner;")
        .unwrap();
    db.pragma_update(None, "user_version", latest.checked_sub(1).unwrap())
        .unwrap();
    drop(db);
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    assert_eq!(store.creation(&identity).await.unwrap().unwrap(), original);
    assert_eq!(
        store
            .admit_creation(any_candidate("legacy-person-key"))
            .await
            .unwrap(),
        original
    );
    store.close().await;
}
#[tokio::test]
async fn changed_owner_or_operation_index_is_corruption_not_an_absent_or_replayable_key() {
    for sql in [
        "UPDATE creation_receipts SET owner='{\"kind\":\"person\"}'",
        "UPDATE creation_receipts SET operation='00000000-0000-0000-0000-000000000000'",
        "DELETE FROM creation_claims",
    ] {
        let (_temp, dir, store) = open().await;
        let identity = Identity::for_agent(
            NodeId::random(),
            SessionId::random(),
            Key::parse("bound").unwrap(),
        );
        let _admitted = store
            .admit_creation(agent_candidate(identity.clone()))
            .await
            .unwrap();
        store.close().await;
        drop(store);
        let db = rusqlite::Connection::open(dir.database_path()).unwrap();
        db.execute_batch(sql).unwrap();
        drop(db);
        let store = Store::open(&dir, StoreConfig::default())
            .await
            .unwrap()
            .store;
        assert!(matches!(
            store.creation(&identity).await,
            Err(StoreError::Creation(
                CreationError::NamespaceCorrupt | CreationError::ClaimsCorrupt
            ))
        ));
        assert!(
            store
                .admit_creation(agent_candidate(identity))
                .await
                .is_err()
        );
        store.close().await;
    }
}
