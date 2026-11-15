//! Small injected budgets exercise the same serialized SQL path as production.

use super::*;
use vessel_proto::creation::{
    Advance, Certainty, Fingerprint, FirstInput, Kind, Phase, Reservation, Revision, Stage,
    WorkspaceClaim,
};
use vessel_proto::{BlobHash, ErrorCode, WorkspaceId};

const COUNT_LIMIT: usize = 2;
const BYTE_SLOTS: usize = 2;
const ROOM: usize = RESERVED_BYTES_PER_RECEIPT.saturating_mul(BYTE_SLOTS);

fn budget() -> Budget {
    Budget {
        count: COUNT_LIMIT,
        retained: ROOM,
        row: MAX_RECEIPT_BYTES,
    }
}

fn database() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    crate::schema::migrate(&mut db).unwrap();
    db
}

fn candidate(key: &str) -> Admission {
    let receipt = Receipt::admitted(
        Kind::Workspace,
        Reservation {
            operation: WorkspaceOperationId::random(),
            workspace: WorkspaceId::random(),
            workspace_claim: WorkspaceClaim::Create,
            resource: None,
            first_input: FirstInput::Absent,
        },
        Fingerprint::new(BlobHash::from_bytes([7; 32])),
    )
    .unwrap();
    Admission {
        identity: Identity::new(
            Kind::Workspace,
            vessel_proto::creation::Key::parse(key).unwrap(),
        ),
        receipt,
    }
}

fn apply_committed(db: &mut Connection, write: Write, budget: Budget) -> Result<(), StoreError> {
    let tx = db.transaction()?;
    let _receipt = apply_with(&tx, write, budget)?;
    tx.commit()?;
    Ok(())
}

#[test]
fn count_and_byte_saturation_keep_existing_keys_and_refuse_new_keys_without_claims() {
    let byte_budget = Budget {
        count: MAX_RECEIPTS,
        retained: RESERVED_BYTES_PER_RECEIPT,
        ..budget()
    };
    for (limits, count) in [(budget(), COUNT_LIMIT), (byte_budget, 1)] {
        let mut db = database();
        let first = candidate("first");
        let identity = first.identity.clone();
        let original = first.receipt.snapshot().reserved.clone();
        apply_committed(&mut db, Write::Admit(first), limits).unwrap();
        for index in 1..count {
            apply_committed(
                &mut db,
                Write::Admit(candidate(&format!("key-{index}"))),
                limits,
            )
            .unwrap();
        }
        let refused = candidate("too-many");
        let refused_id = refused.receipt.snapshot().reserved.operation;
        assert!(matches!(
            apply_committed(&mut db, Write::Admit(refused), limits),
            Err(StoreError::Creation(CreationError::Full))
        ));
        let claims: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM creation_claims WHERE operation=?1",
                [refused_id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(claims, 0);
        apply_committed(&mut db, Write::Admit(candidate("first")), limits).unwrap();
        assert_eq!(
            lookup(&db, &identity).unwrap().unwrap().snapshot().reserved,
            original
        );
    }
}

#[test]
fn future_growth_and_restart_recovery_fit_the_admitted_slot_while_full() {
    let mut db = database();
    let limits = Budget {
        count: 1,
        retained: RESERVED_BYTES_PER_RECEIPT,
        ..budget()
    };
    let admission = candidate("existing");
    let identity = admission.identity.clone();
    apply_committed(&mut db, Write::Admit(admission), limits).unwrap();
    let mut current = lookup(&db, &identity).unwrap().unwrap();
    let id = current.snapshot().reserved.operation;
    apply_committed(
        &mut db,
        Write::Advance {
            operation: id,
            expected: current.snapshot().revision,
            update: Advance::Begin(Stage::Workspace),
        },
        limits,
    )
    .unwrap();
    recover(&mut db).unwrap();
    current = lookup(&db, &identity).unwrap().unwrap();
    assert_eq!(
        current.snapshot().phase,
        Phase::Failed {
            stage: Stage::Workspace,
            certainty: Certainty::Unknown,
            code: ErrorCode::Unavailable
        }
    );
    assert!(encode(&current, MAX_RECEIPT_BYTES).is_ok());
    apply_committed(&mut db, Write::Admit(candidate("existing")), limits).unwrap();
    assert!(matches!(
        apply_committed(
            &mut db,
            Write::Advance {
                operation: id,
                expected: current.snapshot().revision,
                update: Advance::Begin(Stage::Workspace)
            },
            limits
        ),
        Err(StoreError::Creation(CreationError::InvalidTransition))
    ));
    assert!(matches!(
        apply_committed(&mut db, Write::Admit(candidate("new")), limits),
        Err(StoreError::Creation(CreationError::Full))
    ));
}

#[test]
fn oversized_later_mutation_retains_the_previous_row_and_revision() {
    let mut db = database();
    let admission = candidate("growth");
    let identity = admission.identity.clone();
    apply_committed(&mut db, Write::Admit(admission), budget()).unwrap();
    let original = lookup(&db, &identity).unwrap().unwrap();
    let exact = codec::encode(&original).unwrap().len();
    let limits = Budget {
        row: exact,
        ..budget()
    };
    let id = original.snapshot().reserved.operation;
    assert!(matches!(
        apply_committed(
            &mut db,
            Write::Advance {
                operation: id,
                expected: Revision::INITIAL,
                update: Advance::Begin(Stage::Workspace)
            },
            limits
        ),
        Err(StoreError::Creation(CreationError::ReceiptTooLarge))
    ));
    assert_eq!(lookup(&db, &identity).unwrap().unwrap(), original);
}

#[test]
fn corrupted_reservation_accounting_is_not_a_free_slot_or_an_absent_key() {
    let mut db = database();
    let admission = candidate("accounting");
    let identity = admission.identity.clone();
    apply_committed(&mut db, Write::Admit(admission), budget()).unwrap();
    let _changed = db
        .execute("UPDATE creation_receipts SET reserved_bytes=0", [])
        .unwrap();
    assert!(matches!(
        lookup(&db, &identity),
        Err(StoreError::Creation(CreationError::CapacityCorrupt))
    ));
    assert!(matches!(
        apply_committed(&mut db, Write::Admit(candidate("new")), budget()),
        Err(StoreError::Creation(CreationError::CapacityCorrupt))
    ));
}

#[test]
fn existing_key_conflict_is_not_hidden_by_exhausted_capacity() {
    let mut db = database();
    apply_committed(&mut db, Write::Admit(candidate("bound")), budget()).unwrap();
    let mut conflicting = candidate("bound");
    conflicting.receipt = Receipt::admitted(
        Kind::Workspace,
        conflicting.receipt.snapshot().reserved.clone(),
        Fingerprint::new(BlobHash::from_bytes([8; 32])),
    )
    .unwrap();
    let full = Budget {
        count: 0,
        retained: 0,
        ..budget()
    };
    assert!(matches!(
        apply_committed(&mut db, Write::Admit(conflicting), full),
        Err(StoreError::Creation(CreationError::IntentConflict))
    ));
}
