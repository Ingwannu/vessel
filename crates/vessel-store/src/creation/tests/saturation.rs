//! Last-slot concurrency runs through the production writer after a full restart.

use super::*;
use vessel_proto::creation::{MAX_RECEIPTS, MAX_RETAINED_BYTES, RESERVED_BYTES_PER_RECEIPT};

/// Fixture bulk loading avoids an admission scan for every preparatory row.
fn seed_before_last_slot(dir: &DataDir) -> Result<(), StoreError> {
    let mut db = rusqlite::Connection::open(dir.database_path())?;
    let tx = db.transaction()?;
    // Only complete retained reservations fit; no partial last slot is admitted.
    let slots = MAX_RECEIPTS.min(
        MAX_RETAINED_BYTES
            .checked_div(RESERVED_BYTES_PER_RECEIPT)
            .unwrap(),
    );
    for index in 0..slots.saturating_sub(1) {
        let admission = any_candidate(&format!("retained-{index}"));
        let reserved = admission.receipt.snapshot().reserved.clone();
        let _changed = tx.execute(
            "INSERT INTO creation_receipts(key_hash,operation,reserved_bytes,receipt) VALUES (?1,?2,?3,?4)",
            rusqlite::params![admission.identity.digest().to_string(), reserved.operation.to_string(),
                i64::try_from(RESERVED_BYTES_PER_RECEIPT).map_err(|_overflow| CreationError::CapacityCorrupt)?,
                crate::codec::encode(&admission.receipt)?],
        )?;
        for (kind, resource) in [
            ("workspace", reserved.workspace.to_string()),
            (
                "agent",
                match reserved.resource {
                    Some(Resource::Agent { session }) => session.to_string(),
                    Some(Resource::Terminal { terminal }) => terminal.to_string(),
                    None => return Err(CreationError::InvalidTransition.into()),
                },
            ),
        ] {
            let _changed = tx.execute(
                "INSERT INTO creation_claims(kind,resource,operation) VALUES (?1,?2,?3)",
                rusqlite::params![kind, resource, reserved.operation.to_string()],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

#[tokio::test]
async fn concurrent_last_slot_has_one_winner_and_reconciliation_survives_full_restart() {
    let (_temp, dir, store) = open().await;
    store.close().await;
    drop(store);
    seed_before_last_slot(&dir).unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let one = any_candidate("last-one");
    let two = any_candidate("last-two");
    let identities = [one.identity.clone(), two.identity.clone()];
    let (one, two) = tokio::join!(store.admit_creation(one), store.admit_creation(two));
    let (winner, identity, loser) = match (one, two) {
        (Ok(winner), Err(StoreError::Creation(CreationError::Full))) => {
            (winner, &identities[0], &identities[1])
        }
        (Err(StoreError::Creation(CreationError::Full)), Ok(winner)) => {
            (winner, &identities[1], &identities[0])
        }
        other => panic!("one slot must admit exactly one candidate: {other:?}"),
    };
    assert_eq!(store.creation(loser).await.unwrap(), None);
    let operation = winner.snapshot().reserved.operation;
    let _started = store
        .advance_creation(
            operation,
            winner.snapshot().revision,
            Advance::Begin(Stage::Workspace),
        )
        .await
        .unwrap();
    store.close().await;
    drop(store);
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let retained = store.creation_operation(operation).await.unwrap().unwrap();
    assert_eq!(retained.snapshot().reserved, winner.snapshot().reserved);
    assert!(matches!(
        retained.snapshot().phase,
        Phase::Failed {
            certainty: Certainty::Unknown,
            ..
        }
    ));
    let retried = store
        .admit_creation(Admission {
            identity: identity.clone(),
            receipt: any_candidate("unused").receipt,
        })
        .await
        .unwrap();
    assert_eq!(retried, retained);
    assert!(matches!(
        store.admit_creation(any_candidate("after-restart")).await,
        Err(StoreError::Creation(CreationError::Full))
    ));
    store.close().await;
}

#[tokio::test]
async fn deleting_a_completed_resource_does_not_release_its_key_or_claim() {
    let (_temp, dir, store) = open().await;
    let admission = any_candidate("completed");
    let identity = admission.identity.clone();
    let reserved = admission.receipt.snapshot().reserved.clone();
    let Some(Resource::Agent { session }) = reserved.resource else {
        panic!("agent fixture required");
    };
    let info = vessel_proto::SessionInfo::new(
        session,
        vessel_proto::SessionSpec::new(
            vessel_proto::HarnessKind::Codex,
            vessel_proto::SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    );
    let mut current = store.admit_creation(admission).await.unwrap();
    store.create_session(info).await.unwrap();
    // Store creation always starts a live log. End it through the real status
    // projection before exercising deletion; changing SessionInfo is not one.
    store
        .append(
            session,
            vec![std::sync::Arc::new(vessel_proto::EventRecord {
                seq: vessel_proto::EventSeq::new(0),
                at: chrono::DateTime::UNIX_EPOCH,
                event: vessel_proto::SessionEvent::Status {
                    status: vessel_proto::SessionStatus::Exited { code: Some(0) },
                },
            })],
        )
        .await
        .unwrap();
    assert!(matches!(
        store.session(session).await.unwrap().unwrap().status,
        vessel_proto::SessionStatus::Exited { code: Some(0) }
    ));
    for stage in [Stage::Workspace, Stage::Resource, Stage::Prompt] {
        current = store
            .advance_creation(
                reserved.operation,
                current.snapshot().revision,
                Advance::Begin(stage),
            )
            .await
            .unwrap();
        current = store
            .advance_creation(
                reserved.operation,
                current.snapshot().revision,
                Advance::Finished(stage),
            )
            .await
            .unwrap();
    }
    current = store
        .advance_creation(
            reserved.operation,
            current.snapshot().revision,
            Advance::Complete,
        )
        .await
        .unwrap();
    store.delete_session(session).await.unwrap();
    store.close().await;
    drop(store);
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let retried = store
        .admit_creation(Admission {
            identity,
            receipt: any_candidate("unused").receipt,
        })
        .await
        .unwrap();
    assert_eq!(retried, current);
    assert_eq!(store.session(session).await.unwrap(), None);
    assert!(matches!(
        store
            .admit_creation(candidate("new-key", 7, WorkspaceId::random(), session))
            .await,
        Err(StoreError::Creation(CreationError::ResourceClaimed))
    ));
    store.close().await;
}
