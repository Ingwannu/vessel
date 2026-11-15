//! Real transaction/reopen boundaries preserve target metadata without rewriting acceptance.

use super::tests::{admit, database, prompt, session};
use super::{ReceiptAdmission, binding, finish_receipt, observation::observe_input, usage, verify};
use vessel_proto::timeline::{
    ObservePromptReceiptReply, PromptDisposition, PromptReceiptBinding, SubmissionAcceptedKind,
};
use vessel_proto::{Creator, DeviceId, EventSeq, QueueEntryId};

#[test]
fn a_settled_turn_is_immutable_and_observation_still_binds_the_original_creator() {
    let mut db = database();
    let id = session(&db);
    let request = prompt(id, "message", "original");
    let ReceiptAdmission::New(token) = admit(&mut db, &request) else {
        panic!("fresh")
    };
    let first = token.binding_token();
    let duplicate = token.binding_token();
    let stale = token.binding_token();
    let transaction = db.transaction().unwrap();
    let receipt = finish_receipt(
        &transaction,
        token,
        SubmissionAcceptedKind::Prompt {
            disposition: PromptDisposition::Started,
        },
    )
    .unwrap();
    transaction.commit().unwrap();
    let before = usage(&db, None).unwrap();
    let target = PromptReceiptBinding::Turn {
        started: EventSeq::new(17),
    };
    let transaction = db.transaction().unwrap();
    binding::record(&transaction, first, target).unwrap();
    binding::record(&transaction, duplicate, target).unwrap();
    assert!(
        binding::record(
            &transaction,
            stale,
            PromptReceiptBinding::Turn {
                started: EventSeq::new(99)
            }
        )
        .is_err()
    );
    transaction.commit().unwrap();
    assert_eq!(
        observe_input(&db, &request, &Creator::Owner).unwrap(),
        ObservePromptReceiptReply::Accepted {
            receipt,
            binding: target
        }
    );
    assert!(matches!(
        observe_input(
            &db,
            &request,
            &Creator::Device {
                id: DeviceId::random()
            }
        )
        .unwrap(),
        ObservePromptReceiptReply::KeyConflict { .. }
    ));
    assert_eq!(usage(&db, None).unwrap(), before);
    verify(&db).unwrap();
}

#[test]
fn withdrawn_requires_the_original_queue_entry_and_pending_is_never_accepted() {
    let mut db = database();
    let id = session(&db);
    let request = prompt(id, "queue", "original");
    let ReceiptAdmission::New(token) = admit(&mut db, &request) else {
        panic!("fresh")
    };
    let pending = token.binding_token();
    let wrong = token.binding_token();
    let actual = token.binding_token();
    let transaction = db.transaction().unwrap();
    assert!(
        binding::record(
            &transaction,
            pending,
            PromptReceiptBinding::Turn {
                started: EventSeq::new(17)
            }
        )
        .is_err()
    );
    transaction.commit().unwrap();
    let entry = QueueEntryId::random();
    let transaction = db.transaction().unwrap();
    let _receipt = finish_receipt(
        &transaction,
        token,
        SubmissionAcceptedKind::Prompt {
            disposition: PromptDisposition::Queued { entry },
        },
    )
    .unwrap();
    transaction.commit().unwrap();
    let transaction = db.transaction().unwrap();
    assert!(
        binding::record(
            &transaction,
            wrong,
            PromptReceiptBinding::Withdrawn {
                entry: QueueEntryId::random()
            }
        )
        .is_err()
    );
    binding::record(
        &transaction,
        actual,
        PromptReceiptBinding::Withdrawn { entry },
    )
    .unwrap();
    transaction.commit().unwrap();
    assert!(
        matches!(observe_input(&db, &request, &Creator::Owner).unwrap(), ObservePromptReceiptReply::Accepted { binding: PromptReceiptBinding::Withdrawn { entry: known }, .. } if known == entry)
    );
}

#[test]
fn failed_binding_commit_is_unobserved_and_committed_binding_survives_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("bindings.sqlite");
    let mut db = rusqlite::Connection::open(&path).unwrap();
    crate::schema::configure(&db).unwrap();
    crate::schema::migrate(&mut db).unwrap();
    let id = session(&db);
    let request = prompt(id, "message", "original");
    let ReceiptAdmission::New(token) = admit(&mut db, &request) else {
        panic!("fresh")
    };
    let abandoned = token.binding_token();
    let committed = token.binding_token();
    let transaction = db.transaction().unwrap();
    let _receipt = finish_receipt(
        &transaction,
        token,
        SubmissionAcceptedKind::Prompt {
            disposition: PromptDisposition::Started,
        },
    )
    .unwrap();
    transaction.commit().unwrap();
    let target = PromptReceiptBinding::Turn {
        started: EventSeq::new(17),
    };
    let transaction = db.transaction().unwrap();
    binding::record(&transaction, abandoned, target).unwrap();
    transaction.rollback().unwrap();
    assert!(matches!(
        observe_input(&db, &request, &Creator::Owner).unwrap(),
        ObservePromptReceiptReply::Accepted {
            binding: PromptReceiptBinding::Unobserved,
            ..
        }
    ));
    let transaction = db.transaction().unwrap();
    binding::record(&transaction, committed, target).unwrap();
    transaction.commit().unwrap();
    let before = usage(&db, None).unwrap();
    drop(db);
    let reopened = rusqlite::Connection::open(path).unwrap();
    crate::schema::configure(&reopened).unwrap();
    verify(&reopened).unwrap();
    assert_eq!(usage(&reopened, None).unwrap(), before);
    assert!(
        matches!(observe_input(&reopened, &request, &Creator::Owner).unwrap(), ObservePromptReceiptReply::Accepted { binding, .. } if binding == target)
    );
}
