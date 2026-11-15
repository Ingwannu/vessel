use super::*;

#[test]
fn one_migration_creates_all_tables_and_rolls_back_atomically() {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("CREATE TABLE sessions(id TEXT PRIMARY KEY);")
        .unwrap();
    {
        let transaction = connection.transaction().unwrap();
        install_staged(&transaction).unwrap();
        let names: Vec<String> = transaction.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'checkpoint_%' ORDER BY name").unwrap()
            .query_map([], |row| row.get(0)).unwrap().collect::<Result<_,_>>().unwrap();
        assert_eq!(
            names,
            [
                "checkpoint_allocations",
                "checkpoint_captures",
                "checkpoint_garbage",
                "checkpoint_paths",
                "checkpoint_plans",
                "checkpoint_restores"
            ]
        );
        transaction.rollback().unwrap();
    }
    let count: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name LIKE 'checkpoint_%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(CHECKPOINTS_SCHEMA, super::super::schema::migration_count());
}
#[test]
#[ignore = "skeleton: wire the typed journal SQL operations before enabling the feature"]
fn duplicate_restore_id_returns_durable_outcome_without_reapplying_paths() {
    // Proof obligation: implement durable idempotency.
}
#[test]
#[ignore = "skeleton: wire candidate publication fencing and restart reconciliation"]
fn expired_candidate_is_never_promoted_by_restart_or_late_storage_completion() {
    // Proof obligation: implement publication recovery.
}

pub(super) fn evidence() -> vessel_proto::checkpoint::SnapshotEvidence {
    use vessel_proto::{
        BlobHash,
        checkpoint::{IgnoreFingerprint, RootFingerprint, SnapshotEvidence},
    };
    SnapshotEvidence::new(
        vessel_proto_physical(),
        RootFingerprint::new(BlobHash::from_bytes([1; 32])),
        IgnoreFingerprint::new(BlobHash::from_bytes([2; 32])),
        Vec::new(),
    )
    .unwrap()
}

fn vessel_proto_physical() -> vessel_proto::checkpoint::PhysicalRoot {
    vessel_proto::checkpoint::PhysicalRoot::new(vessel_proto::BlobHash::from_bytes([0; 32]))
}

pub(super) fn observed(
    image: vessel_proto::checkpoint::FileImage,
) -> vessel_proto::checkpoint::ObservedSnapshot {
    use vessel_proto::{
        BlobHash,
        checkpoint::{
            FileFingerprint, FileState, ObjectFingerprint, ObservedEntry, ObservedPresence,
            ObservedSnapshot, SnapshotEvidence,
        },
    };
    let entries = image
        .entries()
        .iter()
        .map(|entry| ObservedEntry {
            path: entry.path.clone(),
            presence: match entry.state {
                FileState::Absent => ObservedPresence::Absent,
                FileState::Present { .. } => ObservedPresence::Present {
                    full: FileFingerprint::new(BlobHash::from_bytes([3; 32])),
                    object: ObjectFingerprint::new(BlobHash::from_bytes([4; 32])),
                },
            },
        })
        .collect();
    let scope = evidence();
    let evidence =
        SnapshotEvidence::new(scope.physical(), scope.root(), scope.ignore(), entries).unwrap();
    ObservedSnapshot::new(image, evidence).unwrap()
}

pub(super) fn staged(plan: &Plan) -> vessel_proto::checkpoint::StagedPaths {
    use sha2::Digest as _;
    use vessel_proto::checkpoint::{
        FileState, ObjectFingerprint, ObservedPresence, StagedPath, StagedPaths,
    };
    StagedPaths::new(
        plan.desired
            .changes(&plan.current)
            .into_iter()
            .map(|change| {
                let before = plan
                    .evidence
                    .entries()
                    .iter()
                    .find(|entry| entry.path == change.path)
                    .and_then(|entry| match entry.presence {
                        ObservedPresence::Absent => None,
                        ObservedPresence::Present { object, .. } => Some(object),
                    });
                let desired = match plan.desired.image().get(&change.path).unwrap() {
                    FileState::Absent => None,
                    FileState::Present { .. } => {
                        Some(ObjectFingerprint::new(vessel_proto::BlobHash::from_bytes(
                            sha2::Sha256::digest(change.path.to_string().as_bytes()).into(),
                        )))
                    }
                };
                StagedPath {
                    path: change.path,
                    before,
                    desired,
                }
            })
            .collect(),
    )
    .unwrap()
}
