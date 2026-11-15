//! Admission and CAS use the real store writer; no external process is started.

use super::*;
use crate::{DataDir, Store, StoreConfig, StoreError};
use vessel_proto::creation::{
    Certainty, CreationError, Fingerprint, FirstInput, Phase, Reservation, Resource, Stage,
    WorkspaceClaim,
};
use vessel_proto::{BlobHash, ErrorCode, SessionId, WorkspaceId};

mod namespaces;
mod saturation;

fn candidate(key: &str, fingerprint: u8, workspace: WorkspaceId, session: SessionId) -> Admission {
    let identity = Identity::new(Kind::Workspace, Key::parse(key).unwrap());
    let receipt = Receipt::admitted(
        Kind::Workspace,
        Reservation {
            operation: WorkspaceOperationId::random(),
            workspace,
            workspace_claim: WorkspaceClaim::Create,
            resource: Some(Resource::Agent { session }),
            first_input: FirstInput::Identified {
                message: vessel_proto::timeline::ClientMessageId::random(),
            },
        },
        Fingerprint::new(BlobHash::from_bytes([fingerprint; 32])),
    )
    .unwrap();
    Admission { identity, receipt }
}

pub(crate) async fn open() -> (tempfile::TempDir, DataDir, Store) {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    (temp, dir, store)
}

pub(crate) fn any_candidate(key: &str) -> Admission {
    candidate(key, 7, WorkspaceId::random(), SessionId::random())
}

#[tokio::test]
async fn concurrent_same_key_intent_returns_one_reserved_operation() {
    let (_temp, _dir, store) = open().await;
    let first = any_candidate("same-key");
    let reserved = first.receipt.snapshot().reserved.clone();
    let second = any_candidate("same-key");
    let (one, two) = tokio::join!(store.admit_creation(first), store.admit_creation(second));
    assert_eq!(one.unwrap().snapshot().reserved, reserved);
    assert_eq!(two.unwrap().snapshot().reserved, reserved);
    let conflicting = candidate("same-key", 8, WorkspaceId::random(), SessionId::random());
    assert!(matches!(
        store.admit_creation(conflicting).await,
        Err(StoreError::Creation(CreationError::IntentConflict))
    ));
    store.close().await;
}

#[tokio::test]
async fn colliding_claim_rolls_back_all_earlier_claims_in_the_admission() {
    let (_temp, _dir, store) = open().await;
    let session = SessionId::random();
    let first = candidate("first", 7, WorkspaceId::random(), session);
    let _admitted = store.admit_creation(first).await.unwrap();
    let workspace = WorkspaceId::random();
    let colliding = candidate("second", 7, workspace, session);
    let identity = colliding.identity.clone();
    assert!(matches!(
        store.admit_creation(colliding).await,
        Err(StoreError::Creation(CreationError::ResourceClaimed))
    ));
    assert_eq!(store.creation(&identity).await.unwrap(), None);
    let recovered = candidate("third", 7, workspace, SessionId::random());
    assert!(
        store.admit_creation(recovered).await.is_ok(),
        "the failed operation did not retain the workspace claim"
    );
    store.close().await;
}

#[tokio::test]
async fn cas_admits_one_side_effect_owner_and_reopening_marks_it_unknown() {
    let (_temp, dir, store) = open().await;
    let admission = any_candidate("restart");
    let identity = admission.identity.clone();
    let admitted = store.admit_creation(admission).await.unwrap();
    let operation = admitted.snapshot().reserved.operation;
    let revision = admitted.snapshot().revision;
    let (one, two) = tokio::join!(
        store.advance_creation(operation, revision, Advance::Begin(Stage::Workspace)),
        store.advance_creation(operation, revision, Advance::Begin(Stage::Workspace))
    );
    assert_eq!(
        usize::from(one.is_ok()).saturating_add(usize::from(two.is_ok())),
        1
    );
    let failed = if let Err(error) = one {
        error
    } else {
        two.unwrap_err()
    };
    assert!(matches!(
        failed,
        StoreError::Creation(CreationError::StaleRevision)
    ));
    store.close().await;
    drop(store);
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let recovered = store.creation(&identity).await.unwrap().unwrap();
    assert_eq!(recovered.snapshot().reserved, admitted.snapshot().reserved);
    assert_eq!(
        recovered.snapshot().phase,
        Phase::Failed {
            stage: Stage::Workspace,
            certainty: Certainty::Unknown,
            code: ErrorCode::Unavailable
        }
    );
    assert!(matches!(
        store
            .advance_creation(
                operation,
                recovered.snapshot().revision,
                Advance::Begin(Stage::Workspace)
            )
            .await,
        Err(StoreError::Creation(CreationError::InvalidTransition))
    ));
    store.close().await;
}

#[tokio::test]
async fn prompt_failure_and_existing_workspace_membership_do_not_reclaim_registration() {
    let (_temp, dir, store) = open().await;
    let admitted = store.admit_creation(any_candidate("prompt")).await.unwrap();
    let operation = admitted.snapshot().reserved.operation;
    let mut current = admitted;
    for stage in [Stage::Workspace, Stage::Resource] {
        current = store
            .advance_creation(
                operation,
                current.snapshot().revision,
                Advance::Begin(stage),
            )
            .await
            .unwrap();
        current = store
            .advance_creation(
                operation,
                current.snapshot().revision,
                Advance::Finished(stage),
            )
            .await
            .unwrap();
    }
    current = store
        .advance_creation(
            operation,
            current.snapshot().revision,
            Advance::Begin(Stage::Prompt),
        )
        .await
        .unwrap();
    current = store
        .advance_creation(
            operation,
            current.snapshot().revision,
            Advance::Fail {
                code: ErrorCode::Conflict,
                certainty: Certainty::Known,
            },
        )
        .await
        .unwrap();
    let workspace = current.snapshot().reserved.workspace;
    let original = current.snapshot().reserved.resource;
    let extra = Receipt::admitted(
        Kind::Agent,
        Reservation {
            operation: WorkspaceOperationId::random(),
            workspace,
            workspace_claim: WorkspaceClaim::Existing,
            resource: Some(Resource::Agent {
                session: SessionId::random(),
            }),
            first_input: FirstInput::Absent,
        },
        Fingerprint::new(BlobHash::from_bytes([0; 32])),
    )
    .unwrap();
    assert!(
        store
            .admit_creation(Admission {
                identity: Identity::new(Kind::Agent, Key::parse("another-chat").unwrap()),
                receipt: extra
            })
            .await
            .is_ok()
    );
    store.close().await;
    drop(store);
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    current = store
        .advance_creation(
            operation,
            current.snapshot().revision,
            Advance::Begin(Stage::Prompt),
        )
        .await
        .unwrap();
    assert_eq!(current.snapshot().reserved.resource, original);
    store.close().await;
}

#[test]
fn corrupt_receipts_are_refused_instead_of_skipped_or_treated_as_absent() {
    let mut connection = rusqlite::Connection::open_in_memory().unwrap();
    crate::schema::migrate(&mut connection).unwrap();
    let identity = Identity::new(
        Kind::Workspace,
        Key::parse("synthetic-key-not-a-token").unwrap(),
    );
    let _changed = connection
        .execute(
            "INSERT INTO creation_receipts(key_hash,operation,reserved_bytes,receipt) VALUES (?1,?2,?3,?4)",
            rusqlite::params![
                identity.digest().to_string(),
                WorkspaceOperationId::random().to_string(),
                i64::try_from(vessel_proto::creation::RESERVED_BYTES_PER_RECEIPT).unwrap(),
                "broken"
            ],
        )
        .unwrap();
    assert!(matches!(
        lookup(&connection, &identity),
        Err(StoreError::CorruptCreation(_))
    ));
    let raw: String = connection
        .query_row("SELECT key_hash FROM creation_receipts", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert!(!raw.contains("synthetic-key"));
}
