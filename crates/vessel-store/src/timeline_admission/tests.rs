//! Replay and reservation semantics tested on the real transactional storage edge.

use std::sync::{Arc, Barrier};

use rusqlite::{Connection, TransactionBehavior};
use vessel_proto::timeline::{SubmitPrompt, SubmitPromptReply};
use vessel_proto::{
    ContentBlock, Creator, HarnessKind, RawJson, SessionInfo, SessionMode, SessionSpec,
};

use super::*;

pub(super) fn database() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    crate::schema::configure(&db).unwrap();
    crate::schema::migrate(&mut db).unwrap();
    db
}

pub(super) fn session(db: &Connection) -> SessionId {
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
        chrono::DateTime::UNIX_EPOCH,
    );
    crate::log::create_session(db, &info).unwrap();
    info.id
}

pub(super) fn prompt(session: SessionId, id: &str, text: &str) -> SubmitPrompt {
    SubmitPrompt {
        native_command: None,
        session,
        message: id.parse().unwrap(),
        content: vec![ContentBlock::Text {
            text: text.to_owned(),
        }],
        delivery: vessel_proto::timeline::SubmissionDelivery::Start,
    }
}

pub(super) fn admit(db: &mut Connection, prompt: &SubmitPrompt) -> ReceiptAdmission {
    let transaction = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    let fingerprint = ReceiptFingerprint::for_prompt(prompt, &Creator::Owner).unwrap();
    let admission = admit_receipt(
        &transaction,
        prompt.session,
        prompt.message.clone(),
        fingerprint,
    )
    .unwrap();
    transaction.commit().unwrap();
    admission
}

#[test]
fn a_pending_retry_is_uncertain_and_an_accepted_retry_keeps_the_exact_outcome() {
    let mut db = database();
    let session = session(&db);
    let prompt = prompt(session, "message", "run");
    let ReceiptAdmission::New(token) = admit(&mut db, &prompt) else {
        panic!("fresh admission required")
    };
    assert!(matches!(
        admit(&mut db, &prompt),
        ReceiptAdmission::Existing(SubmitPromptReply::OutcomeUnknown { .. })
    ));
    let delivered = vessel_proto::timeline::SubmissionAcceptedKind::Prompt {
        disposition: vessel_proto::timeline::PromptDisposition::Queued {
            entry: vessel_proto::QueueEntryId::random(),
        },
    };
    let transaction = db.transaction().unwrap();
    let receipt = finish_receipt(&transaction, token, delivered).unwrap();
    transaction.commit().unwrap();
    assert_eq!(
        admit(&mut db, &prompt).existing(),
        Some(SubmitPromptReply::Accepted { receipt })
    );
    let changed = self::prompt(session, "message", "different");
    assert!(matches!(
        admit(&mut db, &changed),
        ReceiptAdmission::Existing(SubmitPromptReply::KeyConflict { .. })
    ));
}

impl ReceiptAdmission {
    fn existing(self) -> Option<SubmitPromptReply> {
        match self {
            Self::Existing(reply) => Some(reply),
            Self::New(_) => None,
        }
    }
}

#[test]
fn fingerprints_sort_opaque_object_keys_and_include_authenticated_creator_and_delivery() {
    let id = SessionId::random();
    let mut first = prompt(id, "same", "run");
    first.content = vec![ContentBlock::Opaque {
        raw: RawJson::from_text(r#"{"b":{"z":1,"a":2},"a":3}"#).unwrap(),
    }];
    let mut second = first.clone();
    second.content = vec![ContentBlock::Opaque {
        raw: RawJson::from_text(r#"{"a":3,"b":{"a":2,"z":1}}"#).unwrap(),
    }];
    let fingerprint = ReceiptFingerprint::for_prompt(&first, &Creator::Owner).unwrap();
    assert_eq!(
        fingerprint,
        ReceiptFingerprint::for_prompt(&second, &Creator::Owner).unwrap()
    );
    second.delivery = vessel_proto::timeline::SubmissionDelivery::Queue;
    assert_ne!(
        fingerprint,
        ReceiptFingerprint::for_prompt(&second, &Creator::Owner).unwrap()
    );
    let agent = Creator::Agent {
        node: vessel_proto::NodeId::random(),
        session: id,
    };
    assert_ne!(
        fingerprint,
        ReceiptFingerprint::for_prompt(&first, &agent).unwrap()
    );
    assert!(!format!("{fingerprint:?}").contains("run"));
}

#[test]
fn saturation_keeps_all_replay_keys_and_releases_only_with_session_removal() {
    let mut db = database();
    let session = session(&db);
    let first = prompt(session, "first", "run");
    assert!(matches!(admit(&mut db, &first), ReceiptAdmission::New(_)));
    let accepted = prompt(session, "accepted", "queued");
    let ReceiptAdmission::New(accepted_token) = admit(&mut db, &accepted) else {
        panic!("fresh admission")
    };
    let transaction = db.transaction().unwrap();
    let accepted_receipt = finish_receipt(
        &transaction,
        accepted_token,
        vessel_proto::timeline::SubmissionAcceptedKind::Prompt {
            disposition: vessel_proto::timeline::PromptDisposition::Started,
        },
    )
    .unwrap();
    transaction.commit().unwrap();
    let mut admitted = 2_u64;
    loop {
        let next = prompt(session, &format!("message-{admitted}"), "run");
        match admit(&mut db, &next) {
            ReceiptAdmission::New(_) => admitted = admitted.checked_add(1).unwrap(),
            ReceiptAdmission::Existing(SubmitPromptReply::Capacity { capacity, .. }) => {
                assert_eq!(
                    capacity,
                    AdmissionCapacity {
                        scope: AdmissionScope::Session,
                        resource: AdmissionResource::Bytes
                    }
                );
                break;
            }
            ReceiptAdmission::Existing(reply) => panic!("unexpected admission {reply:?}"),
        }
    }
    let at_capacity = usage(&db, Some(session)).unwrap();
    assert_eq!(
        admit(&mut db, &accepted).existing(),
        Some(SubmitPromptReply::Accepted {
            receipt: accepted_receipt
        })
    );
    assert_eq!(at_capacity.records, admitted);
    assert!(matches!(
        admit(&mut db, &first),
        ReceiptAdmission::Existing(SubmitPromptReply::OutcomeUnknown { .. })
    ));
    assert_eq!(usage(&db, Some(session)).unwrap(), at_capacity);
    // Log retention is independent of retained replay identities.
    let _setup = db
        .execute(
            "DELETE FROM events WHERE session_id=?1",
            [session.to_string()],
        )
        .unwrap();
    assert_eq!(usage(&db, Some(session)).unwrap(), at_capacity);
    let _setup = db
        .execute("DELETE FROM sessions WHERE id=?1", [session.to_string()])
        .unwrap();
    assert_eq!(
        usage(&db, Some(session)).unwrap(),
        Usage {
            records: 0,
            bytes: 0
        }
    );
    verify(&db).unwrap();
}

#[test]
fn every_count_and_byte_guard_refuses_at_its_boundary_without_reducing_reservations() {
    for scope in [AdmissionScope::Session, AdmissionScope::Node] {
        let (records, bytes) = match scope {
            AdmissionScope::Session => (MAX_SESSION_ADMISSIONS, MAX_SESSION_ADMISSION_BYTES),
            AdmissionScope::Node => (MAX_NODE_ADMISSIONS, MAX_NODE_ADMISSION_BYTES),
        };
        assert_eq!(
            capacity(Usage { records, bytes: 0 }, AdmissionKind::Receipt, scope)
                .unwrap_err()
                .resource,
            AdmissionResource::Records
        );
        for kind in [AdmissionKind::Receipt, AdmissionKind::Question] {
            let fits = bytes.checked_sub(kind.reserved_bytes().get()).unwrap();
            assert!(
                capacity(
                    Usage {
                        records: 0,
                        bytes: fits
                    },
                    kind,
                    scope
                )
                .is_ok()
            );
            assert_eq!(
                capacity(
                    Usage {
                        records: 0,
                        bytes: fits.checked_add(1).unwrap()
                    },
                    kind,
                    scope
                )
                .unwrap_err()
                .resource,
                AdmissionResource::Bytes
            );
        }
    }
}

#[test]
fn pending_state_and_full_reservations_survive_a_connection_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("admissions.sqlite");
    let mut db = Connection::open(&path).unwrap();
    crate::schema::configure(&db).unwrap();
    crate::schema::migrate(&mut db).unwrap();
    let id = session(&db);
    let prompt = prompt(id, "durable", "run");
    assert!(matches!(admit(&mut db, &prompt), ReceiptAdmission::New(_)));
    let before = usage(&db, None).unwrap();
    drop(db);
    let mut reopened = Connection::open(&path).unwrap();
    crate::schema::configure(&reopened).unwrap();
    verify(&reopened).unwrap();
    assert_eq!(usage(&reopened, None).unwrap(), before);
    assert!(matches!(
        admit(&mut reopened, &prompt),
        ReceiptAdmission::Existing(SubmitPromptReply::OutcomeUnknown { .. })
    ));
    let _setup = reopened
        .execute("UPDATE timeline_admissions SET reserved_bytes=1", [])
        .unwrap();
    assert!(matches!(
        verify(&reopened),
        Err(StoreError::AdmissionIntegrity)
    ));
}

#[test]
fn concurrent_same_identity_has_one_fresh_admission_and_one_uncertain_replay() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("concurrent.sqlite");
    let mut setup = Connection::open(&path).unwrap();
    crate::schema::configure(&setup).unwrap();
    crate::schema::migrate(&mut setup).unwrap();
    let id = session(&setup);
    let barrier = Arc::new(Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let mut db = Connection::open(path).unwrap();
                crate::schema::configure(&db).unwrap();
                let prompt = prompt(id, "same", "run");
                let _barrier = barrier.wait();
                matches!(admit(&mut db, &prompt), ReceiptAdmission::New(_))
            })
        })
        .collect();
    assert_eq!(
        workers
            .into_iter()
            .map(|worker| usize::from(worker.join().unwrap()))
            .sum::<usize>(),
        1
    );
    assert_eq!(usage(&setup, None).unwrap().records, 1);
}

#[test]
fn two_distinct_ids_cannot_overbook_the_final_session_reservation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("last-slot.sqlite");
    let mut setup = Connection::open(&path).unwrap();
    crate::schema::configure(&setup).unwrap();
    crate::schema::migrate(&mut setup).unwrap();
    let id = session(&setup);
    let available = MAX_SESSION_ADMISSION_BYTES
        .checked_div(AdmissionKind::Receipt.reserved_bytes().get())
        .unwrap();
    for index in 0..available.saturating_sub(1) {
        assert!(matches!(
            admit(&mut setup, &prompt(id, &format!("seed-{index}"), "run")),
            ReceiptAdmission::New(_)
        ));
    }
    let barrier = Arc::new(Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|index| {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let mut db = Connection::open(path).unwrap();
                crate::schema::configure(&db).unwrap();
                let _barrier = barrier.wait();
                admit(&mut db, &prompt(id, &format!("racer-{index}"), "run"))
            })
        })
        .collect();
    let outcomes: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, ReceiptAdmission::New(_)))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(
                outcome,
                ReceiptAdmission::Existing(SubmitPromptReply::Capacity { .. })
            ))
            .count(),
        1
    );
    assert_eq!(usage(&setup, Some(id)).unwrap().records, available);
    verify(&setup).unwrap();
}
