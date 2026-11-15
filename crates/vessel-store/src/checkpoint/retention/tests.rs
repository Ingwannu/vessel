//! SQL startup/retention accounting; these fixtures do not prove filesystem cleanup.
use super::*;
use crate::{
    ByteCount,
    checkpoint::{
        journal, paths, publication,
        writer_tests::{info, plan as make_plan},
    },
};
use vessel_proto::{
    EventRecord, EventSeq, SessionEvent, SessionStatus,
    checkpoint::{CheckpointId, RestoreId, RestoreOutcome},
};

fn database() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    crate::schema::configure_writer(&db).unwrap();
    crate::schema::migrate(&mut db).unwrap();
    db
}

fn capture(db: &Connection) -> super::super::records::Plan {
    let info = info();
    crate::log::create_session(db, &info).unwrap();
    let plan = make_plan(info.id);
    journal::candidate(db, plan.key, ByteCount::new(1), DateTime::UNIX_EPOCH).unwrap();
    journal::keep_candidate(
        db,
        plan.key,
        &super::super::tests::observed(plan.desired.image().clone()),
    )
    .unwrap();
    publication::activate(db, plan.key, CheckpointId::random()).unwrap();
    plan
}

fn end(db: &Connection, id: SessionId) {
    crate::log::append(
        db,
        id,
        [&EventRecord {
            seq: EventSeq::ZERO,
            at: DateTime::UNIX_EPOCH,
            event: SessionEvent::Status {
                status: SessionStatus::Exited { code: Some(0) },
            },
        }],
    )
    .unwrap();
}

fn cap() -> Retention {
    Retention {
        max_ended_sessions: 0,
        ..Retention::default()
    }
}

#[test]
fn checkpoint_retention_rolls_back_garbage_transfer_with_session_deletion() {
    let mut db = database();
    let plan = capture(&db);
    end(&db, plan.key.session);
    let transaction = db.transaction().unwrap();
    assert_eq!(
        prune(&transaction, &cap(), Utc::now()).unwrap(),
        vec![plan.key.session]
    );
    assert_eq!(garbage::pending(&transaction).unwrap().len(), 1);
    transaction.rollback().unwrap();
    assert!(
        crate::reader::select_session(&db, plan.key.session)
            .unwrap()
            .is_some()
    );
    assert!(garbage::pending(&db).unwrap().is_empty());
    let transaction = db.transaction().unwrap();
    assert_eq!(
        prune(&transaction, &cap(), Utc::now()).unwrap(),
        vec![plan.key.session]
    );
    transaction.commit().unwrap();
    assert!(
        crate::reader::select_session(&db, plan.key.session)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        garbage::pending(&db).unwrap()[0].reserved,
        ByteCount::new(1)
    );
}

#[test]
fn checkpoint_startup_fences_candidates_and_pins_prepared_restores_before_retention() {
    let mut db = database();
    let plan = capture(&db);
    journal::put_plan(&db, &plan, DateTime::UNIX_EPOCH).unwrap();
    let restore = RestoreId::random();
    let operation = journal::admit(&db, &plan, restore, DateTime::UNIX_EPOCH).unwrap();
    paths::prepare(&db, &operation).unwrap();
    end(&db, plan.key.session);
    let other = info();
    crate::log::create_session(&db, &other).unwrap();
    let candidate = make_plan(other.id);
    journal::candidate(&db, candidate.key, ByteCount::new(9), DateTime::UNIX_EPOCH).unwrap();
    end(&db, other.id);
    startup(&mut db).unwrap();
    startup(&mut db).unwrap();
    assert!(publication::activate(&db, candidate.key, CheckpointId::random()).is_err());
    assert_eq!(
        journal::status(&db, operation.session, operation.id, operation.owner)
            .unwrap()
            .outcome,
        RestoreOutcome::RecoveryRequired { restore }
    );
    let transaction = db.transaction().unwrap();
    assert_eq!(
        prune(&transaction, &cap(), Utc::now()).unwrap(),
        vec![other.id]
    );
    transaction.commit().unwrap();
    assert!(
        crate::reader::select_session(&db, plan.key.session)
            .unwrap()
            .is_some()
    );
    assert_eq!(
        garbage::pending(&db).unwrap()[0].reserved,
        ByteCount::new(9)
    );
}

#[test]
fn checkpoint_success_waits_for_metadata_even_after_its_preview_expired() {
    let mut db = database();
    let plan = capture(&db);
    journal::put_plan(&db, &plan, DateTime::UNIX_EPOCH).unwrap();
    let operation = journal::admit(&db, &plan, RestoreId::random(), DateTime::UNIX_EPOCH).unwrap();
    paths::prepare(&db, &operation).unwrap();
    super::super::super::staging::commit(
        &db,
        &operation,
        &super::super::super::tests::staged(&plan),
    )
    .unwrap();
    paths::advance(
        &db,
        &operation,
        &serde_json::from_str("\"file\"").unwrap(),
        paths::Phase::Pending,
        paths::Phase::Applied,
    )
    .unwrap();
    journal::outcome(
        &db,
        &operation,
        &RestoreOutcome::Restored {
            changed: vessel_proto::checkpoint::PathCount::new(1).unwrap(),
        },
    )
    .unwrap();
    end(&db, operation.session);
    let transaction = db.transaction().unwrap();
    assert!(prune(&transaction, &cap(), Utc::now()).unwrap().is_empty());
    super::super::super::metadata::append_ended(&transaction, &operation, Utc::now()).unwrap();
    assert_eq!(
        prune(&transaction, &cap(), Utc::now()).unwrap(),
        vec![operation.session]
    );
    transaction.commit().unwrap();
    assert_eq!(garbage::pending(&db).unwrap().len(), 1);
}
