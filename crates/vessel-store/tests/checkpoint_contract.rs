//! Public non-cfg(test) storage contract, with synthetic evidence and no file restore.
use chrono::DateTime;
use std::sync::Arc;
use vessel_proto::{
    BlobHash, EventRecord, EventSeq, HarnessKind, SessionEvent, SessionId, SessionInfo,
    SessionMode, SessionSpec, SessionStatus, WorktreeId,
    checkpoint::{
        Boundary, CaptureGeneration, CaptureId, CaptureKey, CheckpointEvent, CheckpointFileBytes,
        CheckpointId, CheckpointOwner, Entry, FileFingerprint, FileImage, FileMetadata, FileState,
        GroupId, IgnoreFingerprint, ObjectFingerprint, ObservedEntry, ObservedPresence,
        ObservedSnapshot, OrdinaryMode, OwnerId, PathCount, PhysicalRoot, RestoreId,
        RestoreOutcome, RestorePlanId, RootFingerprint, SnapshotEvidence,
    },
};
use vessel_store::{
    DataDir, Store, StoreConfig,
    checkpoint::{Allocation, CheckpointWrite, PathPhase, Plan},
};

fn evidence(entries: Vec<ObservedEntry>) -> SnapshotEvidence {
    SnapshotEvidence::new(
        PhysicalRoot::new(BlobHash::from_bytes([1; 32])),
        RootFingerprint::new(BlobHash::from_bytes([2; 32])),
        IgnoreFingerprint::new(BlobHash::from_bytes([3; 32])),
        entries,
    )
    .unwrap()
}

#[tokio::test]
async fn checkpoint_public_writer_requires_a_durable_boundary_and_preserves_ended_history() {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let info = SessionInfo::new(
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
    );
    store.create_session(info.clone()).await.unwrap();
    let key = CaptureKey {
        capture: CaptureId::random(),
        session: info.id,
        worktree: WorktreeId::random(),
        generation: CaptureGeneration::FIRST,
        boundary: Boundary::BeforeTurn {
            prompt: EventSeq::ZERO,
        },
    };
    assert!(
        store
            .checkpoint_write(CheckpointWrite::ReserveCapture {
                key,
                at: DateTime::UNIX_EPOCH
            })
            .await
            .is_err()
    );
    store
        .append(
            info.id,
            vec![Arc::new(EventRecord {
                seq: EventSeq::ZERO,
                at: DateTime::UNIX_EPOCH,
                event: SessionEvent::Checkpoint {
                    event: CheckpointEvent::PromptAccepted,
                },
            })],
        )
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::ReserveCapture {
            key,
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
    let metadata = FileMetadata {
        owner: OwnerId::new(1000),
        group: GroupId::new(1000),
        mode: OrdinaryMode::new(0o600).unwrap(),
    };
    let path: vessel_proto::checkpoint::CheckpointPath = serde_json::from_str("\"file\"").unwrap();
    let image = FileImage::new(vec![Entry {
        path: path.clone(),
        state: FileState::Present {
            digest: BlobHash::from_bytes([4; 32]),
            bytes: CheckpointFileBytes::new(0).unwrap(),
            metadata,
        },
    }])
    .unwrap();
    let snapshot = ObservedSnapshot::new(
        image.clone(),
        evidence(vec![ObservedEntry {
            path: path.clone(),
            presence: ObservedPresence::Present {
                full: FileFingerprint::new(BlobHash::from_bytes([5; 32])),
                object: ObjectFingerprint::new(BlobHash::from_bytes([6; 32])),
            },
        }]),
    )
    .unwrap();
    store
        .checkpoint_write(CheckpointWrite::Candidate {
            key,
            snapshot: Box::new(snapshot.clone()),
        })
        .await
        .unwrap();
    let checkpoint = CheckpointId::random();
    store
        .checkpoint_write(CheckpointWrite::Activate { key, checkpoint })
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::CaptureSettled { key })
        .await
        .unwrap();
    let selected = store
        .checkpoint_selected(info.id, checkpoint)
        .await
        .unwrap();
    assert_eq!(selected.key, key);
    assert_eq!(selected.snapshot, snapshot);
    assert!(
        store
            .checkpoint_selected(SessionId::random(), checkpoint)
            .await
            .is_err()
    );
    assert_eq!(
        store
            .checkpoint_page(info.id, None)
            .await
            .unwrap()
            .marks
            .len(),
        1
    );
    let current = FileImage::new(Vec::new()).unwrap();
    let plan = Plan {
        id: RestorePlanId::random(),
        key,
        owner: CheckpointOwner::LocalOwner,
        evidence: evidence(Vec::new()),
        desired: image.effective(&current, metadata).unwrap(),
        current,
        expires: DateTime::from_timestamp(300, 0).unwrap(),
    };
    assert!(
        store
            .checkpoint_write(CheckpointWrite::Plan {
                plan: Box::new(plan.clone()),
                at: DateTime::UNIX_EPOCH
            })
            .await
            .is_err()
    );
    store
        .checkpoint_write(CheckpointWrite::Allocate {
            key,
            allocation: Allocation::Preview { plan: plan.id },
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
    assert_eq!(
        store.checkpoint_plan(plan.id, plan.owner).await.unwrap(),
        (plan.clone(), None)
    );
    assert_eq!(
        store
            .checkpoint_changes(plan.id, plan.owner, None, DateTime::UNIX_EPOCH)
            .await
            .unwrap()
            .changes
            .len(),
        1
    );
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
    let operation = store
        .checkpoint_operation(info.id, restore, plan.owner)
        .await
        .unwrap();
    assert_eq!(
        store
            .checkpoint_paths(operation.clone())
            .await
            .unwrap()
            .len(),
        1
    );
    store
        .checkpoint_write(CheckpointWrite::Staged {
            operation: Box::new(operation.clone()),
            paths: vessel_proto::checkpoint::StagedPaths::new(vec![
                vessel_proto::checkpoint::StagedPath {
                    path: path.clone(),
                    before: None,
                    desired: Some(ObjectFingerprint::new(BlobHash::from_bytes([8; 32]))),
                },
            ])
            .unwrap(),
        })
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::Advance {
            operation: Box::new(operation.clone()),
            path,
            from: PathPhase::Pending,
            to: PathPhase::Applied,
        })
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::Outcome {
            operation: Box::new(operation.clone()),
            result: RestoreOutcome::Restored {
                changed: PathCount::new(1).unwrap(),
            },
        })
        .await
        .unwrap();
    assert_eq!(
        store.checkpoint_event_seq(operation.clone()).await.unwrap(),
        None
    );
    let ended_at = DateTime::from_timestamp(1, 0).unwrap();
    store
        .append(
            info.id,
            vec![Arc::new(EventRecord {
                seq: EventSeq::new(1),
                at: ended_at,
                event: SessionEvent::Status {
                    status: SessionStatus::Exited { code: Some(0) },
                },
            })],
        )
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::EndedMetadata {
            operation: Box::new(operation.clone()),
            at: DateTime::from_timestamp(2, 0).unwrap(),
        })
        .await
        .unwrap();
    assert_eq!(
        store.checkpoint_event_seq(operation.clone()).await.unwrap(),
        Some(EventSeq::new(2))
    );
    let history = store
        .read_events(info.id, EventSeq::ZERO, 10)
        .await
        .unwrap();
    store.close().await;
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    store
        .checkpoint_write(CheckpointWrite::EndedMetadata {
            operation: Box::new(operation),
            at: DateTime::from_timestamp(3, 0).unwrap(),
        })
        .await
        .unwrap();
    assert_eq!(
        store
            .read_events(info.id, EventSeq::ZERO, 10)
            .await
            .unwrap(),
        history
    );
    let info = store.session(info.id).await.unwrap().unwrap();
    assert_eq!(info.ended_at, Some(ended_at));
    assert_eq!(info.status, SessionStatus::Exited { code: Some(0) });
    store.close().await;
}
