//! Commit acknowledgements and lost-answer recovery through the real store writer.

use fixtures::{data_dir, new_session, open};
use std::collections::BTreeMap;
use vessel_proto::conversation::{AnswerValue, FieldKind, QuestionField};
use vessel_proto::timeline::{QuestionDeliveryState, QuestionLifetime};
use vessel_proto::{Question, QuestionAnswer, QuestionId, ToolCallId};
use vessel_store::{
    QuestionAdmission, QuestionAnswerAdmission, QuestionAnswerNotSent, QuestionAnswerTarget,
    StoreConfig,
};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(dead_code, reason = "only session and directory fixtures are needed")]
mod fixtures;

#[cfg(test)]
fn form() -> Question {
    Question {
        tool_scope: None,
        id: QuestionId::random(),
        message: "Which route?".into(),
        tool_call: Some(ToolCallId::new("async-item")),
        lifetime: QuestionLifetime::Session,
        delivery_state: QuestionDeliveryState::Pending,
        fields: vec![QuestionField {
            name: "route".parse().unwrap(),
            title: None,
            description: None,
            required: true,
            presentation: None,
            kind: FieldKind::Text {
                min_length: Some(1),
                max_length: None,
                format: None,
                pattern: None,
                default: None,
            },
        }],
    }
}
#[cfg(test)]
fn answer() -> QuestionAnswer {
    QuestionAnswer::Accept {
        values: BTreeMap::from([("route".parse().unwrap(), AnswerValue::Text("left".into()))]),
    }
}

#[tokio::test]
async fn admitted_answer_is_non_actionable_after_lost_ack_and_restart() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let info = new_session(0);
    store.create_session(info.clone()).await.unwrap();
    let question = form();
    assert!(matches!(
        store
            .offer_question(info.id, ToolCallId::new("async-item"), question.clone())
            .await
            .unwrap(),
        QuestionAdmission::New(_)
    ));
    let QuestionAnswerAdmission::New(token) = store
        .begin_question_answer(info.id, question.id, answer(), QuestionAnswerTarget::Idle)
        .await
        .unwrap()
    else {
        panic!("fresh claim");
    };
    assert_eq!(
        store.pending_questions(info.id).await.unwrap()[0].delivery_state,
        QuestionDeliveryState::Answering
    );
    assert!(matches!(
        store
            .begin_question_answer(info.id, question.id, answer(), QuestionAnswerTarget::Idle)
            .await
            .unwrap(),
        QuestionAnswerAdmission::AlreadyClaimed(QuestionDeliveryState::Answering)
    ));
    drop(token);
    store.close().await;
    drop(store);
    let store = open(&dir, StoreConfig::default()).await;
    assert_eq!(
        store.pending_questions(info.id).await.unwrap()[0].delivery_state,
        QuestionDeliveryState::OutcomeUnknown
    );
    assert!(matches!(
        store
            .begin_question_answer(info.id, question.id, answer(), QuestionAnswerTarget::Idle)
            .await
            .unwrap(),
        QuestionAnswerAdmission::AlreadyClaimed(QuestionDeliveryState::OutcomeUnknown)
    ));
    let mut replay = question;
    replay.id = QuestionId::random();
    assert!(
        matches!(store.offer_question(info.id, ToolCallId::new("async-item"), replay).await.unwrap(),QuestionAdmission::Existing(kept) if kept.delivery_state == QuestionDeliveryState::OutcomeUnknown)
    );
    store.close().await;
}

#[tokio::test]
async fn only_known_not_sent_restores_pending_and_settlement_keeps_a_tombstone() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let info = new_session(0);
    store.create_session(info.clone()).await.unwrap();
    let question = form();
    let _admitted = store
        .offer_question(info.id, ToolCallId::new("async-item"), question.clone())
        .await
        .unwrap();
    let QuestionAnswerAdmission::New(token) = store
        .begin_question_answer(info.id, question.id, answer(), QuestionAnswerTarget::Idle)
        .await
        .unwrap()
    else {
        panic!("fresh claim");
    };
    store
        .refuse_question_answer(token, QuestionAnswerNotSent::TurnChanged)
        .await
        .unwrap();
    assert_eq!(
        store.pending_questions(info.id).await.unwrap()[0].delivery_state,
        QuestionDeliveryState::Pending
    );
    let QuestionAnswerAdmission::New(token) = store
        .begin_question_answer(info.id, question.id, answer(), QuestionAnswerTarget::Idle)
        .await
        .unwrap()
    else {
        panic!("fresh retry after no effect");
    };
    let outcome = store.finish_question_answer(token).await.unwrap();
    assert!(matches!(
        outcome,
        vessel_proto::QuestionOutcome::Answered { .. }
    ));
    assert!(store.pending_questions(info.id).await.unwrap().is_empty());
    assert!(matches!(
        store
            .offer_question(info.id, ToolCallId::new("async-item"), question.clone())
            .await
            .unwrap(),
        QuestionAdmission::Settled
    ));
    assert!(matches!(
        store
            .begin_question_answer(info.id, question.id, answer(), QuestionAnswerTarget::Idle)
            .await
            .unwrap(),
        QuestionAnswerAdmission::Settled
    ));
    store.close().await;
}
