//! Durable state tests do not claim provider steering or native UI evidence.

use std::collections::BTreeMap;

use vessel_proto::conversation::{AnswerValue, FieldKind, QuestionField};

use super::*;
use crate::timeline_admission::{
    tests::{database, session},
    usage, verify,
};

fn form() -> Question {
    Question {
        id: QuestionId::random(),
        message: "Choose a route".to_owned(),
        tool_call: None,
        fields: vec![QuestionField {
            name: "route".parse().unwrap(),
            title: Some("Question 1".to_owned()),
            description: None,
            required: true,
            kind: FieldKind::Text {
                min_length: Some(1),
                max_length: None,
                format: None,
                pattern: None,
                default: None,
            },

            presentation: None,
        }],

        lifetime: vessel_proto::timeline::QuestionLifetime::Session,
        delivery_state: Default::default(),
    }
}

fn answer(text: &str) -> QuestionAnswer {
    QuestionAnswer::Accept {
        values: BTreeMap::from([("route".parse().unwrap(), AnswerValue::Text(text.to_owned()))]),
    }
}

#[test]
fn replay_keeps_the_original_request_id_and_resolution_is_independent_of_a_tool_end() {
    let mut db = database();
    let session = session(&db);
    let question = form();
    let source = ToolCallId::new("async-item");
    let transaction = db.transaction().unwrap();
    assert_eq!(
        admit_question(&transaction, session, source.clone(), question.clone()).unwrap(),
        QuestionAdmission::New(question.clone())
    );
    transaction.commit().unwrap();
    let mut replay = question.clone();
    replay.id = QuestionId::random();
    let transaction = db.transaction().unwrap();
    assert_eq!(
        admit_question(&transaction, session, source.clone(), replay).unwrap(),
        QuestionAdmission::Existing(question.clone())
    );
    transaction.commit().unwrap();
    // Completing or pruning a tool's ordinary history never deletes its question row.
    db.execute(
        "DELETE FROM events WHERE session_id=?1",
        [session.to_string()],
    )
    .unwrap();
    assert!(matches!(
        load(&db, session, &source).unwrap().unwrap().state,
        QuestionState::Pending
    ));
    let transaction = db.transaction().unwrap();
    let AnswerAdmission::New(token) = begin_answer(
        &transaction,
        session,
        question.id,
        answer("left"),
        AnswerTarget::Idle,
    )
    .unwrap() else {
        panic!("fresh answer")
    };
    transaction.commit().unwrap();
    let transaction = db.transaction().unwrap();
    let outcome = finish_answer(&transaction, token).unwrap();
    assert!(matches!(outcome, QuestionOutcome::Answered { .. }));
    transaction.commit().unwrap();
    let transaction = db.transaction().unwrap();
    assert!(matches!(
        begin_answer(
            &transaction,
            session,
            question.id,
            answer("right"),
            AnswerTarget::Idle
        )
        .unwrap(),
        AnswerAdmission::Settled
    ));
    assert_eq!(
        admit_question(&transaction, session, source, question).unwrap(),
        QuestionAdmission::Settled
    );
    transaction.commit().unwrap();
    verify(&db).unwrap();
}

#[test]
fn invalid_answer_does_not_claim_and_known_not_sent_restores_the_exact_pending_request() {
    let mut db = database();
    let session = session(&db);
    let question = form();
    let transaction = db.transaction().unwrap();
    admit_question(
        &transaction,
        session,
        ToolCallId::new("source"),
        question.clone(),
    )
    .unwrap();
    let invalid = QuestionAnswer::Accept {
        values: BTreeMap::new(),
    };
    let AnswerAdmission::Invalid(error) = begin_answer(
        &transaction,
        session,
        question.id,
        invalid,
        AnswerTarget::Idle,
    )
    .unwrap() else {
        panic!("invalid")
    };
    assert!(matches!(
        error,
        vessel_proto::conversation::InvalidAnswer::Missing(_)
    ));
    let target = AnswerTarget::Turn {
        started: EventSeq::new(4),
    };
    let AnswerAdmission::New(token) =
        begin_answer(&transaction, session, question.id, answer("left"), target).unwrap()
    else {
        panic!("fresh")
    };
    assert_eq!(
        token.record.state,
        QuestionState::Answering {
            answer: answer("left"),
            target
        }
    );
    assert!(matches!(
        begin_answer(&transaction, session, question.id, answer("right"), target).unwrap(),
        AnswerAdmission::AlreadyClaimed(QuestionDeliveryState::Answering)
    ));
    reject_before_effect(&transaction, token, AnswerNotSent::TurnChanged).unwrap();
    assert!(matches!(
        begin_answer(&transaction, session, question.id, answer("retry"), target).unwrap(),
        AnswerAdmission::New(_)
    ));
    transaction.commit().unwrap();
}

#[test]
fn restart_marks_admitted_unacknowledged_answers_uncertain_and_keeps_their_reservation() {
    let mut db = database();
    let session = session(&db);
    let question = form();
    let source = ToolCallId::new("source");
    let transaction = db.transaction().unwrap();
    admit_question(&transaction, session, source.clone(), question.clone()).unwrap();
    assert!(matches!(
        begin_answer(
            &transaction,
            session,
            question.id,
            answer("left"),
            AnswerTarget::Idle
        )
        .unwrap(),
        AnswerAdmission::New(_)
    ));
    transaction.commit().unwrap();
    let reserved = usage(&db, Some(session)).unwrap();
    let transaction = db.transaction().unwrap();
    recover_claims(&transaction).unwrap();
    transaction.commit().unwrap();
    assert_eq!(usage(&db, Some(session)).unwrap(), reserved);
    let transaction = db.transaction().unwrap();
    assert!(matches!(
        begin_answer(
            &transaction,
            session,
            question.id,
            answer("again"),
            AnswerTarget::Idle
        )
        .unwrap(),
        AnswerAdmission::AlreadyClaimed(QuestionDeliveryState::OutcomeUnknown)
    ));
    assert_eq!(
        admit_question(&transaction, session, source, question.clone()).unwrap(),
        QuestionAdmission::Existing({
            let mut question = question;
            question.delivery_state = QuestionDeliveryState::OutcomeUnknown;
            question
        })
    );
    transaction.commit().unwrap();
    verify(&db).unwrap();
}

#[test]
fn dismissal_retains_source_tombstone_and_changed_source_content_conflicts() {
    let mut db = database();
    let session = session(&db);
    let question = form();
    let source = ToolCallId::new("source");
    let transaction = db.transaction().unwrap();
    admit_question(&transaction, session, source.clone(), question.clone()).unwrap();
    let mut changed = question.clone();
    changed.message = "Different".to_owned();
    assert_eq!(
        admit_question(&transaction, session, source.clone(), changed).unwrap(),
        QuestionAdmission::Conflict
    );
    let AnswerAdmission::New(token) = begin_answer(
        &transaction,
        session,
        question.id,
        QuestionAnswer::Decline,
        AnswerTarget::Idle,
    )
    .unwrap() else {
        panic!("fresh")
    };
    assert_eq!(
        finish_answer(&transaction, token).unwrap(),
        QuestionOutcome::Declined
    );
    transaction.commit().unwrap();
    assert_eq!(usage(&db, Some(session)).unwrap().records, 1);
    assert!(matches!(
        load(&db, session, &source).unwrap().unwrap().state,
        QuestionState::Settled { .. }
    ));
}

#[test]
fn unsupported_and_pre_send_refusals_never_settle_the_pending_request() {
    for reason in [
        AnswerNotSent::Unsupported,
        AnswerNotSent::RejectedBeforeSend,
    ] {
        let mut db = database();
        let id = session(&db);
        let question = form();
        let transaction = db.transaction().unwrap();
        admit_question(
            &transaction,
            id,
            ToolCallId::new("source"),
            question.clone(),
        )
        .unwrap();
        let AnswerAdmission::New(token) = begin_answer(
            &transaction,
            id,
            question.id,
            answer("left"),
            AnswerTarget::Idle,
        )
        .unwrap() else {
            panic!("fresh")
        };
        reject_before_effect(&transaction, token, reason).unwrap();
        assert!(matches!(
            begin_answer(
                &transaction,
                id,
                question.id,
                answer("retry"),
                AnswerTarget::Idle
            )
            .unwrap(),
            AnswerAdmission::New(_)
        ));
        transaction.commit().unwrap();
    }
}
