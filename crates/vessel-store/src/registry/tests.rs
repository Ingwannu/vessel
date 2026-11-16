//! Adversarial registry transactions: independent identity, atomic CAS and ownership.

use super::*;
use chrono::DateTime;
use rusqlite::Connection;
use vessel_proto::registry::{
    RegistryRevision, WorkspaceBacking, WorkspaceDescriptor, WorkspaceIsolation,
    WorkspaceLifecycle, WorkspaceResource, WorkspaceVisibility,
};
use vessel_proto::{ProjectId, SessionId, WorkspaceId};

fn db() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    crate::schema::configure(&db).unwrap();
    crate::schema::migrate(&mut db).unwrap();
    db
}
fn record(id: WorkspaceId) -> RegistryRecord {
    RegistryRecord {
        descriptor: WorkspaceDescriptor {
            id,
            project: ProjectId::random(),
            path: "/work/project".into(),
            automation: vessel_proto::registry::WorkspaceAutomation::Allowed,
            title: None,
            pin: vessel_proto::registry::WorkspacePin::Unpinned,
            backing: WorkspaceBacking::Local,
            lifecycle: WorkspaceLifecycle::Active,
            visibility: WorkspaceVisibility::Foreground,
            created_at: DateTime::UNIX_EPOCH,
            revision: RegistryRevision::ZERO,
        },
        recovery: None,
    }
}
fn apply(db: &mut Connection, change: &RegistryWrite) -> Result<(), StoreError> {
    let transaction = db.transaction()?;
    let _change = super::apply(&transaction, change)?;
    transaction.commit().map_err(StoreError::from)
}
fn put(
    db: &mut Connection,
    record: RegistryRecord,
    expected: Option<RegistryRevision>,
) -> Result<(), StoreError> {
    apply(
        db,
        &RegistryWrite::Workspace {
            record: Box::new(record),
            expected,
        },
    )
}

#[test]
fn shared_cwd_remains_two_workspaces_and_stale_cas_changes_nothing() {
    let mut db = db();
    let first = record(WorkspaceId::random());
    let second = RegistryRecord {
        descriptor: WorkspaceDescriptor {
            id: WorkspaceId::random(),
            ..first.descriptor.clone()
        },
        recovery: None,
    };
    put(&mut db, first.clone(), None).unwrap();
    put(&mut db, second.clone(), None).unwrap();
    let original = load(&db).unwrap();
    assert_eq!(original.workspaces.len(), 2);
    assert_eq!(original.workspaces[0].path, original.workspaces[1].path);
    let mut changed = first.clone();
    changed.descriptor.revision = RegistryRevision::ZERO.next().unwrap();
    changed.descriptor.title = Some("First workspace".parse().unwrap());
    put(&mut db, changed.clone(), Some(RegistryRevision::ZERO)).unwrap();
    let after = load(&db).unwrap();
    assert!(matches!(
        put(&mut db, changed, Some(RegistryRevision::ZERO)),
        Err(StoreError::RegistryConflict)
    ));
    assert_eq!(load(&db).unwrap(), after);
    assert_eq!(workspace(&db, second.descriptor.id).unwrap(), Some(second));
}

#[test]
fn membership_is_unique_and_archiving_claim_blocks_new_admission() {
    let mut db = db();
    let first = record(WorkspaceId::random());
    let second = record(WorkspaceId::random());
    put(&mut db, first.clone(), None).unwrap();
    put(&mut db, second.clone(), None).unwrap();
    let resource = WorkspaceResource::Session {
        session: SessionId::random(),
    };
    apply(
        &mut db,
        &RegistryWrite::Membership {
            workspace: first.descriptor.id,
            resource,
            expected: RegistryRevision::ZERO,
        },
    )
    .unwrap();
    let snapshot = load(&db).unwrap();
    assert!(matches!(
        apply(
            &mut db,
            &RegistryWrite::Membership {
                workspace: second.descriptor.id,
                resource,
                expected: RegistryRevision::ZERO
            }
        ),
        Err(StoreError::RegistryConflict)
    ));
    assert_eq!(load(&db).unwrap(), snapshot);
    let mut archiving = workspace(&db, first.descriptor.id).unwrap().unwrap();
    let expected = archiving.descriptor.revision;
    archiving.descriptor.revision = expected.next().unwrap();
    archiving.descriptor.lifecycle = WorkspaceLifecycle::Archiving {
        at: DateTime::UNIX_EPOCH,
    };
    put(&mut db, archiving.clone(), Some(expected)).unwrap();
    let before = load(&db).unwrap();
    let resource = WorkspaceResource::Session {
        session: SessionId::random(),
    };
    assert!(matches!(
        apply(
            &mut db,
            &RegistryWrite::Membership {
                workspace: first.descriptor.id,
                resource,
                expected: archiving.descriptor.revision
            }
        ),
        Err(StoreError::RegistryInactive(_))
    ));
    assert_eq!(load(&db).unwrap(), before);
}

#[test]
fn member_admission_invalidates_an_archive_snapshot_taken_before_it() {
    let mut db = db();
    let first = record(WorkspaceId::random());
    put(&mut db, first.clone(), None).unwrap();
    apply(
        &mut db,
        &RegistryWrite::Membership {
            workspace: first.descriptor.id,
            resource: WorkspaceResource::Session {
                session: SessionId::random(),
            },
            expected: RegistryRevision::ZERO,
        },
    )
    .unwrap();
    let mut stale_archive = first;
    stale_archive.descriptor.revision = RegistryRevision::ZERO.next().unwrap();
    stale_archive.descriptor.lifecycle = WorkspaceLifecycle::Archiving {
        at: DateTime::UNIX_EPOCH,
    };
    assert!(matches!(
        put(&mut db, stale_archive, Some(RegistryRevision::ZERO)),
        Err(StoreError::RegistryConflict)
    ));
}

#[test]
fn reserved_backing_identity_is_idempotent_but_changed_intent_is_refused() {
    let mut db = db();
    let workspace = WorkspaceId::random();
    let reservation = RegistryReservation {
        workspace,
        intent: WorkspaceProvision {
            project: ProjectId::random(),
            isolation: WorkspaceIsolation::Local,
            automation: vessel_proto::registry::WorkspaceAutomation::Allowed,
            title: None,
            visibility: WorkspaceVisibility::Foreground,
        },
        worktree: None,
        stage: ReservationStage::Reserved,
    };
    apply(&mut db, &RegistryWrite::Reserve(reservation.clone())).unwrap();
    let before_replay = load(&db).unwrap();
    apply(&mut db, &RegistryWrite::Reserve(reservation.clone())).unwrap();
    assert_eq!(load(&db).unwrap(), before_replay);
    let mut conflict = reservation.clone();
    conflict.intent.project = ProjectId::random();
    assert!(matches!(
        apply(&mut db, &RegistryWrite::Reserve(conflict)),
        Err(StoreError::RegistryConflict)
    ));
    assert_eq!(
        super::reservation(&db, workspace).unwrap(),
        Some(reservation)
    );
    apply(
        &mut db,
        &RegistryWrite::Stage {
            workspace,
            expected: ReservationStage::Reserved,
            stage: ReservationStage::Creating,
        },
    )
    .unwrap();
    assert!(matches!(
        apply(
            &mut db,
            &RegistryWrite::Stage {
                workspace,
                expected: ReservationStage::Reserved,
                stage: ReservationStage::Creating
            }
        ),
        Err(StoreError::RegistryConflict)
    ));
}

#[test]
fn corrupt_registry_rows_are_errors_instead_of_partial_snapshots() {
    let mut db = db();
    let first = record(WorkspaceId::random());
    put(&mut db, first, None).unwrap();
    let _changed = db
        .execute("UPDATE workspace_registry SET descriptor='invalid'", [])
        .unwrap();
    assert!(matches!(load(&db), Err(StoreError::RegistryDecode(_))));
}

#[test]
fn failed_mutation_rolls_back_even_when_global_revision_is_exhausted() {
    let mut db = db();
    let _changed = db
        .execute("UPDATE workspace_registry_meta SET revision=?1", [i64::MAX])
        .unwrap();
    let first = record(WorkspaceId::random());
    assert!(matches!(
        put(&mut db, first.clone(), None),
        Err(StoreError::RegistryRevisionExhausted)
    ));
    assert!(workspace(&db, first.descriptor.id).unwrap().is_none());
}

#[test]
fn restart_marks_interrupted_creation_uncertain_without_reallocating_ids() {
    let mut db = db();
    let reservation = RegistryReservation {
        workspace: WorkspaceId::random(),
        intent: WorkspaceProvision {
            project: ProjectId::random(),
            isolation: WorkspaceIsolation::Local,
            automation: vessel_proto::registry::WorkspaceAutomation::Allowed,
            title: None,
            visibility: WorkspaceVisibility::Foreground,
        },
        worktree: None,
        stage: ReservationStage::Creating,
    };
    apply(&mut db, &RegistryWrite::Reserve(reservation.clone())).unwrap();
    apply(&mut db, &RegistryWrite::RecoverCreating).unwrap();
    let found = super::reservation(&db, reservation.workspace)
        .unwrap()
        .unwrap();
    assert_eq!(found.workspace, reservation.workspace);
    assert_eq!(found.stage, ReservationStage::Uncertain);
    let before = load(&db).unwrap();
    apply(&mut db, &RegistryWrite::RecoverCreating).unwrap();
    assert_eq!(load(&db).unwrap(), before);
}

#[test]
fn descriptor_and_ready_stage_commit_or_roll_back_together() {
    let mut db = db();
    let record = record(WorkspaceId::random());
    let reservation = RegistryReservation {
        workspace: record.descriptor.id,
        intent: WorkspaceProvision {
            project: record.descriptor.project,
            isolation: WorkspaceIsolation::Local,
            automation: vessel_proto::registry::WorkspaceAutomation::Allowed,
            title: None,
            visibility: WorkspaceVisibility::Foreground,
        },
        worktree: None,
        stage: ReservationStage::Reserved,
    };
    apply(&mut db, &RegistryWrite::Reserve(reservation.clone())).unwrap();
    assert!(matches!(
        apply(&mut db, &RegistryWrite::Completed(Box::new(record.clone()))),
        Err(StoreError::RegistryConflict)
    ));
    assert!(workspace(&db, record.descriptor.id).unwrap().is_none());
    apply(
        &mut db,
        &RegistryWrite::Stage {
            workspace: record.descriptor.id,
            expected: ReservationStage::Reserved,
            stage: ReservationStage::Creating,
        },
    )
    .unwrap();
    apply(&mut db, &RegistryWrite::Completed(Box::new(record.clone()))).unwrap();
    assert_eq!(workspace(&db, record.descriptor.id).unwrap(), Some(record));
    assert_eq!(
        super::reservation(&db, reservation.workspace)
            .unwrap()
            .unwrap()
            .stage,
        ReservationStage::Ready
    );
}

#[tokio::test]
async fn watchers_only_wake_after_a_committed_registry_change() {
    const WAIT: std::time::Duration = std::time::Duration::from_secs(10);
    tokio::time::timeout(WAIT, async {
        let temp = tempfile::tempdir().unwrap();
        let data = crate::DataDir::new(temp.path().join("data"));
        data.prepare().unwrap();
        let store = crate::Store::open(&data, crate::StoreConfig::default())
            .await
            .unwrap()
            .store;
        let mut changes = store.registry_changes();
        assert!(!changes.has_changed().unwrap());
        store.recover_registry_reservations().await.unwrap();
        assert!(!changes.has_changed().unwrap());
        let record = record(WorkspaceId::random());
        store
            .put_registry_workspace(record.clone(), None)
            .await
            .unwrap();
        changes.changed().await.unwrap();
        let fresh = store.registry_changes();
        assert!(
            !fresh.has_changed().unwrap(),
            "a fresh subscriber must not inherit historical invalidations"
        );
        assert_eq!(
            store
                .registry_workspace(record.descriptor.id)
                .await
                .unwrap(),
            Some(record.clone())
        );
        assert!(matches!(
            store.put_registry_workspace(record, None).await,
            Err(StoreError::RegistryConflict)
        ));
        assert!(!changes.has_changed().unwrap());
        store.close().await;
    })
    .await
    .expect("bounded store invalidation test");
}

#[test]
fn resource_owner_lookup_does_not_match_a_shared_directory() {
    let mut db = db();
    let first = record(WorkspaceId::random());
    let second = RegistryRecord {
        descriptor: WorkspaceDescriptor {
            id: WorkspaceId::random(),
            ..first.descriptor.clone()
        },
        recovery: None,
    };
    put(&mut db, first.clone(), None).unwrap();
    put(&mut db, second.clone(), None).unwrap();
    let resource = WorkspaceResource::Session {
        session: SessionId::random(),
    };
    assert_eq!(resource_owner(&db, resource).unwrap(), None);
    apply(
        &mut db,
        &RegistryWrite::Membership {
            workspace: second.descriptor.id,
            resource,
            expected: RegistryRevision::ZERO,
        },
    )
    .unwrap();
    assert_eq!(
        resource_owner(&db, resource).unwrap(),
        Some(second.descriptor.id)
    );
}

#[test]
fn archive_cannot_finish_before_a_pre_admitted_actor_start_settles() {
    let mut db = db();
    let record = record(WorkspaceId::random());
    let id = record.descriptor.id;
    let resource = WorkspaceResource::Session {
        session: SessionId::random(),
    };
    put(&mut db, record, None).unwrap();
    apply(
        &mut db,
        &RegistryWrite::Claim {
            workspace: id,
            resource,
            expected: RegistryRevision::ZERO,
        },
    )
    .unwrap();
    let lease = WorkspaceStartLease::new(id, resource, RegistryRevision::ZERO.next().unwrap());
    let mut archiving = workspace(&db, id).unwrap().unwrap();
    let expected = archiving.descriptor.revision;
    archiving.descriptor.revision = expected.next().unwrap();
    archiving.descriptor.lifecycle = WorkspaceLifecycle::Archiving {
        at: DateTime::UNIX_EPOCH,
    };
    put(&mut db, archiving, Some(expected)).unwrap();
    let mut archived = workspace(&db, id).unwrap().unwrap();
    let expected = archived.descriptor.revision;
    archived.descriptor.revision = expected.next().unwrap();
    archived.descriptor.lifecycle = WorkspaceLifecycle::Archived {
        at: DateTime::UNIX_EPOCH,
    };
    assert!(matches!(
        put(&mut db, archived, Some(expected)),
        Err(StoreError::RegistryConflict)
    ));
    assert_eq!(pending_starts(&db, id).unwrap().len(), 1);
    apply(
        &mut db,
        &RegistryWrite::Settle {
            lease,
            settled: RegistryStartSettlement::Ready,
        },
    )
    .unwrap();
    assert!(pending_starts(&db, id).unwrap().is_empty());
    let mut archived = workspace(&db, id).unwrap().unwrap();
    let expected = archived.descriptor.revision;
    archived.descriptor.revision = expected.next().unwrap();
    archived.descriptor.lifecycle = WorkspaceLifecycle::Archived {
        at: DateTime::UNIX_EPOCH,
    };
    put(&mut db, archived, Some(expected)).unwrap();
}

#[test]
fn stale_start_completion_cannot_settle_a_newer_retry_for_the_same_actor_id() {
    let mut db = db();
    let record = record(WorkspaceId::random());
    let id = record.descriptor.id;
    let resource = WorkspaceResource::Session {
        session: SessionId::random(),
    };
    put(&mut db, record, None).unwrap();
    apply(
        &mut db,
        &RegistryWrite::Claim {
            workspace: id,
            resource,
            expected: RegistryRevision::ZERO,
        },
    )
    .unwrap();
    let first = WorkspaceStartLease::new(id, resource, RegistryRevision::ZERO.next().unwrap());
    apply(
        &mut db,
        &RegistryWrite::Settle {
            lease: first.clone(),
            settled: RegistryStartSettlement::Failed,
        },
    )
    .unwrap();
    let expected = workspace(&db, id).unwrap().unwrap().descriptor.revision;
    apply(
        &mut db,
        &RegistryWrite::Claim {
            workspace: id,
            resource,
            expected,
        },
    )
    .unwrap();
    let second = WorkspaceStartLease::new(id, resource, expected.next().unwrap());
    assert!(matches!(
        apply(
            &mut db,
            &RegistryWrite::Settle {
                lease: first,
                settled: RegistryStartSettlement::Ready
            }
        ),
        Err(StoreError::RegistryConflict)
    ));
    let pending = pending_starts(&db, id).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].lease, second);
    apply(&mut db, &RegistryWrite::RecoverCreating).unwrap();
    assert_eq!(
        pending_starts(&db, id).unwrap()[0].state,
        RegistryStartState::Uncertain
    );
    apply(
        &mut db,
        &RegistryWrite::Settle {
            lease: second,
            settled: RegistryStartSettlement::Ready,
        },
    )
    .unwrap();
}

mod capacity;

mod automation;

mod restart;

#[test]
fn admission_distinguishes_missing_inactive_and_active_without_changing_state() {
    let mut db = db();
    let id = WorkspaceId::random();
    assert!(
        matches!(write::require_active(&db,id),Err(StoreError::RegistryMissing(missing)) if missing==id)
    );
    let mut record = record(id);
    put(&mut db, record.clone(), None).unwrap();
    assert!(write::require_active(&db, id).is_ok());
    record.descriptor.revision = RegistryRevision::ZERO.next().unwrap();
    record.descriptor.lifecycle = WorkspaceLifecycle::Restoring {
        at: DateTime::UNIX_EPOCH,
    };
    put(&mut db, record, Some(RegistryRevision::ZERO)).unwrap();
    let before = load(&db).unwrap();
    assert!(
        matches!(write::require_active(&db,id),Err(StoreError::RegistryInactive(inactive)) if inactive==id)
    );
    assert_eq!(load(&db).unwrap(), before);
}

mod memberships;
mod placement;
mod project_reference;
