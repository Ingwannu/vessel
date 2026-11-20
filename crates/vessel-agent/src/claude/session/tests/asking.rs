//! What the agent asks a person, over the fake: tool requests with the
//! agent's own choices, a plan's approval, questions, and every way a
//! request is settled (answered, rejected, withdrawn by a cancel or by the
//! agent).

use std::collections::BTreeMap;

use serde_json::{Value, json};
use vessel_proto::conversation::{AnswerValue, FieldKind};
use vessel_proto::{
    PermissionId, PermissionOptionId, PermissionOptionKind, PermissionOutcome, PermissionRequest,
    Question, QuestionAnswer, QuestionId, QuestionOutcome,
};

use super::*;

impl Running {
    /// Events up to and including the first `stop` matches, failing on a
    /// turn's end before it.
    async fn until(&mut self, stop: impl Fn(&AgentEvent) -> bool) -> Vec<AgentEvent> {
        let mut events = Vec::new();
        loop {
            match self.next().await {
                AgentOutput::Event(event) => {
                    let found = stop(&event);
                    let ended = matches!(
                        event,
                        AgentEvent::TurnEnded { .. } | AgentEvent::TurnFailed { .. }
                    );
                    events.push(event);
                    if found {
                        return events;
                    }
                    assert!(!ended, "the turn ended first: {events:?}");
                }
                AgentOutput::Disconnected(end) => panic!("disconnected: {end:?}"),
                AgentOutput::Options(_)
                | AgentOutput::Commands(_)
                | AgentOutput::Usage(_)
                | AgentOutput::Introduced(_) => {}
            }
        }
    }

    /// The permission request a prompt of `text` leads to.
    async fn permission(&mut self, text: &str) -> (Vec<AgentEvent>, PermissionRequest) {
        self.prompt(super::text(text)).await.unwrap();
        let events = self
            .until(|event| matches!(event, AgentEvent::PermissionRequested(_)))
            .await;
        let Some(AgentEvent::PermissionRequested(request)) = events.last().cloned() else {
            panic!("{events:?}");
        };
        (events, request)
    }

    async fn answer(&self, id: PermissionId, outcome: PermissionOutcome) -> Result<(), InputError> {
        let (reply, answer) = oneshot::channel();
        self.commands
            .send(Command::Answer { id, outcome, reply })
            .await
            .unwrap();
        answer.await.unwrap()
    }

    async fn choose(&self, id: PermissionId, option: &str) -> Result<(), InputError> {
        let option = PermissionOptionId::new(option);
        self.answer(id, PermissionOutcome::Selected { option })
            .await
    }

    async fn reply(&self, id: QuestionId, answer: QuestionAnswer) -> Result<(), InputError> {
        let (reply, answered) = oneshot::channel();
        self.commands
            .send(Command::AnswerQuestion { id, answer, reply })
            .await
            .unwrap();
        answered.await.unwrap()
    }
}

fn asking() -> AgentSetup {
    AgentSetup {
        questions: true,
        ..setup()
    }
}

/// What the agent received, as the fake replies it: the answer's JSON.
fn received(events: &[AgentEvent]) -> Value {
    let text = said(events, false);
    serde_json::from_str(&text).unwrap_or_else(|_| panic!("not an answer: {text}"))
}

#[tokio::test]
async fn a_tool_request_offers_the_agents_own_choices_and_one_goes_back_as_given() {
    within(async {
        let mut session = start(setup(), &[]);
        let _ready = session.ready().await;
        let (events, request) = session.permission("tool").await;
        let Some(AgentEvent::ToolCall(call)) = events.first() else {
            panic!("{events:?}");
        };
        assert_eq!(request.tool_call, call.id);
        let offered: Vec<_> = request
            .options
            .iter()
            .map(|option| (option.id.as_str(), option.name.as_str(), option.kind))
            .collect();
        // The deny rule and the switch into bypass are not offered.
        assert_eq!(
            offered,
            [
                ("allow", "Allow", PermissionOptionKind::AllowOnce),
                (
                    "suggestion-0",
                    "Always allow Bash(ls) (this session)",
                    PermissionOptionKind::AllowAlways
                ),
                ("reject", "Reject", PermissionOptionKind::RejectOnce),
            ]
        );
        session.choose(request.id, "suggestion-0").await.unwrap();
        let events = session.turn().await;
        assert_eq!(
            events[0],
            AgentEvent::PermissionResolved {
                id: request.id,
                outcome: PermissionOutcome::Selected {
                    option: PermissionOptionId::new("suggestion-0"),
                },
            }
        );
        assert_eq!(
            received(&events),
            json!({
                "behavior": "allow",
                "updatedInput": {"command": "ls", "description": "List"},
                "updatedPermissions": [{"type": "addRules",
                    "rules": [{"toolName": "Bash", "ruleContent": "ls"}],
                    "behavior": "allow", "destination": "session"}],
                "toolUseID": call.id.as_str(),
            })
        );
        // Answered once: a second answer is told so.
        assert!(matches!(
            session.choose(request.id, "allow").await,
            Err(InputError::PermissionSettled(_))
        ));
    })
    .await;
}

#[tokio::test]
async fn a_rejection_denies_the_call_and_the_turn_goes_on() {
    within(async {
        let mut session = start(setup(), &[]);
        let _ready = session.ready().await;
        let (_, request) = session.permission("tool").await;
        // Nothing changes on a wrong answer.
        assert!(matches!(
            session
                .answer(PermissionId::random(), PermissionOutcome::Cancelled)
                .await,
            Err(InputError::PermissionNotFound(_))
        ));
        assert!(matches!(
            session.choose(request.id, "maybe").await,
            Err(InputError::UnknownOption { .. })
        ));
        session.choose(request.id, "reject").await.unwrap();
        let events = session.turn().await;
        let failed = events.iter().any(|event| {
            matches!(event, AgentEvent::ToolCallUpdate(update)
                if update.status == Some(ToolCallStatus::Failed))
        });
        assert!(failed, "{events:?}");
        let answer = received(&events);
        assert_eq!(answer["behavior"], "deny");
        assert!(answer["message"].as_str().unwrap().contains("rejected"));
        assert!(matches!(events.last(), Some(AgentEvent::TurnEnded { .. })));
    })
    .await;
}

#[tokio::test]
async fn a_cancel_withdraws_what_was_asked_before_it_interrupts() {
    within(async {
        let mut session = start(setup(), &[]);
        let _ready = session.ready().await;
        let (_, request) = session.permission("tool").await;
        session.cancel().await;
        let events = session.turn().await;
        assert_eq!(
            events[0],
            AgentEvent::PermissionResolved {
                id: request.id,
                outcome: PermissionOutcome::Cancelled,
            }
        );
        let answer = received(&events);
        assert_eq!(answer["behavior"], "deny");
        assert!(answer["message"].as_str().unwrap().contains("withdrawn"));
        assert!(matches!(
            events.last(),
            Some(AgentEvent::TurnEnded {
                reason: StopReason::Cancelled,
                ..
            })
        ));
    })
    .await;
}

#[tokio::test]
async fn a_request_the_agent_withdraws_is_settled_without_an_answer() {
    within(async {
        let mut session = start(setup(), &[]);
        let _ready = session.ready().await;
        let (_, request) = session.permission("withdraw").await;
        let events = session.turn().await;
        assert_eq!(
            events[0],
            AgentEvent::PermissionResolved {
                id: request.id,
                outcome: PermissionOutcome::Cancelled,
            }
        );
        assert_eq!(said(&events, false), "Withdrawn.");
        assert!(matches!(
            session.choose(request.id, "allow").await,
            Err(InputError::PermissionSettled(_))
        ));
    })
    .await;
}

#[tokio::test]
async fn a_request_ahead_of_its_frame_shows_its_call_first() {
    within(async {
        let mut session = start(setup(), &[]);
        let _ready = session.ready().await;
        let (events, request) = session.permission("unannounced").await;
        let [
            AgentEvent::ToolCall(call),
            AgentEvent::PermissionRequested(_),
        ] = &events[..]
        else {
            panic!("{events:?}");
        };
        assert_eq!(call.title, "Bash: pwd");
        assert_eq!(request.tool_call, call.id);
        session.choose(request.id, "allow").await.unwrap();
        let events = session.turn().await;
        assert_eq!(received(&events)["updatedInput"], json!({"command": "pwd"}));
    })
    .await;
}

#[tokio::test]
async fn a_plan_is_approved_into_accept_edits_or_rejected() {
    within(async {
        let mut session = start(setup(), &[]);
        let _ready = session.ready().await;
        let (_, request) = session.permission("plan").await;
        let offered: Vec<_> = request
            .options
            .iter()
            .map(|option| (option.id.as_str(), option.kind))
            .collect();
        assert_eq!(
            offered,
            [
                ("reject", PermissionOptionKind::RejectOnce),
                ("implement", PermissionOptionKind::AllowOnce),
            ]
        );
        session.choose(request.id, "implement").await.unwrap();
        let answer = received(&session.turn().await);
        assert_eq!(answer["behavior"], "allow");
        assert_eq!(answer["updatedInput"], json!({"plan": "1. Edit the file"}));
        assert_eq!(
            answer["updatedPermissions"],
            json!([{"type": "setMode", "mode": "acceptEdits", "destination": "session"}])
        );
    })
    .await;
}

/// The question a prompt of `question` asks.
async fn asked(session: &mut Running) -> Question {
    session.prompt(text("question")).await.unwrap();
    let events = session
        .until(|event| matches!(event, AgentEvent::QuestionAsked(_)))
        .await;
    let Some(AgentEvent::QuestionAsked(question)) = events.last().cloned() else {
        panic!("{events:?}");
    };
    question
}

#[tokio::test]
async fn a_question_is_asked_and_its_answers_go_back_by_the_questions_text() {
    within(async {
        let mut session = start(asking(), &[]);
        let _ready = session.ready().await;
        let question = asked(&mut session).await;
        assert_eq!(question.message, "Which colour?\nWhich sizes?");
        let shapes: Vec<_> = question
            .fields
            .iter()
            .map(|field| {
                let FieldKind::Choice {
                    options, multiple, ..
                } = &field.kind
                else {
                    panic!("{field:?}");
                };
                (
                    field.name.as_str().to_owned(),
                    field.title.clone(),
                    options.len(),
                    *multiple,
                )
            })
            .collect();
        assert_eq!(
            shapes,
            [
                ("question_1".to_owned(), Some("Colour".to_owned()), 2, false),
                ("question_2".to_owned(), Some("Sizes".to_owned()), 2, true),
            ]
        );
        let values = BTreeMap::from([
            (
                "question_1".parse().unwrap(),
                AnswerValue::Choice("Blue".to_owned()),
            ),
            (
                "question_2".parse().unwrap(),
                AnswerValue::Choices(vec!["S".to_owned(), "L".to_owned()]),
            ),
        ]);
        // An answer outside the form changes nothing.
        let wrong = BTreeMap::from([(
            "question_1".parse().unwrap(),
            AnswerValue::Choice("Green".to_owned()),
        )]);
        assert!(matches!(
            session
                .reply(question.id, QuestionAnswer::Accept { values: wrong })
                .await,
            Err(InputError::InvalidAnswer { .. })
        ));
        session
            .reply(
                question.id,
                QuestionAnswer::Accept {
                    values: values.clone(),
                },
            )
            .await
            .unwrap();
        let events = session.turn().await;
        assert_eq!(
            events[0],
            AgentEvent::QuestionResolved {
                id: question.id,
                outcome: QuestionOutcome::Answered { values },
            }
        );
        let answer = received(&events);
        assert_eq!(answer["behavior"], "allow");
        assert_eq!(
            answer["updatedInput"]["answers"],
            json!({"Which colour?": "Blue", "Which sizes?": "S, L"})
        );
        assert_eq!(
            answer["updatedInput"]["questions"][0]["question"], "Which colour?",
            "the tool's own input goes back with its answers"
        );
    })
    .await;
}

#[tokio::test]
async fn a_declined_question_denies_the_tool() {
    within(async {
        let mut session = start(asking(), &[]);
        let _ready = session.ready().await;
        let question = asked(&mut session).await;
        session
            .reply(question.id, QuestionAnswer::Decline)
            .await
            .unwrap();
        let events = session.turn().await;
        assert_eq!(
            events[0],
            AgentEvent::QuestionResolved {
                id: question.id,
                outcome: QuestionOutcome::Declined,
            }
        );
        let answer = received(&events);
        assert_eq!(answer["behavior"], "deny");
        assert!(answer["message"].as_str().unwrap().contains("declined"));
    })
    .await;
}

#[tokio::test]
async fn a_question_nobody_can_answer_is_denied_and_the_log_says_why() {
    within(async {
        let mut session = start(setup(), &[]);
        let _ready = session.ready().await;
        session.prompt(text("question")).await.unwrap();
        let events = session.turn().await;
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, AgentEvent::QuestionAsked(_))),
            "{events:?}"
        );
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::HarnessLog { line } if line.contains("Nobody can answer")
        )));
        let answer = received(&events);
        assert_eq!(answer["behavior"], "deny");
        let denial = answer["message"].as_str().unwrap();
        let content = events.iter().find_map(|event| {
            if let AgentEvent::ToolCallUpdate(update) = event {
                update.content.clone()
            } else {
                None
            }
        });
        let Some(content) = content else {
            panic!("{events:?}");
        };
        assert!(matches!(
            &content[0],
            ToolCallContent::Content { content: ContentBlock::Text { text } } if text == denial
        ));
    })
    .await;
}
