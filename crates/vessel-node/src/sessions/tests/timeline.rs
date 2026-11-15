//! Private fake providers exercise admission and exact foreground answer dispatch.
use super::*;
use std::time::Duration;

use vessel_proto::conversation::{AnswerValue, QuestionAnswer};
use vessel_proto::timeline::{
    ClientMessageId, RespondQuestion, RespondQuestionReply, SubmitPrompt, SubmitPromptReply,
};
use vessel_proto::{AgentEvent, ContentBlock, Creator, EventSeq, Question, SessionEvent};

const TIMELINE_RUNTIME_BOUND: Duration = Duration::from_secs(120);

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
        native_command: None,
        session,
        message: ClientMessageId::random(),
        content: vec![ContentBlock::Text { text: text.into() }],
        delivery: vessel_proto::timeline::SubmissionDelivery::Start,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_identified_prompts_dispatch_once_and_replay_after_the_process_stops() {
    vessel_testkit::within(TIMELINE_RUNTIME_BOUND, async {
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
    })
    .await
    .unwrap();
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
    vessel_testkit::within(TIMELINE_RUNTIME_BOUND, async {
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
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_question_answers_steer_the_captured_turn_without_starting_another() {
    vessel_testkit::within(TIMELINE_RUNTIME_BOUND, async {
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
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_lost_provider_answer_ack_stays_uncertain_after_the_process_ends() {
    vessel_testkit::within(TIMELINE_RUNTIME_BOUND, async {
        let fixture = Fixture::new().await;
        let id = create(&fixture).await;
        let question = offered(&fixture, id, "async-noack").await;
        let request = RespondQuestion {
            session: id,
            question: question.id,
            answer: answer(&question),
        };
        let reply = fixture
            .sessions
            .respond_question(request.clone())
            .await
            .unwrap();
        assert!(
            matches!(reply, RespondQuestionReply::OutcomeUnknown { .. }),
            "{reply:?}"
        );
        fixture.settle().await;
        let retained = fixture.store.pending_questions(id).await.unwrap();
        assert_eq!(retained.len(), 1);
        assert_eq!(
            retained[0].delivery_state,
            vessel_proto::timeline::QuestionDeliveryState::OutcomeUnknown
        );
        assert!(matches!(
            fixture.sessions.respond_question(request).await.unwrap(),
            RespondQuestionReply::OutcomeUnknown { .. }
        ));
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
        assert!(!records.iter().any(|record| matches!(
            record.event,
            SessionEvent::Agent {
                event: AgentEvent::QuestionResolved { .. }
            }
        )));
        fixture.stop().await;
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accepted_queued_receipts_have_a_committed_queue_and_keep_the_exact_entry_on_replay() {
    vessel_testkit::within(TIMELINE_RUNTIME_BOUND, async {
        let fixture = Fixture::new().await;
        let id = create(&fixture).await;
        let first = fixture
            .sessions
            .submit_prompt(prompt(id, "wait"), Creator::Owner)
            .await
            .unwrap();
        assert!(matches!(first, SubmitPromptReply::Accepted { .. }));
        let _turn = until(&fixture, id, |event| {
            matches!(event, AgentEvent::TurnStarted { .. })
        })
        .await;
        let mut queued = prompt(id, "queued follow-up");
        queued.delivery = vessel_proto::timeline::SubmissionDelivery::Queue;
        let accepted = fixture
            .sessions
            .submit_prompt(queued.clone(), Creator::Owner)
            .await
            .unwrap();
        let SubmitPromptReply::Accepted { receipt } = &accepted else {
            panic!("accepted queue")
        };
        let vessel_proto::timeline::SubmissionAcceptedKind::Prompt {
            disposition: vessel_proto::timeline::PromptDisposition::Queued { entry },
        } = receipt.accepted
        else {
            panic!("queued")
        };
        let committed = fixture.store.conversation(id).await.unwrap().unwrap();
        assert_eq!(committed.queue.entries.len(), 1);
        assert_eq!(committed.queue.entries[0].id, entry);
        assert_eq!(
            committed.queue.entries[0].client_message_id,
            Some(queued.message.clone())
        );
        assert_eq!(
            fixture
                .sessions
                .submit_prompt(queued, Creator::Owner)
                .await
                .unwrap(),
            accepted
        );
        fixture.stop().await;
    })
    .await
    .unwrap();
}

/// The future trusted plugin broker uses the actor and the existing durable log;
/// no helper runtime/grant parser is activated by this test-only producer.
#[tokio::test]
async fn plugin_observation_uses_the_owning_actor_and_existing_persistence_barrier() {
    vessel_testkit::within(TIMELINE_RUNTIME_BOUND, async {
        use vessel_proto::timeline::{
            PluginTimelineData, PluginTimelineItem, PluginTimelineVersion,
        };
        let fixture = Fixture::new().await;
        let session = create(&fixture).await;
        let Found::Live {
            handle,
            mut durability,
        } = fixture.sessions.find(session).await.unwrap()
        else {
            panic!("live fake")
        };
        let item = PluginTimelineItem {
            id: "fixture-item".parse().unwrap(),
            plugin: "fixture-plugin".parse().unwrap(),
            kind: "fixture-view".parse().unwrap(),
            version: PluginTimelineVersion::new(1).unwrap(),
            data: PluginTimelineData::try_from(
                vessel_proto::RawJson::from_text(r#"{"text":"opaque fixture"}"#).unwrap(),
            )
            .unwrap(),
        };
        let seq = handle.append_plugin(item.clone()).await.unwrap();
        crate::persist::wait_until(session, &mut durability, seq.next().unwrap())
            .await
            .unwrap();
        let page = fixture.store.read_events(session, seq, 1).await.unwrap();
        assert_eq!(page.records.len(), 1);
        assert_eq!(page.records[0].seq, seq);
        assert_eq!(
            page.records[0].event,
            SessionEvent::Agent {
                event: AgentEvent::Plugin(item)
            }
        );
        assert!(!handle.turn_running().await.unwrap());
        handle.kill();
        fixture.settle().await;
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retained_receipts_replay_before_loading_old_attachment_metadata() {
    vessel_testkit::within(TIMELINE_RUNTIME_BOUND, async {
        let fixture = Fixture::new().await;
        let session = create(&fixture).await;
        let mut request = prompt(session, "historical attachment");
        // Model a retained receipt after the old attachment metadata was removed.
        // The trusted store seed is deliberately independent of provider execution.
        request.content.push(ContentBlock::Attachment {
            attachment: vessel_proto::AttachmentRef {
                id: vessel_proto::AttachmentId::random(),
                name: "historical.txt".parse().unwrap(),
                mime: "text/plain".parse().unwrap(),
                kind: vessel_proto::AttachmentKind::File,
                size: 1,
            },
        });
        let token = reserve_fixture_receipt(&fixture, &request).await;
        let receipt = fixture
            .store
            .accept_prompt(
                token,
                vessel_proto::timeline::SubmissionAcceptedKind::Prompt {
                    disposition: vessel_proto::timeline::PromptDisposition::Started,
                },
            )
            .await
            .unwrap();
        assert_eq!(
            fixture
                .sessions
                .submit_prompt(request.clone(), Creator::Owner)
                .await
                .unwrap(),
            SubmitPromptReply::Accepted { receipt }
        );
        let mut conflict = request.clone();
        conflict.content.push(ContentBlock::Text {
            text: "changed".into(),
        });
        assert!(matches!(
            fixture
                .sessions
                .submit_prompt(conflict, Creator::Owner)
                .await
                .unwrap(),
            SubmitPromptReply::KeyConflict { .. }
        ));
        request.message = ClientMessageId::random();
        let token = reserve_fixture_receipt(&fixture, &request).await;
        drop(token);
        assert!(matches!(
            fixture
                .sessions
                .submit_prompt(request.clone(), Creator::Owner)
                .await
                .unwrap(),
            SubmitPromptReply::OutcomeUnknown { .. }
        ));
        request.message = ClientMessageId::random();
        assert!(
            fixture
                .sessions
                .submit_prompt(request, Creator::Owner)
                .await
                .is_err()
        );
        let page = fixture
            .sessions
            .read(session, EventSeq::ZERO, 128)
            .await
            .unwrap();
        assert!(!page.records.iter().any(|record| matches!(
            record.event,
            SessionEvent::Agent {
                event: AgentEvent::TurnStarted { .. }
            }
        )));
        fixture.sessions.live(session).unwrap().kill();
        fixture.settle().await;
    })
    .await
    .unwrap();
}

async fn reserve_fixture_receipt(
    fixture: &Fixture,
    request: &SubmitPrompt,
) -> vessel_store::ReceiptToken {
    let vessel_store::ReceiptAdmission::New(token) = fixture
        .store
        .admit_prompt(request.clone(), Creator::Owner)
        .await
        .unwrap()
    else {
        panic!("fresh fixture receipt")
    };
    token
}

mod receipt_observation;
