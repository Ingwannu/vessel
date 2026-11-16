//! Query-only `SQLite` proves that recovery cannot silently create admissions.

use super::tests::{admit, database, prompt, session};
use super::{ReceiptAdmission, finish_receipt, observation::observe_input, usage};
use vessel_proto::timeline::{
    ObservePromptReceiptReply, PromptDisposition, PromptReceiptBinding, SubmissionAcceptedKind,
};
use vessel_proto::{Creator, QueueEntryId};

#[test]
fn missing_pending_and_conflicting_observation_succeed_on_a_query_only_connection() {
    let mut db = database();
    let id = session(&db);
    let original = prompt(id, "pending", "same original");
    assert!(matches!(
        admit(&mut db, &original),
        ReceiptAdmission::New(_)
    ));
    let before = usage(&db, None).unwrap();
    db.pragma_update(None, "query_only", true).unwrap();
    assert!(matches!(
        observe_input(&db, &original, &Creator::Owner).unwrap(),
        ObservePromptReceiptReply::OutcomeUnknown { .. }
    ));
    let missing = prompt(id, "missing", "no admission");
    assert!(matches!(
        observe_input(&db, &missing, &Creator::Owner).unwrap(),
        ObservePromptReceiptReply::Unobserved { .. }
    ));
    let changed = prompt(id, "pending", "different original");
    assert!(matches!(
        observe_input(&db, &changed, &Creator::Owner).unwrap(),
        ObservePromptReceiptReply::KeyConflict { .. }
    ));
    assert_eq!(usage(&db, None).unwrap(), before);
}

#[test]
fn accepted_observation_keeps_the_exact_queue_and_never_guesses_a_turn() {
    let mut db = database();
    let id = session(&db);
    let queued = prompt(id, "queued", "queue");
    let direct = prompt(id, "direct", "start");
    let entry = QueueEntryId::random();
    for (request, disposition) in [
        (&queued, PromptDisposition::Queued { entry }),
        (&direct, PromptDisposition::Started),
    ] {
        let ReceiptAdmission::New(token) = admit(&mut db, request) else {
            panic!("fresh")
        };
        let transaction = db.transaction().unwrap();
        let _receipt = finish_receipt(
            &transaction,
            token,
            SubmissionAcceptedKind::Prompt { disposition },
        )
        .unwrap();
        transaction.commit().unwrap();
    }
    let before = usage(&db, None).unwrap();
    db.pragma_update(None, "query_only", true).unwrap();
    assert!(
        matches!(observe_input(&db, &queued, &Creator::Owner).unwrap(), ObservePromptReceiptReply::Accepted { binding: PromptReceiptBinding::Queued { entry: actual }, .. } if actual == entry)
    );
    assert!(matches!(
        observe_input(&db, &direct, &Creator::Owner).unwrap(),
        ObservePromptReceiptReply::Accepted {
            binding: PromptReceiptBinding::Unobserved,
            ..
        }
    ));
    assert_eq!(usage(&db, None).unwrap(), before);
}
