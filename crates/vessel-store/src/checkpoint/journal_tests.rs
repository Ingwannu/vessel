//! Real SQLite tests for staged persistence, not a fake key/value store.
use super::{
    journal,
    records::{Error, Plan},
};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use vessel_proto::{
    DeviceId, EventSeq, SessionId, WorktreeId,
    checkpoint::{
        Boundary, CaptureGeneration, CaptureId, CaptureKey, CheckpointError, FileImage, RestoreId,
        RestoreOutcome, RestorePlanId,
    },
};

fn fixture() -> (Connection, CaptureKey) {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE sessions(id TEXT PRIMARY KEY);")
        .unwrap();
    super::install_staged(&connection).unwrap();
    let key = CaptureKey {
        capture: CaptureId::random(),
        session: SessionId::random(),
        worktree: WorktreeId::random(),
        boundary: Boundary::BeforeTurn {
            prompt: EventSeq::new(1),
        },
        generation: CaptureGeneration::FIRST,
    };
    connection
        .execute(
            "INSERT INTO sessions(id) VALUES (?1)",
            [key.session.to_string()],
        )
        .unwrap();
    journal::candidate(
        &connection,
        key,
        crate::ByteCount::new(1),
        DateTime::UNIX_EPOCH,
    )
    .unwrap();
    (connection, key)
}
fn ready_plan(connection: &Connection, key: CaptureKey) -> Plan {
    let empty = FileImage::new(Vec::new()).unwrap();
    journal::keep_candidate(connection, key, &empty).unwrap();
    // A fixture for the later node publication seam, not evidence of that seam.
    connection
        .execute(
            "UPDATE checkpoint_captures SET state='ready' WHERE id=?1",
            [key.capture.to_string()],
        )
        .unwrap();
    Plan {
        id: RestorePlanId::random(),
        key,
        device: DeviceId::random(),
        current: empty.clone(),
        desired: empty,
        expires: DateTime::<Utc>::MAX_UTC,
    }
}
#[test]
fn expired_and_restarted_candidates_cannot_be_persisted_as_ready_images() {
    let (connection, key) = fixture();
    journal::unavailable(&connection, key).unwrap();
    assert!(matches!(
        journal::keep_candidate(&connection, key, &FileImage::new(Vec::new()).unwrap()),
        Err(Error::Refused(CheckpointError::StaleCapture))
    ));
    let mut next = key;
    next.capture = CaptureId::random();
    journal::candidate(
        &connection,
        next,
        crate::ByteCount::new(1),
        DateTime::UNIX_EPOCH,
    )
    .unwrap();
    journal::invalidate_candidates(&connection).unwrap();
    assert!(matches!(
        journal::keep_candidate(&connection, next, &FileImage::new(Vec::new()).unwrap()),
        Err(Error::Refused(CheckpointError::StaleCapture))
    ));
    let reserved: i64 = connection
        .query_row(
            "SELECT sum(reserved_bytes) FROM checkpoint_captures",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        reserved, 2,
        "expiration/restart cannot return physical capacity before cleanup"
    );
}
#[test]
fn duplicate_restore_returns_the_committed_outcome_and_rejects_another_plan() {
    let (mut connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    journal::put_plan(&connection, &plan).unwrap();
    let restore = RestoreId::random();
    let transaction = connection.transaction().unwrap();
    let operation = journal::admit(&transaction, &plan, restore, DateTime::UNIX_EPOCH).unwrap();
    journal::outcome(&transaction, &operation, &RestoreOutcome::Unchanged).unwrap();
    transaction.commit().unwrap();
    assert_eq!(
        journal::status(&connection, restore, plan.device)
            .unwrap()
            .outcome,
        RestoreOutcome::Unchanged
    );
    assert_eq!(
        journal::admit(&connection, &plan, restore, plan.expires)
            .unwrap()
            .outcome,
        RestoreOutcome::Unchanged,
        "an identical retry queries its durable outcome even after plan expiry"
    );
    let mut other = plan.clone();
    other.id = RestorePlanId::random();
    assert!(matches!(
        journal::admit(&connection, &other, restore, DateTime::UNIX_EPOCH),
        Err(Error::Refused(CheckpointError::RestoreIdConflict))
    ));
    assert!(matches!(
        journal::status(&connection, restore, DeviceId::random()),
        Err(Error::Refused(CheckpointError::Forbidden))
    ));
}
#[test]
fn transaction_failure_does_not_consume_preview_or_create_a_restore() {
    let (mut connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    journal::put_plan(&connection, &plan).unwrap();
    let restore = RestoreId::random();
    {
        let transaction = connection.transaction().unwrap();
        journal::admit(&transaction, &plan, restore, DateTime::UNIX_EPOCH).unwrap();
        transaction.rollback().unwrap();
    }
    assert!(matches!(
        journal::status(&connection, restore, plan.device),
        Err(Error::Refused(CheckpointError::NotFound))
    ));
    assert!(journal::admit(&connection, &plan, restore, DateTime::UNIX_EPOCH).is_ok());
}
#[test]
fn altered_owner_or_effective_target_never_reuses_a_preview_authorization() {
    let (connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    journal::put_plan(&connection, &plan).unwrap();
    let mut wrong = plan.clone();
    wrong.device = DeviceId::random();
    assert!(matches!(
        journal::admit(
            &connection,
            &wrong,
            RestoreId::random(),
            DateTime::UNIX_EPOCH
        ),
        Err(Error::Refused(CheckpointError::Forbidden))
    ));
    connection
        .execute(
            "UPDATE checkpoint_plans SET expires_at=?1 WHERE id=?2",
            params![
                DateTime::<Utc>::UNIX_EPOCH.timestamp_millis(),
                plan.id.to_string()
            ],
        )
        .unwrap();
    assert!(matches!(
        journal::admit(
            &connection,
            &plan,
            RestoreId::random(),
            DateTime::UNIX_EPOCH
        ),
        Err(Error::Refused(CheckpointError::StalePreview))
    ));
}
