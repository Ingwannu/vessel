//! Real `SQLite` tests for staged persistence, not a fake key/value store.
use super::{
    journal,
    records::{CheckpointStoreError as Error, Plan},
};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use vessel_proto::{
    EventSeq, SessionId, WorktreeId,
    checkpoint::{
        Boundary, CaptureGeneration, CaptureId, CaptureKey, CheckpointError, CheckpointOwner,
        FileImage, RestoreId, RestoreOutcome, RestorePlanId,
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
    journal::keep_candidate(connection, key, &super::tests::observed(empty.clone())).unwrap();
    super::publication::activate(
        connection,
        key,
        vessel_proto::checkpoint::CheckpointId::random(),
    )
    .unwrap();
    super::quota::model_without_worker(connection, key).unwrap();
    Plan {
        id: RestorePlanId::random(),
        key,
        owner: CheckpointOwner::Device {
            device: vessel_proto::DeviceId::random(),
        },
        evidence: super::tests::evidence(),
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
fn selected_files(count: usize) -> (FileImage, vessel_proto::checkpoint::FileMetadata) {
    use vessel_proto::{
        BlobHash,
        checkpoint::{
            CheckpointFileBytes, Entry, FileMetadata, FileState, GroupId, OrdinaryMode, OwnerId,
        },
    };
    let metadata = FileMetadata {
        owner: OwnerId::new(1000),
        group: GroupId::new(1000),
        mode: OrdinaryMode::new(0o600).unwrap(),
    };
    let selected = FileImage::new(
        (0..count)
            .map(|index| Entry {
                path: serde_json::from_str(
                    &serde_json::to_string(&format!("file-{index:04}")).unwrap(),
                )
                .unwrap(),
                state: FileState::Present {
                    digest: BlobHash::from_bytes([0; 32]),
                    bytes: CheckpointFileBytes::new(0).unwrap(),
                    metadata,
                },
            })
            .collect(),
    )
    .unwrap();
    (selected, metadata)
}
#[test]
fn expired_and_restarted_candidates_cannot_be_persisted_as_ready_images() {
    let (connection, key) = fixture();
    journal::unavailable(&connection, key, CheckpointError::CaptureTimedOut).unwrap();
    assert!(matches!(
        journal::keep_candidate(
            &connection,
            key,
            &super::tests::observed(FileImage::new(Vec::new()).unwrap())
        ),
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
        journal::keep_candidate(
            &connection,
            next,
            &super::tests::observed(FileImage::new(Vec::new()).unwrap())
        ),
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
        journal::status(&connection, key.session, restore, plan.owner)
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
        journal::status(
            &connection,
            key.session,
            restore,
            CheckpointOwner::Device {
                device: vessel_proto::DeviceId::random()
            }
        ),
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
        journal::status(&connection, key.session, restore, plan.owner),
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
    wrong.owner = CheckpointOwner::Device {
        device: vessel_proto::DeviceId::random(),
    };
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
        journal::status(&connection, SessionId::random(), restore, plan.owner),
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
    let recovered = journal::status(&connection, key.session, restore, plan.owner).unwrap();
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
    use vessel_proto::checkpoint::{FileState, PathCount};
    let (mut connection, key) = fixture();
    let mut plan = ready_plan(&connection, key);
    let (selected, metadata) = selected_files(1);
    let path = selected.entries().first().unwrap().path.clone();
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
    assert!(
        paths::advance(
            &connection,
            &operation,
            &path,
            Phase::Pending,
            Phase::Applied
        )
        .is_err(),
        "no filesystem mutation may precede committed staged evidence"
    );
    super::staging::commit(&connection, &operation, &super::tests::staged(&plan)).unwrap();
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
    journal::keep_candidate(&connection, key, &super::tests::observed(empty.clone())).unwrap();
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
    journal::keep_candidate(&connection, next, &super::tests::observed(empty.clone())).unwrap();
    journal::invalidate_candidates(&connection).unwrap();
    assert!(matches!(
        publication::activate(&connection, next, CheckpointId::random()),
        Err(Error::Refused(CheckpointError::StaleCapture))
    ));
}

#[test]
fn preview_pages_share_one_target_expiry_and_device_authorization() {
    use super::changes;
    use vessel_proto::checkpoint::{CHECKPOINT_PAGE_ENTRIES, ChangeCursor, PathCount};
    let (connection, key) = fixture();
    let mut plan = ready_plan(&connection, key);
    let count = CHECKPOINT_PAGE_ENTRIES.saturating_mul(2).saturating_add(1);
    let (selected, metadata) = selected_files(count);
    plan.desired = selected.effective(&plan.current, metadata).unwrap();
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    let mut cursor = None;
    let mut paths = Vec::new();
    loop {
        let page = changes::page(
            &connection,
            plan.id,
            plan.owner,
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
            CheckpointOwner::Device {
                device: vessel_proto::DeviceId::random()
            },
            None,
            DateTime::UNIX_EPOCH
        ),
        Err(Error::Refused(CheckpointError::Forbidden))
    ));
    assert!(matches!(
        changes::page(
            &connection,
            plan.id,
            plan.owner,
            Some(ChangeCursor(PathCount::new(1).unwrap())),
            DateTime::UNIX_EPOCH
        ),
        Err(Error::Refused(CheckpointError::InvalidCursor))
    ));
    assert!(matches!(
        changes::page(&connection, plan.id, plan.owner, None, plan.expires),
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
        changes::page(&connection, plan.id, plan.owner, None, DateTime::UNIX_EPOCH),
        Err(Error::Refused(CheckpointError::StalePreview))
    ));
}

#[test]
fn preview_admission_enforces_lifetime_and_per_device_pin_limit() {
    let (connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    let mut next = plan.clone();
    for _ in 0..vessel_proto::checkpoint::CHECKPOINT_PREVIEWS_PER_OWNER {
        next.id = RestorePlanId::random();
        journal::put_plan(&connection, &next, DateTime::UNIX_EPOCH).unwrap();
    }
    next.id = RestorePlanId::random();
    assert!(matches!(
        journal::put_plan(&connection, &next, DateTime::UNIX_EPOCH),
        Err(Error::Refused(CheckpointError::Busy))
    ));
    next.owner = CheckpointOwner::Device {
        device: vessel_proto::DeviceId::random(),
    };
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

#[test]
fn delegated_origins_sharing_one_delivery_device_cannot_share_preview_or_restore_status() {
    use vessel_proto::{DeviceId, NodeId, checkpoint::CheckpointHumanOrigin};
    let (connection, key) = fixture();
    let mut plan = ready_plan(&connection, key);
    let delivering = DeviceId::random();
    let origin_node = NodeId::random();
    plan.owner = CheckpointOwner::DelegatedHuman {
        delivering,
        origin_node,
        origin: CheckpointHumanOrigin::Owner,
    };
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    let restore = RestoreId::random();
    let _operation = journal::admit(&connection, &plan, restore, DateTime::UNIX_EPOCH).unwrap();
    for wrong_owner in [
        CheckpointOwner::DelegatedHuman {
            delivering,
            origin_node: NodeId::random(),
            origin: CheckpointHumanOrigin::Owner,
        },
        CheckpointOwner::DelegatedHuman {
            delivering,
            origin_node,
            origin: CheckpointHumanOrigin::Device {
                device: DeviceId::random(),
            },
        },
        CheckpointOwner::Device { device: delivering },
        CheckpointOwner::LocalOwner,
    ] {
        assert!(matches!(
            super::plans::owned(&connection, plan.id, wrong_owner),
            Err(Error::Refused(CheckpointError::Forbidden))
        ));
        assert!(matches!(
            journal::status(&connection, key.session, restore, wrong_owner),
            Err(Error::Refused(CheckpointError::Forbidden))
        ));
        let mut wrong = plan.clone();
        wrong.owner = wrong_owner;
        assert!(matches!(
            journal::admit(&connection, &wrong, restore, DateTime::UNIX_EPOCH),
            Err(Error::Refused(CheckpointError::Forbidden))
        ));
    }
    assert_eq!(
        journal::status(&connection, key.session, restore, plan.owner)
            .unwrap()
            .owner,
        plan.owner
    );
}

#[test]
fn local_owner_keeps_its_authenticated_identity_across_restore_retries() {
    let (connection, key) = fixture();
    let mut plan = ready_plan(&connection, key);
    plan.owner = CheckpointOwner::LocalOwner;
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    let restore = RestoreId::random();
    let operation = journal::admit(&connection, &plan, restore, DateTime::UNIX_EPOCH).unwrap();
    journal::outcome(&connection, &operation, &RestoreOutcome::Unchanged).unwrap();
    assert_eq!(
        journal::admit(&connection, &plan, restore, plan.expires)
            .unwrap()
            .outcome,
        RestoreOutcome::Unchanged
    );
}

#[test]
fn retiring_a_session_keeps_its_archive_reservation_until_cleanup_acknowledgement() {
    use super::garbage;
    let (mut connection, key) = fixture();
    let _ready = ready_plan(&connection, key);
    let transaction = connection.transaction().unwrap();
    garbage::retire_session(&transaction, key.session, DateTime::UNIX_EPOCH).unwrap();
    let _deleted = transaction
        .execute(
            "DELETE FROM sessions WHERE id=?1",
            [key.session.to_string()],
        )
        .unwrap();
    transaction.commit().unwrap();
    let pending = garbage::pending(&connection).unwrap();
    assert_eq!(pending.len(), 1);
    let item = pending.first().unwrap();
    assert_eq!(
        item.object,
        garbage::Object::Capture {
            worktree: key.worktree,
            capture: key.capture
        }
    );
    assert_eq!(item.reserved, crate::ByteCount::new(1));
    let mut next = key;
    next.capture = CaptureId::random();
    next.session = SessionId::random();
    let _inserted = connection
        .execute(
            "INSERT INTO sessions(id) VALUES (?1)",
            [next.session.to_string()],
        )
        .unwrap();
    assert!(matches!(
        journal::candidate(
            &connection,
            next,
            crate::ByteCount::new(vessel_proto::checkpoint::CHECKPOINT_NODE_BYTES),
            DateTime::UNIX_EPOCH
        ),
        Err(Error::Refused(CheckpointError::StorageFull))
    ));
    garbage::cleaned(&connection, item.object).unwrap();
    garbage::cleaned(&connection, item.object).unwrap();
    assert!(garbage::pending(&connection).unwrap().is_empty());
    journal::candidate(
        &connection,
        next,
        crate::ByteCount::new(vessel_proto::checkpoint::CHECKPOINT_NODE_BYTES),
        DateTime::UNIX_EPOCH,
    )
    .unwrap();
}

#[test]
fn active_preview_and_recovery_data_prevent_session_retirement() {
    let (connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    assert!(matches!(
        super::garbage::retire_session(&connection, key.session, DateTime::UNIX_EPOCH),
        Err(Error::Refused(CheckpointError::Busy))
    ));
    let operation = journal::admit(
        &connection,
        &plan,
        RestoreId::random(),
        DateTime::UNIX_EPOCH,
    )
    .unwrap();
    assert!(matches!(
        super::garbage::retire_session(&connection, key.session, plan.expires),
        Err(Error::Refused(CheckpointError::RecoveryPending))
    ));
    assert!(super::garbage::pending(&connection).unwrap().is_empty());
    journal::outcome(&connection, &operation, &RestoreOutcome::Unchanged).unwrap();
    super::garbage::retire_session(&connection, key.session, plan.expires).unwrap();
    assert_eq!(super::garbage::pending(&connection).unwrap().len(), 1);
}

#[test]
fn equal_contents_cannot_hide_a_changed_root_or_ignore_observation() {
    use vessel_proto::{
        BlobHash,
        checkpoint::{IgnoreFingerprint, RootFingerprint, SnapshotEvidence},
    };
    let (connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    for evidence in [
        SnapshotEvidence::new(
            vessel_proto_physical(),
            RootFingerprint::new(BlobHash::from_bytes([3; 32])),
            plan.evidence.ignore(),
            Vec::new(),
        )
        .unwrap(),
        SnapshotEvidence::new(
            vessel_proto_physical(),
            plan.evidence.root(),
            IgnoreFingerprint::new(BlobHash::from_bytes([4; 32])),
            Vec::new(),
        )
        .unwrap(),
    ] {
        let mut changed = plan.clone();
        changed.evidence = evidence;
        assert!(matches!(
            journal::admit(
                &connection,
                &changed,
                RestoreId::random(),
                DateTime::UNIX_EPOCH
            ),
            Err(Error::Refused(CheckpointError::StalePreview))
        ));
    }
    assert!(matches!(
        journal::admit(
            &connection,
            &plan,
            RestoreId::random(),
            DateTime::UNIX_EPOCH
        )
        .unwrap()
        .outcome,
        RestoreOutcome::InProgress { .. }
    ));
}

fn vessel_proto_physical() -> vessel_proto::checkpoint::PhysicalRoot {
    vessel_proto::checkpoint::PhysicalRoot::new(vessel_proto::BlobHash::from_bytes([0; 32]))
}

#[test]
fn preview_refuses_changed_capture_policy_and_a_different_physical_root() {
    use vessel_proto::{
        BlobHash,
        checkpoint::{IgnoreFingerprint, PhysicalRoot, SnapshotEvidence},
    };
    let (connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    for evidence in [
        SnapshotEvidence::new(
            PhysicalRoot::new(BlobHash::from_bytes([7; 32])),
            plan.evidence.root(),
            plan.evidence.ignore(),
            Vec::new(),
        )
        .unwrap(),
        SnapshotEvidence::new(
            plan.evidence.physical(),
            plan.evidence.root(),
            IgnoreFingerprint::new(BlobHash::from_bytes([8; 32])),
            Vec::new(),
        )
        .unwrap(),
    ] {
        let mut changed = plan.clone();
        changed.evidence = evidence;
        assert!(matches!(
            journal::put_plan(&connection, &changed, DateTime::UNIX_EPOCH),
            Err(Error::Refused(CheckpointError::StalePreview))
        ));
    }
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
}

#[test]
fn checkpoint_recovery_resolves_only_a_verified_whole_before_image_and_never_success() {
    use vessel_proto::{
        BlobHash,
        checkpoint::{IgnoreFingerprint, ObservedSnapshot, SnapshotEvidence},
    };
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
    super::paths::prepare(&connection, &operation).unwrap();
    let _count = super::recovery::pending(&connection).unwrap();
    let wrong = ObservedSnapshot::new(
        plan.current.clone(),
        SnapshotEvidence::new(
            plan.evidence.physical(),
            plan.evidence.root(),
            IgnoreFingerprint::new(BlobHash::from_bytes([9; 32])),
            Vec::new(),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        super::recovery::resolved(
            &connection,
            &operation,
            &wrong,
            CheckpointError::CaptureUnstable
        ),
        Err(Error::Refused(CheckpointError::RecoveryPending))
    ));
    let observed = ObservedSnapshot::new(plan.current.clone(), plan.evidence.clone()).unwrap();
    super::recovery::resolved(
        &connection,
        &operation,
        &observed,
        CheckpointError::CaptureUnstable,
    )
    .unwrap();
    // A lost final acknowledgement can retry the same recovery decision.
    super::recovery::resolved(
        &connection,
        &operation,
        &observed,
        CheckpointError::CaptureUnstable,
    )
    .unwrap();
    let result = journal::status(&connection, key.session, operation.id, plan.owner)
        .unwrap()
        .outcome;
    assert_eq!(
        result,
        RestoreOutcome::RolledBack {
            reason: CheckpointError::CaptureUnstable
        }
    );
    assert!(matches!(
        super::recovery::resolved(
            &connection,
            &operation,
            &observed,
            CheckpointError::StorageFull
        ),
        Err(Error::Refused(CheckpointError::RestoreIdConflict))
    ));
}

#[test]
fn checkpoint_recovery_refuses_equal_contents_on_a_replacement_inode() {
    use vessel_proto::{
        BlobHash,
        checkpoint::{
            FileFingerprint, ObjectFingerprint, ObservedEntry, ObservedPresence, ObservedSnapshot,
            SnapshotEvidence,
        },
    };
    let (connection, key) = fixture();
    let (image, metadata) = selected_files(1);
    let original = super::tests::observed(image.clone());
    journal::keep_candidate(&connection, key, &original).unwrap();
    super::publication::activate(
        &connection,
        key,
        vessel_proto::checkpoint::CheckpointId::random(),
    )
    .unwrap();
    let plan = Plan {
        id: RestorePlanId::random(),
        key,
        owner: CheckpointOwner::LocalOwner,
        current: image.clone(),
        evidence: original.evidence().clone(),
        desired: image.effective(&image, metadata).unwrap(),
        expires: DateTime::UNIX_EPOCH
            .checked_add_signed(
                chrono::TimeDelta::from_std(vessel_proto::checkpoint::CHECKPOINT_PREVIEW_LIFETIME)
                    .unwrap(),
            )
            .unwrap(),
    };
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    let operation = journal::admit(
        &connection,
        &plan,
        RestoreId::random(),
        DateTime::UNIX_EPOCH,
    )
    .unwrap();
    super::paths::prepare(&connection, &operation).unwrap();
    let _count = super::recovery::pending(&connection).unwrap();
    let path = plan.evidence.entries().first().unwrap().path.clone();
    let evidence = SnapshotEvidence::new(
        plan.evidence.physical(),
        plan.evidence.root(),
        plan.evidence.ignore(),
        vec![ObservedEntry {
            path,
            presence: ObservedPresence::Present {
                full: FileFingerprint::new(BlobHash::from_bytes([9; 32])),
                object: ObjectFingerprint::new(BlobHash::from_bytes([9; 32])),
            },
        }],
    )
    .unwrap();
    let replacement = ObservedSnapshot::new(image, evidence).unwrap();
    assert!(matches!(
        super::recovery::resolved(
            &connection,
            &operation,
            &replacement,
            CheckpointError::CaptureUnstable
        ),
        Err(Error::Refused(CheckpointError::RecoveryPending))
    ));
    assert!(matches!(
        journal::status(&connection, key.session, operation.id, plan.owner)
            .unwrap()
            .outcome,
        RestoreOutcome::RecoveryRequired { .. }
    ));
    super::recovery::resolved(
        &connection,
        &operation,
        &original,
        CheckpointError::CaptureUnstable,
    )
    .unwrap();
}

#[test]
fn checkpoint_private_reservation_pins_retirement_then_follows_garbage_until_cleanup() {
    use super::{
        allocation::{self, Allocation},
        garbage,
    };
    let (connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    let allocation = Allocation::Preview { plan: plan.id };
    allocation::reserve(
        &connection,
        key,
        allocation,
        plan.owner,
        DateTime::UNIX_EPOCH,
    )
    .unwrap();
    let later = plan
        .expires
        .checked_add_signed(chrono::TimeDelta::seconds(1))
        .unwrap();
    assert!(matches!(
        garbage::retire_session(&connection, key.session, later),
        Err(Error::Refused(CheckpointError::Busy))
    ));
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    allocation::attach(&connection, key, allocation, plan.owner).unwrap();
    let charged: i64 = connection
        .query_row(
            "SELECT sum(reserved_bytes) FROM checkpoint_allocations",
            [],
            |row| row.get(0),
        )
        .unwrap();
    garbage::retire_session(&connection, key.session, later).unwrap();
    let pending = garbage::pending(&connection).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(
        pending[0].reserved.get(),
        u64::try_from(charged).unwrap().checked_add(1).unwrap()
    );
    assert!(matches!(
        super::quota::available(
            &connection,
            crate::ByteCount::new(vessel_proto::checkpoint::CHECKPOINT_NODE_BYTES)
        ),
        Err(Error::Refused(CheckpointError::StorageFull))
    ));
    garbage::cleaned(&connection, pending[0].object).unwrap();
    super::quota::available(
        &connection,
        crate::ByteCount::new(vessel_proto::checkpoint::CHECKPOINT_NODE_BYTES),
    )
    .unwrap();
}

#[test]
fn checkpoint_private_reservation_cannot_cross_delegated_origins_on_one_delivering_device() {
    use super::allocation::{self, Allocation};
    use vessel_proto::{DeviceId, NodeId, checkpoint::CheckpointHumanOrigin};
    let (connection, key) = fixture();
    let plan = ready_plan(&connection, key);
    let delivering = DeviceId::random();
    let origin = CheckpointHumanOrigin::Owner;
    let owner = CheckpointOwner::DelegatedHuman {
        delivering,
        origin_node: NodeId::random(),
        origin,
    };
    let other = CheckpointOwner::DelegatedHuman {
        delivering,
        origin_node: NodeId::random(),
        origin,
    };
    let allocation = Allocation::Preview { plan: plan.id };
    allocation::reserve(&connection, key, allocation, owner, DateTime::UNIX_EPOCH).unwrap();
    assert!(matches!(
        allocation::cleaned(&connection, key, allocation, other),
        Err(Error::Refused(CheckpointError::Forbidden))
    ));
    assert!(matches!(
        allocation::attach(&connection, key, allocation, other),
        Err(Error::Refused(CheckpointError::Forbidden))
    ));
    allocation::cleaned(&connection, key, allocation, owner).unwrap();
}
