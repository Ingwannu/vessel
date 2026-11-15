//! SQL policy/accounting only; physical collection is a separate node obligation.
use super::{
    journal, publication, quota, rotation,
    writer_tests::{info, plan},
};
use crate::ByteCount;
use chrono::{DateTime, TimeDelta};
use rusqlite::Connection;
use vessel_proto::checkpoint::{CHECKPOINTS_PER_SESSION, CheckpointId, CheckpointState};
fn used(db: &Connection) -> i64 {
    db.query_row("SELECT COALESCE((SELECT sum(reserved_bytes) FROM checkpoint_captures),0)+COALESCE((SELECT sum(reserved_bytes) FROM checkpoint_allocations),0)+COALESCE((SELECT sum(reserved_bytes) FROM checkpoint_garbage),0)",[],|row|row.get(0)).unwrap()
}
#[test]
fn checkpoint_rotation_keeps_worker_and_preview_pins_and_transfers_accounting_atomically() {
    let mut db = Connection::open_in_memory().unwrap();
    crate::schema::configure_writer(&db).unwrap();
    crate::schema::migrate(&mut db).unwrap();
    let session = info();
    crate::log::create_session(&db, &session).unwrap();
    let mut records = Vec::new();
    for index in 0..CHECKPOINTS_PER_SESSION.checked_add(3).unwrap() {
        let plan = plan(session.id);
        let at = DateTime::UNIX_EPOCH
            .checked_add_signed(TimeDelta::seconds(i64::try_from(index).unwrap()))
            .unwrap();
        journal::candidate(&db, plan.key, ByteCount::new(1_000_000), at).unwrap();
        journal::keep_candidate(
            &db,
            plan.key,
            &super::tests::observed(plan.desired.image().clone()),
        )
        .unwrap();
        let checkpoint = CheckpointId::random();
        publication::activate(&db, plan.key, checkpoint).unwrap();
        if index != 1 {
            quota::model_without_worker(&db, plan.key).unwrap();
        }
        records.push((plan, checkpoint));
    }
    // Oldest pinned preview and second-oldest real worker are additional to 40.
    journal::put_plan(&db, &records[0].0, DateTime::UNIX_EPOCH).unwrap();
    let before = used(&db);
    let at = DateTime::UNIX_EPOCH
        .checked_add_signed(TimeDelta::seconds(100))
        .unwrap();
    let transaction = db.transaction().unwrap();
    rotation::collect(&transaction, at).unwrap();
    assert_eq!(
        used(&transaction),
        before,
        "no physical deletion has happened"
    );
    let expired: i64 = transaction
        .query_row(
            "SELECT count(*) FROM checkpoint_captures WHERE state='expired'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(expired, 1);
    transaction.rollback().unwrap();
    assert_eq!(used(&db), before);
    assert!(super::garbage::pending(&db).unwrap().is_empty());
    let transaction = db.transaction().unwrap();
    rotation::collect(&transaction, at).unwrap();
    transaction.commit().unwrap();
    let marks = super::list::page(&db, session.id, None).unwrap();
    assert_eq!(
        marks
            .marks
            .iter()
            .filter(|mark| mark.state == CheckpointState::Expired)
            .count(),
        1
    );
    let (obsolete, _) = &records[2];
    assert!(super::plans::owned(&db, records[0].0.id, records[0].0.owner).is_ok());
    let old = super::list::page(
        &db,
        session.id,
        Some(vessel_proto::checkpoint::CheckpointCursor(
            obsolete.key.capture,
        )),
    )
    .unwrap();
    assert_eq!(
        old.marks.len(),
        2,
        "expired cursor still locates its boundary"
    );
    let garbage = super::garbage::pending(&db).unwrap();
    assert_eq!(garbage.len(), 1);
    assert_eq!(used(&db), before);
    // A later session deletion must merge into existing physical garbage, rather
    // than fail its transaction or lose the boundary metadata's remaining charge.
    db.execute("DELETE FROM checkpoint_plans", []).unwrap();
    quota::model_without_worker(&db, records[1].0.key).unwrap();
    super::garbage::retire_session(&db, session.id, at).unwrap();
    assert_eq!(used(&db), before);
    assert_eq!(super::garbage::pending(&db).unwrap().len(), records.len());
}

#[test]
fn checkpoint_private_expiry_never_collects_a_current_worker_and_restart_fences_old_publication() {
    use super::{Allocation, CheckpointWrite, allocation, records::CheckpointStoreError};
    use vessel_proto::checkpoint::{CheckpointError, RestorePlanId};
    let mut db = Connection::open_in_memory().unwrap();
    crate::schema::configure_writer(&db).unwrap();
    crate::schema::migrate(&mut db).unwrap();
    let session = info();
    crate::log::create_session(&db, &session).unwrap();
    let plan = plan(session.id);
    journal::candidate(
        &db,
        plan.key,
        ByteCount::new(1_000_000),
        DateTime::UNIX_EPOCH,
    )
    .unwrap();
    journal::keep_candidate(
        &db,
        plan.key,
        &super::tests::observed(plan.desired.image().clone()),
    )
    .unwrap();
    publication::activate(&db, plan.key, CheckpointId::random()).unwrap();
    quota::model_without_worker(&db, plan.key).unwrap();
    let unpublished = Allocation::Preview {
        plan: RestorePlanId::random(),
    };
    let published = Allocation::Preview { plan: plan.id };
    for allocation in [unpublished, published] {
        allocation::reserve(&db, plan.key, allocation, plan.owner, DateTime::UNIX_EPOCH).unwrap();
    }
    journal::put_plan(&db, &plan, DateTime::UNIX_EPOCH).unwrap();
    rotation::collect(&db, DateTime::UNIX_EPOCH).unwrap();
    assert!(
        super::garbage::pending(&db).unwrap().is_empty(),
        "elapsed time is not proof that a current worker stopped"
    );
    super::retention::startup(&mut db).unwrap();
    assert!(matches!(
        super::write::apply(
            &db,
            CheckpointWrite::Plan {
                plan: Box::new(plan.clone()),
                at: DateTime::UNIX_EPOCH
            }
        ),
        Err(crate::StoreError::Checkpoint(
            CheckpointStoreError::Refused(CheckpointError::RestoreIdConflict)
        ))
    ));
    let current = Allocation::Preview {
        plan: RestorePlanId::random(),
    };
    allocation::reserve(&db, plan.key, current, plan.owner, DateTime::UNIX_EPOCH).unwrap();
    let before = used(&db);
    rotation::collect(&db, DateTime::UNIX_EPOCH).unwrap();
    let garbage = super::garbage::pending(&db).unwrap();
    assert_eq!(
        garbage.len(),
        1,
        "only old unpublished allocation is garbage"
    );
    assert_eq!(
        used(&db),
        before,
        "old private payload remains fully charged"
    );
    let at = DateTime::UNIX_EPOCH
        .checked_add_signed(TimeDelta::seconds(600))
        .unwrap();
    rotation::collect(&db, at).unwrap();
    assert_eq!(used(&db), before);
    assert_eq!(
        super::garbage::pending(&db).unwrap().len(),
        2,
        "expired published preview joins garbage after old-worker reconciliation"
    );
    let phase: String = db
        .query_row(
            "SELECT phase FROM checkpoint_allocations WHERE object=?1",
            [serde_json::to_string(&current).unwrap()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        phase, "reserved",
        "a fresh actual worker is never swept as a prior-process orphan"
    );
    assert!(allocation::preview_reserved(&db, &plan).is_err());
}

#[test]
fn checkpoint_pressure_plans_reclamation_but_never_spends_pending_garbage_early() {
    use vessel_proto::checkpoint::CHECKPOINT_CAPTURE_RESERVATION_BYTES;
    let mut db = Connection::open_in_memory().unwrap();
    crate::schema::configure_writer(&db).unwrap();
    crate::schema::migrate(&mut db).unwrap();
    let session = info();
    crate::log::create_session(&db, &session).unwrap();
    for index in 0..4 {
        let plan = plan(session.id);
        let at = DateTime::UNIX_EPOCH
            .checked_add_signed(TimeDelta::seconds(index))
            .unwrap();
        journal::candidate(
            &db,
            plan.key,
            ByteCount::new(CHECKPOINT_CAPTURE_RESERVATION_BYTES),
            at,
        )
        .unwrap();
        journal::keep_candidate(
            &db,
            plan.key,
            &super::tests::observed(plan.desired.image().clone()),
        )
        .unwrap();
        publication::activate(&db, plan.key, CheckpointId::random()).unwrap();
        quota::model_without_worker(&db, plan.key).unwrap();
    }
    let charge = used(&db);
    rotation::collect(&db, DateTime::UNIX_EPOCH).unwrap();
    let garbage = super::garbage::pending(&db).unwrap();
    assert_eq!(
        garbage.len(),
        2,
        "preserved tombstones mean one envelope is not quite enough"
    );
    assert_eq!(used(&db), charge);
    assert!(quota::available(&db, ByteCount::new(CHECKPOINT_CAPTURE_RESERVATION_BYTES)).is_err());
    rotation::collect(&db, DateTime::UNIX_EPOCH).unwrap();
    assert_eq!(
        super::garbage::pending(&db).unwrap(),
        garbage,
        "already planned GC prevents over-pruning"
    );
    for item in garbage {
        super::garbage::cleaned(&db, item.object).unwrap();
    }
    assert!(quota::available(&db, ByteCount::new(CHECKPOINT_CAPTURE_RESERVATION_BYTES)).is_ok());
}

#[tokio::test]
async fn checkpoint_completed_payload_retirement_keeps_idempotent_results_after_image_expiry_and_restart()
 {
    use super::{Allocation, CheckpointWrite};
    use crate::{DataDir, Store, StoreConfig};
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    dir.prepare().unwrap();
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    let operation = super::metadata_tests::completed(&store).await;
    let (selected, _) = store
        .checkpoint_plan(operation.plan, operation.owner)
        .await
        .unwrap();
    for allocation in [
        Allocation::Preview { plan: selected.id },
        Allocation::Restore {
            restore: operation.id,
        },
    ] {
        store
            .checkpoint_write(CheckpointWrite::AllocationAttached {
                key: selected.key,
                allocation,
                owner: selected.owner,
            })
            .await
            .unwrap();
    }
    store
        .checkpoint_write(CheckpointWrite::Collect {
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
    assert!(
        store.checkpoint_garbage().await.unwrap().is_empty(),
        "missing result marker pins completed payloads too"
    );
    let _ended_at = super::metadata_tests::ended(&store, &operation).await;
    store
        .checkpoint_write(CheckpointWrite::EndedMetadata {
            operation: Box::new(operation.clone()),
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
    store
        .checkpoint_write(CheckpointWrite::Collect {
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
    let garbage = store.checkpoint_garbage().await.unwrap();
    assert_eq!(
        garbage.len(),
        2,
        "preview and restore payloads retire without forgetting result metadata"
    );
    assert_eq!(
        store
            .checkpoint_operation(operation.session, operation.id, operation.owner)
            .await
            .unwrap(),
        operation
    );
    // Accounting-only fixture: physical cleanup is exercised by workspace/node
    // fixtures; these explicit acks model its verified terminal completion.
    for item in garbage {
        store
            .checkpoint_write(CheckpointWrite::GarbageCleaned {
                object: item.object,
            })
            .await
            .unwrap();
    }
    for index in 1..=CHECKPOINTS_PER_SESSION {
        let newer = plan(operation.session);
        let at = DateTime::UNIX_EPOCH
            .checked_add_signed(TimeDelta::seconds(i64::try_from(index).unwrap()))
            .unwrap();
        store
            .checkpoint_write(CheckpointWrite::Reserve {
                key: newer.key,
                bytes: ByteCount::new(1),
                at,
            })
            .await
            .unwrap();
        store
            .checkpoint_write(CheckpointWrite::Candidate {
                key: newer.key,
                snapshot: Box::new(super::tests::observed(newer.desired.image().clone())),
            })
            .await
            .unwrap();
        store
            .checkpoint_write(CheckpointWrite::Activate {
                key: newer.key,
                checkpoint: CheckpointId::random(),
            })
            .await
            .unwrap();
    }
    store
        .checkpoint_write(CheckpointWrite::Collect {
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
    let marks = store
        .checkpoint_page(operation.session, None)
        .await
        .unwrap();
    assert_eq!(
        marks
            .marks
            .iter()
            .filter(|mark| mark.state == CheckpointState::Expired)
            .count(),
        1
    );
    assert!(
        store
            .checkpoint_plan(operation.plan, operation.owner)
            .await
            .is_err(),
        "expired image never authorizes a new restore"
    );
    assert_eq!(
        store
            .checkpoint_operation(operation.session, operation.id, operation.owner)
            .await
            .unwrap(),
        operation
    );
    assert!(
        store
            .checkpoint_event_seq(operation.clone())
            .await
            .unwrap()
            .is_some()
    );
    store.close().await;
    let store = Store::open(&dir, StoreConfig::default())
        .await
        .unwrap()
        .store;
    store
        .checkpoint_write(CheckpointWrite::Admit {
            plan: Box::new(selected),
            restore: operation.id,
            at: DateTime::UNIX_EPOCH,
        })
        .await
        .unwrap();
    assert_eq!(
        store
            .checkpoint_operation(operation.session, operation.id, operation.owner)
            .await
            .unwrap(),
        operation,
        "same-id retry after expiry/restart reads the completed outcome"
    );
    assert_eq!(
        store
            .read_events(operation.session, vessel_proto::EventSeq::ZERO, 10)
            .await
            .unwrap()
            .records
            .len(),
        4
    );
    store.close().await;
}
