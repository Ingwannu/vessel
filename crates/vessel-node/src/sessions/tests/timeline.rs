//! Private fake providers exercise admission and exact foreground answer dispatch.
use super::*;
use std::time::Duration;
use vessel_proto::conversation::{AnswerValue, QuestionAnswer};
use vessel_proto::timeline::{
    ClientMessageId, RespondQuestion, RespondQuestionReply, SubmitPrompt, SubmitPromptReply,
};
use vessel_proto::{AgentEvent, ContentBlock, Creator, Delivery, EventSeq, Question, SessionEvent};

async fn create(fixture: &Fixture) -> SessionId {
    let mut spec = fixture.structured();
    spec.harness = HarnessKind::Codex.into();
    let id = fixture
        .sessions
        .create(
            spec,
            LaunchOptions {
                questions: true,
                ..LaunchOptions::default()
            },
            None,
        )
        .await
        .unwrap()
        .id;
    let Found::Live { handle, .. } = fixture.sessions.find(id).await.unwrap() else {
        panic!("live fake")
    };
    let _running = ready(&handle).await.unwrap();
    id
}

async fn until(
    fixture: &Fixture,
    session: SessionId,
    wanted: impl Fn(&AgentEvent) -> bool,
) -> AgentEvent {
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut cursor = EventSeq::ZERO;
        loop {
            let page = fixture
                .sessions
                .read_waiting(session, cursor, 128, Duration::from_secs(1))
                .await
                .unwrap();
            for record in page.records {
                if let SessionEvent::Agent { event } = record.event {
                    if wanted(&event) {
                        return event;
                    }
                }
            }
            cursor = page.next;
        }
    })
    .await
    .unwrap()
}

fn prompt(session: SessionId, text: &str) -> SubmitPrompt {
    SubmitPrompt {
        session,
        message: ClientMessageId::random(),
        content: vec![ContentBlock::Text { text: text.into() }],
        delivery: Delivery::Start,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_identified_prompts_dispatch_once_and_replay_after_the_process_stops() {
    let fixture = Fixture::new().await;
    let id = create(&fixture).await;
    let request = prompt(id, "wait");
    let (first, second) = tokio::join!(
        fixture
            .sessions
            .submit_prompt(request.clone(), Creator::Owner),
        fixture
            .sessions
            .submit_prompt(request.clone(), Creator::Owner)
    );
    let first = first.unwrap();
    let second = second.unwrap();
    let accepted = if matches!(first, SubmitPromptReply::Accepted { .. }) {
        first
    } else {
        second.clone()
    };
    assert!(matches!(accepted, SubmitPromptReply::Accepted { .. }));
    assert!(matches!(
        second,
        SubmitPromptReply::Accepted { .. } | SubmitPromptReply::OutcomeUnknown { .. }
    ));
    let _turn = until(&fixture, id, |event| {
        matches!(event, AgentEvent::TurnStarted { .. })
    })
    .await;
    assert_eq!(
        fixture
            .sessions
            .submit_prompt(request.clone(), Creator::Owner)
            .await
            .unwrap(),
        accepted
    );
    let mut conflict = request.clone();
    conflict.content = vec![ContentBlock::Text {
        text: "different".into(),
    }];
    assert!(matches!(
        fixture
            .sessions
            .submit_prompt(conflict, Creator::Owner)
            .await
            .unwrap(),
        SubmitPromptReply::KeyConflict { .. }
    ));
    let page = fixture
        .sessions
        .read(id, EventSeq::ZERO, 128)
        .await
        .unwrap();
    assert_eq!(
        page.records
            .iter()
            .filter(|record| matches!(
                record.event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnStarted { .. }
                }
            ))
            .count(),
        1
    );
    let Found::Live { handle, .. } = fixture.sessions.find(id).await.unwrap() else {
        panic!("live")
    };
    handle.kill();
    fixture.settle().await;
    assert_eq!(
        fixture
            .sessions
            .submit_prompt(request, Creator::Owner)
            .await
            .unwrap(),
        accepted
    );
    fixture.stop().await;
}

async fn offered(fixture: &Fixture, session: SessionId, scenario: &str) -> Question {
    let result = fixture
        .sessions
        .submit_prompt(prompt(session, scenario), Creator::Owner)
        .await
        .unwrap();
    assert!(matches!(result, SubmitPromptReply::Accepted { .. }));
    let AgentEvent::QuestionAsked(question) = until(fixture, session, |event| {
        matches!(event, AgentEvent::QuestionAsked(_))
    })
    .await
    else {
        panic!("question")
    };
    question
}

fn answer(question: &Question) -> QuestionAnswer {
    QuestionAnswer::Accept {
        values: [(
            question.fields[0].name.clone(),
            AnswerValue::Choice("left".into()),
        )]
        .into(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_questions_survive_turn_end_and_idle_answers_start_one_followup() {
    let fixture = Fixture::new().await;
    let id = create(&fixture).await;
    let question = offered(&fixture, id, "async-duplicate").await;
    let _ended = until(&fixture, id, |event| {
        matches!(event, AgentEvent::TurnEnded { .. })
    })
    .await;
    let before = fixture.sessions.conversation(id).await.unwrap();
    assert_eq!(before.questions.len(), 1);
    assert_eq!(before.questions[0].id, question.id);
    let reply = fixture
        .sessions
        .respond_question(RespondQuestion {
            session: id,
            question: question.id,
            answer: answer(&question),
        })
        .await
        .unwrap();
    let RespondQuestionReply::Settled { seq, .. } = reply else {
        panic!("durable settlement: {reply:?}")
    };
    let after = fixture.sessions.conversation(id).await.unwrap();
    assert!(after.questions.is_empty());
    assert!(after.through.is_some_and(|through| through > seq));
    assert!(
        fixture
            .store
            .pending_questions(id)
            .await
            .unwrap()
            .is_empty()
    );
    let replay = fixture
        .sessions
        .respond_question(RespondQuestion {
            session: id,
            question: question.id,
            answer: answer(&question),
        })
        .await
        .unwrap();
    assert!(matches!(replay, RespondQuestionReply::Conflict { .. }));
    let records = fixture
        .sessions
        .read(id, EventSeq::ZERO, 128)
        .await
        .unwrap()
        .records;
    assert_eq!(
        records
            .iter()
            .filter(|record| matches!(
                record.event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnStarted { .. }
                }
            ))
            .count(),
        2
    );
    fixture.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_question_answers_steer_the_captured_turn_without_starting_another() {
    let fixture = Fixture::new().await;
    let id = create(&fixture).await;
    let question = offered(&fixture, id, "async-wait").await;
    let reply = fixture
        .sessions
        .respond_question(RespondQuestion {
            session: id,
            question: question.id,
            answer: answer(&question),
        })
        .await
        .unwrap();
    assert!(
        matches!(reply, RespondQuestionReply::Settled { .. }),
        "{reply:?}"
    );
    let _echo = until(&fixture, id, |event| {
        matches!(event, AgentEvent::Steered { .. })
    })
    .await;
    let records = fixture
        .sessions
        .read(id, EventSeq::ZERO, 128)
        .await
        .unwrap()
        .records;
    assert_eq!(
        records
            .iter()
            .filter(|record| matches!(
                record.event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnStarted { .. }
                }
            ))
            .count(),
        1
    );
    fixture.stop().await;
}
