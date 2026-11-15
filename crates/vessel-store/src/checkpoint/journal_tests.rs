//! Real `SQLite` tests for staged persistence, not a fake key/value store.
use super::{
    journal,
    records::{CheckpointStoreError as Error, Plan},
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
    let _changed = connection
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
    super::publication::activate(
        connection,
        key,
        vessel_proto::checkpoint::CheckpointId::random(),
    )
    .unwrap();
    Plan {
        id: RestorePlanId::random(),
        key,
        device: DeviceId::random(),
        current: empty.clone(),
        desired: empty
            .effective(
                &empty,
                vessel_proto::checkpoint::FileMetadata {
                    owner: vessel_proto::checkpoint::OwnerId::new(1000),
                    group: vessel_proto::checkpoint::GroupId::new(1000),
                    mode: vessel_proto::checkpoint::OrdinaryMode::new(0o600).unwrap(),
                },
            )
            .unwrap(),
        expires: DateTime::<Utc>::UNIX_EPOCH
            .checked_add_signed(
                chrono::TimeDelta::from_std(vessel_proto::checkpoint::CHECKPOINT_PREVIEW_LIFETIME)
                    .unwrap(),
            )
            .unwrap(),
    }
}
#[test]
fn expired_and_restarted_candidates_cannot_be_persisted_as_ready_images() {
    let (connection, key) = fixture();
    journal::unavailable(&connection, key, CheckpointError::CaptureTimedOut).unwrap();
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
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    let restore = RestoreId::random();
    let transaction = connection.transaction().unwrap();
    let operation = journal::admit(&transaction, &plan, restore, DateTime::UNIX_EPOCH).unwrap();
    journal::outcome(&transaction, &operation, &RestoreOutcome::Unchanged).unwrap();
    transaction.commit().unwrap();
    assert_eq!(
        journal::status(&connection, key.session, restore, plan.device)
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
        journal::status(&connection, key.session, restore, DeviceId::random()),
        Err(Error::Refused(CheckpointError::Forbidden))
    ));
}
#[test]
fn transaction_failure_does_not_consume_preview_or_create_a_restore() {
    let (mut connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    let restore = RestoreId::random();
    {
        let transaction = connection.transaction().unwrap();
        let _operation =
            journal::admit(&transaction, &plan, restore, DateTime::UNIX_EPOCH).unwrap();
        transaction.rollback().unwrap();
    }
    assert!(matches!(
        journal::status(&connection, key.session, restore, plan.device),
        Err(Error::Refused(CheckpointError::NotFound))
    ));
    assert!(journal::admit(&connection, &plan, restore, DateTime::UNIX_EPOCH).is_ok());
}
#[test]
fn altered_owner_or_effective_target_never_reuses_a_preview_authorization() {
    let (connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
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
    let _changed = connection
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

#[test]
fn a_late_callback_cannot_replace_a_terminal_result_or_a_different_session() {
    let (connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    let restore = RestoreId::random();
    let operation = journal::admit(&connection, &plan, restore, DateTime::UNIX_EPOCH).unwrap();
    journal::outcome(&connection, &operation, &RestoreOutcome::Unchanged).unwrap();
    assert!(matches!(
        journal::outcome(
            &connection,
            &operation,
            &RestoreOutcome::RecoveryRequired { restore }
        ),
        Err(Error::Refused(CheckpointError::RestoreIdConflict))
    ));
    assert!(matches!(
        journal::status(&connection, SessionId::random(), restore, plan.device),
        Err(Error::Refused(CheckpointError::Forbidden))
    ));
}
#[test]
fn restart_marks_pending_restores_recovery_without_discarding_before_images() {
    let (connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    let restore = RestoreId::random();
    let operation = journal::admit(&connection, &plan, restore, DateTime::UNIX_EPOCH).unwrap();
    assert_eq!(super::recovery::pending(&connection).unwrap(), 1);
    let recovered = journal::status(&connection, key.session, restore, plan.device).unwrap();
    assert_eq!(
        recovered.outcome,
        RestoreOutcome::RecoveryRequired { restore }
    );
    assert_eq!(recovered.before, operation.before);
    assert_eq!(super::recovery::pending(&connection).unwrap(), 0);
    assert!(
        connection
            .execute(
                "DELETE FROM sessions WHERE id=?1",
                [key.session.to_string()]
            )
            .is_err(),
        "recovery data must remain pinned"
    );
}
#[test]
fn quota_counts_obsolete_candidates_until_physical_cleanup() {
    let (connection, key) = fixture();
    journal::unavailable(&connection, key, CheckpointError::CaptureTimedOut).unwrap();
    let mut next = key;
    next.capture = CaptureId::random();
    assert!(matches!(
        journal::candidate(
            &connection,
            next,
            crate::ByteCount::new(vessel_proto::checkpoint::CHECKPOINT_NODE_BYTES),
            DateTime::UNIX_EPOCH
        ),
        Err(Error::Refused(CheckpointError::StorageFull))
    ));
}

#[test]
fn restore_admission_reloads_capture_identity_instead_of_trusting_the_caller() {
    let (connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    let mut forged = plan.clone();
    forged.key.session = SessionId::random();
    assert!(matches!(
        journal::admit(
            &connection,
            &forged,
            RestoreId::random(),
            DateTime::UNIX_EPOCH
        ),
        Err(Error::Refused(CheckpointError::Forbidden))
    ));
    for forged_key in [
        CaptureKey {
            worktree: WorktreeId::random(),
            ..key
        },
        CaptureKey {
            capture: CaptureId::random(),
            ..key
        },
        CaptureKey {
            generation: key.generation.next().unwrap(),
            ..key
        },
        CaptureKey {
            boundary: Boundary::AfterTurn {
                ended: EventSeq::new(1),
            },
            ..key
        },
    ] {
        forged.key = forged_key;
        assert!(matches!(
            journal::admit(
                &connection,
                &forged,
                RestoreId::random(),
                DateTime::UNIX_EPOCH
            ),
            Err(Error::Refused(CheckpointError::StalePreview))
        ));
    }
    let count: i64 = connection
        .query_row("SELECT count(*) FROM checkpoint_restores", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(
        count, 0,
        "a forged association cannot consume or begin a restore"
    );
    assert!(
        journal::admit(
            &connection,
            &plan,
            RestoreId::random(),
            DateTime::UNIX_EPOCH
        )
        .is_ok()
    );
}

#[test]
fn terminal_outcomes_require_the_complete_approved_path_journal() {
    use super::paths::{self, Phase};
    use vessel_proto::{
        BlobHash,
        checkpoint::{Entry, FileBytes, FileState, PathCount},
    };
    let (mut connection, key) = fixture();
    let mut plan = ready_plan(&connection, key);
    let metadata = vessel_proto::checkpoint::FileMetadata {
        owner: vessel_proto::checkpoint::OwnerId::new(1000),
        group: vessel_proto::checkpoint::GroupId::new(1000),
        mode: vessel_proto::checkpoint::OrdinaryMode::new(0o600).unwrap(),
    };
    let path =
        serde_json::from_str::<vessel_proto::checkpoint::CheckpointPath>("\"file\"").unwrap();
    let selected = FileImage::new(vec![Entry {
        path: path.clone(),
        state: FileState::Present {
            digest: BlobHash::from_bytes([1; 32]),
            bytes: FileBytes::new(1).unwrap(),
            metadata,
        },
    }])
    .unwrap();
    plan.desired = selected.effective(&plan.current, metadata).unwrap();
    let _changed = connection
        .execute(
            "UPDATE checkpoint_captures SET image=?1 WHERE id=?2",
            params![
                serde_json::to_string(&selected).unwrap(),
                key.capture.to_string()
            ],
        )
        .unwrap();
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    let restore = RestoreId::random();
    let operation = {
        let transaction = connection.transaction().unwrap();
        let operation = journal::admit(&transaction, &plan, restore, DateTime::UNIX_EPOCH).unwrap();
        paths::prepare(&transaction, &operation).unwrap();
        transaction.commit().unwrap();
        operation
    };
    let entries = paths::list(&connection, &operation).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries.first().unwrap().before, FileState::Absent);
    assert_eq!(entries.first().unwrap().phase, Phase::Pending);
    let success = RestoreOutcome::Restored {
        changed: PathCount::new(1).unwrap(),
    };
    assert!(matches!(
        journal::outcome(&connection, &operation, &success),
        Err(Error::Refused(CheckpointError::RecoveryPending))
    ));
    assert!(matches!(
        journal::outcome(&connection, &operation, &RestoreOutcome::Unchanged),
        Err(Error::Refused(CheckpointError::RecoveryPending))
    ));
    paths::advance(
        &connection,
        &operation,
        &path,
        Phase::Pending,
        Phase::Applied,
    )
    .unwrap();
    assert!(matches!(
        paths::advance(
            &connection,
            &operation,
            &path,
            Phase::Pending,
            Phase::Applied
        ),
        Err(Error::Refused(CheckpointError::RestoreIdConflict))
    ));
    paths::advance(
        &connection,
        &operation,
        &path,
        Phase::Applied,
        Phase::Restored,
    )
    .unwrap();
    assert!(matches!(
        journal::outcome(&connection, &operation, &success),
        Err(Error::Refused(CheckpointError::RecoveryPending))
    ));
    journal::outcome(
        &connection,
        &operation,
        &RestoreOutcome::RolledBack {
            reason: CheckpointError::CaptureUnstable,
        },
    )
    .unwrap();
    assert!(matches!(
        paths::advance(
            &connection,
            &operation,
            &path,
            Phase::Restored,
            Phase::Conflict
        ),
        Err(Error::Refused(CheckpointError::RestoreIdConflict))
    ));
}

#[test]
fn candidate_persistence_is_not_ready_and_activation_cannot_revive_expired_or_restarted_rows() {
    use super::publication;
    use vessel_proto::checkpoint::CheckpointId;
    let (connection, key) = fixture();
    let empty = FileImage::new(Vec::new()).unwrap();
    assert!(matches!(
        publication::activate(&connection, key, CheckpointId::random()),
        Err(Error::Refused(CheckpointError::StaleCapture))
    ));
    journal::keep_candidate(&connection, key, &empty).unwrap();
    let ready: bool = connection
        .query_row(
            "SELECT state='ready' FROM checkpoint_captures WHERE id=?1",
            [key.capture.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!ready);
    let mut wrong = key;
    wrong.generation = wrong.generation.next().unwrap();
    assert!(matches!(
        publication::activate(&connection, wrong, CheckpointId::random()),
        Err(Error::Refused(CheckpointError::StaleCapture))
    ));
    journal::unavailable(&connection, key, CheckpointError::CaptureTimedOut).unwrap();
    assert!(matches!(
        publication::activate(&connection, key, CheckpointId::random()),
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
    journal::keep_candidate(&connection, next, &empty).unwrap();
    journal::invalidate_candidates(&connection).unwrap();
    assert!(matches!(
        publication::activate(&connection, next, CheckpointId::random()),
        Err(Error::Refused(CheckpointError::StaleCapture))
    ));
}

#[test]
fn preview_pages_share_one_target_expiry_and_device_authorization() {
    use super::changes;
    use vessel_proto::{
        BlobHash,
        checkpoint::{
            CHECKPOINT_PAGE_ENTRIES, ChangeCursor, Entry, FileBytes, FileState, PathCount,
        },
    };
    let (connection, key) = fixture();
    let mut plan = ready_plan(&connection, key);
    let metadata = vessel_proto::checkpoint::FileMetadata {
        owner: vessel_proto::checkpoint::OwnerId::new(1000),
        group: vessel_proto::checkpoint::GroupId::new(1000),
        mode: vessel_proto::checkpoint::OrdinaryMode::new(0o600).unwrap(),
    };
    let count = CHECKPOINT_PAGE_ENTRIES.saturating_mul(2).saturating_add(1);
    let selected = FileImage::new(
        (0..count)
            .map(|index| Entry {
                path: serde_json::from_str(
                    &serde_json::to_string(&format!("file-{index:04}")).unwrap(),
                )
                .unwrap(),
                state: FileState::Present {
                    digest: BlobHash::from_bytes([0; 32]),
                    bytes: FileBytes::new(0).unwrap(),
                    metadata,
                },
            })
            .collect(),
    )
    .unwrap();
    plan.desired = selected.effective(&plan.current, metadata).unwrap();
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    let mut cursor = None;
    let mut paths = Vec::new();
    loop {
        let page = changes::page(
            &connection,
            plan.id,
            plan.device,
            cursor,
            DateTime::UNIX_EPOCH,
        )
        .unwrap();
        assert_eq!(page.plan, plan.id);
        assert_eq!(page.expires, plan.expires);
        assert!(page.changes.len() <= CHECKPOINT_PAGE_ENTRIES);
        paths.extend(page.changes.into_iter().map(|change| change.path));
        cursor = page.next;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(
        paths,
        plan.desired
            .image()
            .entries()
            .iter()
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>()
    );
    assert!(matches!(
        changes::page(
            &connection,
            plan.id,
            DeviceId::random(),
            None,
            DateTime::UNIX_EPOCH
        ),
        Err(Error::Refused(CheckpointError::Forbidden))
    ));
    assert!(matches!(
        changes::page(
            &connection,
            plan.id,
            plan.device,
            Some(ChangeCursor(PathCount::new(1).unwrap())),
            DateTime::UNIX_EPOCH
        ),
        Err(Error::Refused(CheckpointError::InvalidCursor))
    ));
    assert!(matches!(
        changes::page(&connection, plan.id, plan.device, None, plan.expires),
        Err(Error::Refused(CheckpointError::StalePreview))
    ));
    let _operation = journal::admit(
        &connection,
        &plan,
        RestoreId::random(),
        DateTime::UNIX_EPOCH,
    )
    .unwrap();
    assert!(matches!(
        changes::page(
            &connection,
            plan.id,
            plan.device,
            None,
            DateTime::UNIX_EPOCH
        ),
        Err(Error::Refused(CheckpointError::StalePreview))
    ));
}

#[test]
fn preview_admission_enforces_lifetime_and_per_device_pin_limit() {
    let (connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    let mut next = plan.clone();
    for _ in 0..vessel_proto::checkpoint::CHECKPOINT_PREVIEWS_PER_DEVICE {
        next.id = RestorePlanId::random();
        journal::put_plan(&connection, &next, DateTime::UNIX_EPOCH).unwrap();
    }
    next.id = RestorePlanId::random();
    assert!(matches!(
        journal::put_plan(&connection, &next, DateTime::UNIX_EPOCH),
        Err(Error::Refused(CheckpointError::Busy))
    ));
    next.device = DeviceId::random();
    next.expires = DateTime::<Utc>::MAX_UTC;
    assert!(matches!(
        journal::put_plan(&connection, &next, DateTime::UNIX_EPOCH),
        Err(Error::Refused(CheckpointError::StalePreview))
    ));
    next.expires = DateTime::UNIX_EPOCH;
    assert!(matches!(
        journal::put_plan(&connection, &next, DateTime::UNIX_EPOCH),
        Err(Error::Refused(CheckpointError::StalePreview))
    ));
    next.expires = plan.expires;
    journal::put_plan(&connection, &next, DateTime::UNIX_EPOCH).unwrap();
}

#[test]
fn empty_effective_target_is_unchanged_and_recovery_cannot_name_another_operation() {
    let (connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    let operation = journal::admit(
        &connection,
        &plan,
        RestoreId::random(),
        DateTime::UNIX_EPOCH,
    )
    .unwrap();
    assert!(matches!(
        journal::outcome(
            &connection,
            &operation,
            &RestoreOutcome::Restored {
                changed: vessel_proto::checkpoint::PathCount::new(0).unwrap()
            }
        ),
        Err(Error::Refused(CheckpointError::RecoveryPending))
    ));
    assert!(matches!(
        journal::outcome(
            &connection,
            &operation,
            &RestoreOutcome::RecoveryRequired {
                restore: RestoreId::random()
            }
        ),
        Err(Error::Refused(CheckpointError::RecoveryPending))
    ));
    journal::outcome(&connection, &operation, &RestoreOutcome::Unchanged).unwrap();
}
