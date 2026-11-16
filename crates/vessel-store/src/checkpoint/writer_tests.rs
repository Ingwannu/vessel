//! The real shared writer commits path intent before replying, and isolates failure.
use super::{
    CheckpointWrite, journal,
    paths::{self, Phase},
    records::{CheckpointStoreError, Plan},
};
use crate::{ByteCount, DataDir, Store, StoreConfig, StoreError};
use chrono::DateTime;
use vessel_proto::{
    BlobHash, EventSeq, HarnessKind, SessionId, SessionInfo, SessionMode, SessionSpec, WorktreeId,
    checkpoint::{
        Boundary, CHECKPOINT_PREVIEW_LIFETIME, CaptureGeneration, CaptureId, CaptureKey,
        CheckpointError, CheckpointFileBytes, CheckpointId, CheckpointOwner, Entry, FileImage,
        FileMetadata, FileState, GroupId, OrdinaryMode, OwnerId, PathCount, RestoreId,
        RestoreOutcome, RestorePlanId,
    },
};

pub(super) fn info() -> SessionInfo {
    SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        DateTime::UNIX_EPOCH,
    )
}
pub(super) fn plan(session: SessionId) -> Plan {
    let key = CaptureKey {
        capture: CaptureId::random(),
        session,
        worktree: WorktreeId::random(),
        boundary: Boundary::BeforeTurn {
            prompt: EventSeq::new(0),
        },
        generation: CaptureGeneration::FIRST,
    };
    let current = FileImage::new(Vec::new()).unwrap();
    let metadata = FileMetadata {
        owner: OwnerId::new(1000),
        group: GroupId::new(1000),
        mode: OrdinaryMode::new(0o600).unwrap(),
    };
    let selected = FileImage::new(vec![Entry {
        path: serde_json::from_str("\"file\"").unwrap(),
        state: FileState::Present {
            digest: BlobHash::from_bytes([0; 32]),
            bytes: CheckpointFileBytes::new(0).unwrap(),
            metadata,
        },
    }])
    .unwrap();
    let desired = selected.effective(&current, metadata).unwrap();
    Plan {
        id: RestorePlanId::random(),
        key,
        owner: CheckpointOwner::LocalOwner,
        evidence: super::tests::evidence(),
        current,
        desired,
        expires: DateTime::UNIX_EPOCH
            .checked_add_signed(chrono::TimeDelta::from_std(CHECKPOINT_PREVIEW_LIFETIME).unwrap())
            .unwrap(),
    }
}

pub(super) async fn read_operation(
    store: &Store,
    plan: &Plan,
    restore: RestoreId,
) -> super::records::Operation {
    let session = plan.key.session;
    let owner = plan.owner;
    store
        .read(move |reader| {
            reader.with(|connection| {
                journal::status(connection, session, restore, owner).map_err(StoreError::from)
            })
        })
        .await
        .unwrap()
}

pub(super) async fn opened() -> (tempfile::TempDir, DataDir, Store) {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    (temp, dir, store)
}

pub(super) async fn allocate(
    store: &Store,
    plan: &Plan,
    allocation: super::Allocation,
) -> Result<(), StoreError> {
    store
        .checkpoint_write(CheckpointWrite::Allocate {
            key: plan.key,
            allocation,
            owner: plan.owner,
            at: DateTime::UNIX_EPOCH,
        })
        .await
}

pub(super) async fn publish_plan(store: &Store, plan: &Plan) {
    allocate(store, plan, super::Allocation::Preview { plan: plan.id })
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::Plan {
            plan: Box::new(plan.clone()),
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
}

pub(super) async fn activate_candidate(store: &Store, plan: &Plan) {
    store
        .checkpoint_write(CheckpointWrite::Candidate {
            key: plan.key,
            snapshot: Box::new(super::tests::observed(plan.desired.image().clone())),
        })
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::Activate {
            key: plan.key,
            checkpoint: CheckpointId::random(),
        })
        .await
        .unwrap();
}

pub(super) async fn publish_at(store: &Store, plan: &Plan, at: chrono::DateTime<chrono::Utc>) {
    store
        .checkpoint_write(CheckpointWrite::Reserve {
            key: plan.key,
            bytes: ByteCount::new(1),
            at,
        })
        .await
        .unwrap();
    activate_candidate(store, plan).await;
}

pub(super) async fn publish_capture(store: &Store, plan: &Plan) {
    publish_at(store, plan, DateTime::UNIX_EPOCH).await;
}

async fn reserve_capture(store: &Store, key: CaptureKey) {
    store
        .checkpoint_write(CheckpointWrite::ReserveCapture {
            key,
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
}

async fn clean_allocation(
    store: &Store,
    plan: &Plan,
    allocation: super::Allocation,
    owner: CheckpointOwner,
) -> Result<(), StoreError> {
    store
        .checkpoint_write(CheckpointWrite::AllocationCleaned {
            key: plan.key,
            allocation,
            owner,
        })
        .await
}

async fn settle_capture(store: &Store, key: CaptureKey) -> Result<(), StoreError> {
    store
        .checkpoint_write(CheckpointWrite::CaptureSettled { key })
        .await
}

#[tokio::test]
async fn checkpoint_staged_objects_commit_all_paths_or_none_and_survive_a_retry() {
    let (_temp, dir, store) = opened().await;
    let info = info();
    store.create_session(info.clone()).await.unwrap();
    let mut plan = plan(info.id);
    let mut selected = plan.desired.image().entries().to_vec();
    let mut other = selected[0].clone();
    other.path = serde_json::from_str("\"other\"").unwrap();
    selected.push(other);
    let FileState::Present { metadata, .. } = selected[0].state else {
        panic!("present fixture")
    };
    plan.desired = FileImage::new(selected)
        .unwrap()
        .effective(&plan.current, metadata)
        .unwrap();
    publish_capture(&store, &plan).await;
    publish_plan(&store, &plan).await;
    let restore = RestoreId::random();
    store
        .checkpoint_write(CheckpointWrite::Admit {
            plan: Box::new(plan.clone()),
            restore,
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
    let operation = read_operation(&store, &plan, restore).await;
    let path = dir.database_path();
    tokio::task::spawn_blocking(move || {
        rusqlite::Connection::open(path).unwrap().execute_batch("CREATE TRIGGER deny_second_stage BEFORE UPDATE OF staged ON checkpoint_paths WHEN NEW.path='other' BEGIN SELECT RAISE(ABORT,'fixture staged path failure'); END;").unwrap();
    }).await.unwrap();
    let staged = super::tests::staged(&plan);
    assert!(
        store
            .checkpoint_write(CheckpointWrite::Staged {
                operation: Box::new(operation.clone()),
                paths: staged.clone()
            })
            .await
            .is_err()
    );
    assert!(
        store
            .checkpoint_paths(operation.clone())
            .await
            .unwrap()
            .iter()
            .all(|entry| entry.staged.is_none())
    );
    let path = dir.database_path();
    tokio::task::spawn_blocking(move || {
        rusqlite::Connection::open(path)
            .unwrap()
            .execute_batch("DROP TRIGGER deny_second_stage")
            .unwrap();
    })
    .await
    .unwrap();
    for _ in 0..2 {
        store
            .checkpoint_write(CheckpointWrite::Staged {
                operation: Box::new(operation.clone()),
                paths: staged.clone(),
            })
            .await
            .unwrap();
    }
    let entries = store.checkpoint_paths(operation.clone()).await.unwrap();
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.staged.clone().unwrap())
            .collect::<Vec<_>>(),
        staged.entries()
    );
    let mut changed = staged.entries().to_vec();
    changed[0].desired = Some(vessel_proto::checkpoint::ObjectFingerprint::new(
        BlobHash::from_bytes([99; 32]),
    ));
    assert!(
        store
            .checkpoint_write(CheckpointWrite::Staged {
                operation: Box::new(operation.clone()),
                paths: vessel_proto::checkpoint::StagedPaths::new(changed).unwrap()
            })
            .await
            .is_err()
    );
    store.close().await;
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    assert_eq!(store.checkpoint_paths(operation).await.unwrap(), entries);
    store.close().await;
}

#[tokio::test]
async fn shared_writer_admits_all_path_intents_before_reply_and_keeps_idempotent_result() {
    let (_temp, dir, store) = opened().await;
    let info = info();
    store.create_session(info.clone()).await.unwrap();
    let plan = plan(info.id);
    publish_capture(&store, &plan).await;
    publish_plan(&store, &plan).await;
    let restore = RestoreId::random();
    for _ in 0..2 {
        store
            .checkpoint_write(CheckpointWrite::Admit {
                plan: Box::new(plan.clone()),
                restore,
                at: DateTime::UNIX_EPOCH,
            })
            .await
            .unwrap();
    }
    let operation = read_operation(&store, &plan, restore).await;
    let read = operation.clone();
    let entries = store
        .read(move |reader| {
            reader.with(|connection| paths::list(connection, &read).map_err(StoreError::from))
        })
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);
    let path = entries.first().unwrap().path.clone();
    assert_eq!(entries.first().unwrap().phase, Phase::Pending);
    store
        .checkpoint_write(CheckpointWrite::Staged {
            operation: Box::new(operation.clone()),
            paths: super::tests::staged(&plan),
        })
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::Advance {
            operation: Box::new(operation.clone()),
            path,
            from: Phase::Pending,
            to: Phase::Applied,
        })
        .await
        .unwrap();
    let result = RestoreOutcome::Restored {
        changed: PathCount::new(1).unwrap(),
    };
    store
        .checkpoint_write(CheckpointWrite::Outcome {
            operation: Box::new(operation),
            result: result.clone(),
        })
        .await
        .unwrap();
    assert_eq!(read_operation(&store, &plan, restore).await.outcome, result);
    let mut next = plan.key;
    next.capture = CaptureId::random();
    store
        .checkpoint_write(CheckpointWrite::Reserve {
            key: next,
            bytes: ByteCount::new(1),
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::Unavailable {
            key: next,
            reason: CheckpointError::CaptureTimedOut,
        })
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::InvalidateCandidates)
        .await
        .unwrap();
    store.close().await;
    let reopened = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    assert_eq!(
        read_operation(&reopened, &plan, restore).await.outcome,
        result
    );
    reopened.close().await;
}

#[tokio::test]
async fn failed_pending_path_insert_rolls_back_restore_admission_in_the_shared_writer() {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let mut connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    crate::schema::configure_writer(&connection).unwrap();
    crate::schema::migrate(&mut connection).unwrap();
    let info = info();
    crate::log::create_session(&connection, &info).unwrap();
    let plan = plan(info.id);
    journal::candidate(
        &connection,
        plan.key,
        ByteCount::new(1),
        DateTime::UNIX_EPOCH,
    )
    .unwrap();
    journal::keep_candidate(
        &connection,
        plan.key,
        &super::tests::observed(plan.desired.image().clone()),
    )
    .unwrap();
    super::publication::activate(&connection, plan.key, CheckpointId::random()).unwrap();
    journal::put_plan(&connection, &plan, DateTime::UNIX_EPOCH).unwrap();
    connection.execute_batch("CREATE TRIGGER checkpoint_deny_path BEFORE INSERT ON checkpoint_paths BEGIN SELECT RAISE(ABORT,'fixture fail'); END;").unwrap();
    drop(connection);
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let restore = RestoreId::random();
    let error = store
        .checkpoint_write(CheckpointWrite::Admit {
            plan: Box::new(plan.clone()),
            restore,
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        StoreError::Checkpoint(CheckpointStoreError::Sql(_))
    ));
    let checked = plan.clone();
    store
        .read(move |reader| {
            reader.with(|connection| {
                assert!(matches!(
                    journal::status(connection, checked.key.session, restore, checked.owner),
                    Err(CheckpointStoreError::Refused(CheckpointError::NotFound))
                ));
                assert_eq!(
                    super::plans::owned(connection, checked.id, checked.owner)
                        .unwrap()
                        .1,
                    None
                );
                Ok(())
            })
        })
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::InvalidateCandidates)
        .await
        .unwrap();
    store.close().await;
}

#[tokio::test]
async fn session_delete_transfers_archive_quota_only_after_the_same_savepoint_commits() {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let info = info();
    store.create_session(info.clone()).await.unwrap();
    let plan = plan(info.id);
    publish_capture(&store, &plan).await;
    assert!(matches!(
        store.delete_session(info.id).await,
        Err(StoreError::SessionLive(_))
    ));
    let empty = store
        .read(|reader| {
            reader.with(|connection| super::garbage::pending(connection).map_err(StoreError::from))
        })
        .await
        .unwrap();
    assert!(
        empty.is_empty(),
        "a failed live-session delete must roll back quota transfer"
    );
    store
        .append(
            info.id,
            vec![std::sync::Arc::new(vessel_proto::EventRecord {
                seq: EventSeq::new(0),
                at: DateTime::UNIX_EPOCH,
                event: vessel_proto::SessionEvent::Status {
                    status: vessel_proto::SessionStatus::Exited { code: Some(0) },
                },
            })],
        )
        .await
        .unwrap();
    store.delete_session(info.id).await.unwrap();
    let pending = store
        .read(|reader| {
            reader.with(|connection| super::garbage::pending(connection).map_err(StoreError::from))
        })
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
    let item = pending.first().unwrap();
    assert_eq!(item.reserved, ByteCount::new(1));
    store
        .checkpoint_write(CheckpointWrite::GarbageCleaned {
            object: item.object,
        })
        .await
        .unwrap();
    let empty = store
        .read(|reader| {
            reader.with(|connection| super::garbage::pending(connection).map_err(StoreError::from))
        })
        .await
        .unwrap();
    assert!(empty.is_empty());
    store.close().await;
}

#[tokio::test]
async fn checkpoint_shared_writer_commits_verified_recovery_and_idempotent_status() {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let info = info();
    store.create_session(info.clone()).await.unwrap();
    let plan = plan(info.id);
    publish_capture(&store, &plan).await;
    store
        .checkpoint_write(CheckpointWrite::Allocate {
            key: plan.key,
            allocation: super::Allocation::Preview { plan: plan.id },
            owner: plan.owner,
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::Plan {
            plan: Box::new(plan.clone()),
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
    let restore = RestoreId::random();
    store
        .checkpoint_write(CheckpointWrite::Admit {
            plan: Box::new(plan.clone()),
            restore,
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
    let operation = read_operation(&store, &plan, restore).await;
    store
        .checkpoint_write(CheckpointWrite::Outcome {
            operation: Box::new(operation.clone()),
            result: RestoreOutcome::RecoveryRequired { restore },
        })
        .await
        .unwrap();
    let observed = vessel_proto::checkpoint::ObservedSnapshot::new(
        plan.current.clone(),
        plan.evidence.clone(),
    )
    .unwrap();
    for _ in 0..2 {
        store
            .checkpoint_write(CheckpointWrite::Recovered {
                operation: Box::new(operation.clone()),
                observed: observed.clone(),
                reason: CheckpointError::CaptureUnstable,
            })
            .await
            .unwrap();
    }
    assert_eq!(
        read_operation(&store, &plan, restore).await.outcome,
        RestoreOutcome::RolledBack {
            reason: CheckpointError::CaptureUnstable
        }
    );
    store.close().await;
    let reopened = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    assert_eq!(
        read_operation(&reopened, &plan, restore).await.outcome,
        RestoreOutcome::RolledBack {
            reason: CheckpointError::CaptureUnstable
        }
    );
    reopened.close().await;
}

#[tokio::test]
async fn checkpoint_private_allocations_use_real_writer_quota_and_never_recycle_cleaned_ids() {
    use super::allocation::Allocation;
    let (_temp, _dir, store) = opened().await;
    let info = info();
    store.create_session(info.clone()).await.unwrap();
    let plan = plan(info.id);
    publish_capture(&store, &plan).await;
    let preview = Allocation::Preview { plan: plan.id };
    publish_plan(&store, &plan).await;
    store
        .checkpoint_write(CheckpointWrite::AllocationAttached {
            key: plan.key,
            allocation: preview,
            owner: plan.owner,
        })
        .await
        .unwrap();
    assert!(matches!(
        clean_allocation(&store, &plan, preview, plan.owner).await,
        Err(StoreError::Checkpoint(CheckpointStoreError::Refused(
            CheckpointError::RecoveryPending
        )))
    ));
    let abandoned = Allocation::Restore {
        restore: RestoreId::random(),
    };
    allocate(&store, &plan, abandoned).await.unwrap();
    assert!(matches!(
        clean_allocation(
            &store,
            &plan,
            abandoned,
            CheckpointOwner::Device {
                device: vessel_proto::DeviceId::random(),
            }
        )
        .await,
        Err(StoreError::Checkpoint(CheckpointStoreError::Refused(
            CheckpointError::Forbidden
        )))
    ));
    for _retry in 0..2 {
        clean_allocation(&store, &plan, abandoned, plan.owner)
            .await
            .unwrap();
    }
    assert!(matches!(
        allocate(&store, &plan, abandoned).await,
        Err(StoreError::Checkpoint(CheckpointStoreError::Refused(
            CheckpointError::RestoreIdConflict
        )))
    ));
    let reserved = store
        .read(|reader| {
            reader.with(|connection| {
                let bytes: i64 = connection.query_row(
                    "SELECT sum(reserved_bytes) FROM checkpoint_allocations",
                    [],
                    |row| row.get(0),
                )?;
                Ok(bytes)
            })
        })
        .await
        .unwrap();
    assert!(
        reserved > 0,
        "cleaned-id metadata and the live preview remain accounted"
    );
    let mut extra = plan.key;
    extra.capture = CaptureId::random();
    let reserved = u64::try_from(reserved).unwrap().checked_add(1).unwrap();
    store
        .checkpoint_write(CheckpointWrite::Reserve {
            key: extra,
            bytes: ByteCount::new(
                vessel_proto::checkpoint::CHECKPOINT_NODE_BYTES
                    .checked_sub(reserved)
                    .unwrap(),
            ),
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
    assert!(matches!(
        allocate(
            &store,
            &plan,
            Allocation::Preview {
                plan: RestorePlanId::random()
            }
        )
        .await,
        Err(StoreError::Checkpoint(CheckpointStoreError::Refused(
            CheckpointError::StorageFull
        )))
    ));
    store.close().await;
}

#[tokio::test]
async fn checkpoint_capture_escrow_releases_only_after_terminal_cleanup_and_keeps_ready_bytes() {
    let (_temp, _dir, store) = opened().await;
    let info = info();
    store.create_session(info.clone()).await.unwrap();
    let plan = plan(info.id);
    store
        .append(
            info.id,
            vec![std::sync::Arc::new(vessel_proto::EventRecord {
                seq: EventSeq::ZERO,
                at: DateTime::UNIX_EPOCH,
                event: vessel_proto::SessionEvent::Checkpoint {
                    event: vessel_proto::checkpoint::CheckpointEvent::PromptAccepted,
                },
            })],
        )
        .await
        .unwrap();
    reserve_capture(&store, plan.key).await;
    assert!(matches!(
        store
            .checkpoint_write(CheckpointWrite::CaptureSettled { key: plan.key })
            .await,
        Err(StoreError::Checkpoint(CheckpointStoreError::Refused(
            CheckpointError::Busy
        )))
    ));
    activate_candidate(&store, &plan).await;
    settle_capture(&store, plan.key).await.unwrap();
    let capture = plan.key.capture.to_string();
    let charged: i64 = store
        .read(move |reader| {
            reader.with(|connection| {
                Ok(connection.query_row(
                    "SELECT reserved_bytes FROM checkpoint_captures WHERE id=?1",
                    [capture],
                    |row| row.get(0),
                )?)
            })
        })
        .await
        .unwrap();
    assert!(charged > 0);
    assert!(u64::try_from(charged).unwrap() < super::quota::CAPTURE_BYTES.get());
    // Duplicate cleanup cannot release the retained payload/metadata a second time.
    settle_capture(&store, plan.key).await.unwrap();
    let mut unavailable = plan.key;
    unavailable.capture = CaptureId::random();
    reserve_capture(&store, unavailable).await;
    store
        .checkpoint_write(CheckpointWrite::Unavailable {
            key: unavailable,
            reason: CheckpointError::CaptureTimedOut,
        })
        .await
        .unwrap();
    store
        .append(
            info.id,
            vec![std::sync::Arc::new(vessel_proto::EventRecord {
                seq: EventSeq::new(1),
                at: DateTime::UNIX_EPOCH,
                event: vessel_proto::SessionEvent::Status {
                    status: vessel_proto::SessionStatus::Exited { code: Some(0) },
                },
            })],
        )
        .await
        .unwrap();
    assert!(matches!(
        store.delete_session(info.id).await,
        Err(StoreError::Checkpoint(CheckpointStoreError::Refused(
            CheckpointError::Busy
        )))
    ));
    assert!(store.checkpoint_garbage().await.unwrap().is_empty());
    // This acknowledgement models actual cleanup returning, not actor timeout.
    settle_capture(&store, unavailable).await.unwrap();
    store.delete_session(info.id).await.unwrap();
    let garbage = store.checkpoint_garbage().await.unwrap();
    assert_eq!(garbage.len(), 2);
    let unavailable_object = super::GarbageObject::Capture {
        worktree: unavailable.worktree,
        capture: unavailable.capture,
    };
    let retained = garbage
        .iter()
        .find(|item| item.object == unavailable_object)
        .unwrap();
    assert!(
        retained.reserved.get() > 0,
        "terminal metadata remains charged"
    );
    assert!(retained.reserved.get() < super::quota::CAPTURE_BYTES.get());
    store.close().await;
}

#[tokio::test]
async fn checkpoint_refused_boundary_is_durable_metadata_without_a_ready_image_or_reused_charge() {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let info = info();
    store.create_session(info.clone()).await.unwrap();
    let plan = plan(info.id);
    let refused = || CheckpointWrite::RefusedBoundary {
        key: plan.key,
        reason: CheckpointError::Busy,
        at: DateTime::UNIX_EPOCH,
    };
    assert!(
        store.checkpoint_write(refused()).await.is_err(),
        "a missing durable anchor never gets synthetic metadata"
    );
    store
        .append(
            info.id,
            vec![std::sync::Arc::new(vessel_proto::EventRecord {
                seq: EventSeq::ZERO,
                at: DateTime::UNIX_EPOCH,
                event: vessel_proto::SessionEvent::Checkpoint {
                    event: vessel_proto::checkpoint::CheckpointEvent::PromptAccepted,
                },
            })],
        )
        .await
        .unwrap();
    for _repeat in 0..2 {
        store.checkpoint_write(refused()).await.unwrap();
    }
    let page = store.checkpoint_page(info.id, None).await.unwrap();
    assert_eq!(page.marks.len(), 1);
    assert_eq!(
        page.marks[0].state,
        vessel_proto::checkpoint::CheckpointState::Unavailable {
            reason: CheckpointError::Busy
        }
    );
    let charged = store
        .read(|reader| {
            reader.with(|connection| super::quota::used(connection).map_err(StoreError::from))
        })
        .await
        .unwrap();
    assert_eq!(charged, ByteCount::new(super::quota::METADATA_BYTES));
    assert!(
        store
            .checkpoint_write(CheckpointWrite::Activate {
                key: plan.key,
                checkpoint: CheckpointId::random()
            })
            .await
            .is_err()
    );
    assert!(
        store
            .checkpoint_write(CheckpointWrite::Candidate {
                key: plan.key,
                snapshot: Box::new(super::tests::observed(plan.desired.image().clone()))
            })
            .await
            .is_err()
    );
    assert!(
        !dir.root().join("checkpoints").exists(),
        "metadata admission performs no checkpoint filesystem I/O"
    );
    store.close().await;
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    assert_eq!(store.checkpoint_page(info.id, None).await.unwrap(), page);
    store.close().await;
}
