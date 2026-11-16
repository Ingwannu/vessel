//! Questions (protocol 10): ACP form elicitations asked, answered,
//! refused and withdrawn, against the `fake-acp` testkit agent (its `ask`
//! prompts).
//!
//! Tests wait on outputs, never on time; the timeout only turns a hang
//! into a failure.

use std::time::Duration;

use vessel_agent::{
    AgentOutput, AgentProcess, AgentSetup, CommandSpec, InputError, ProbeError, Signal,
};
use vessel_proto::conversation::{AnswerValue, InvalidAnswer};
use vessel_proto::{AgentEvent, QuestionAnswer, QuestionId, QuestionOutcome, StopReason};
use vessel_testkit::Fake;

use support::{Agent, accept, check_form, resolved, setup};

#[cfg(test)]
mod support {
    use std::collections::BTreeMap;

    use vessel_agent::{
        AgentInput, AgentOutput, AgentOutputs, AgentProcess, AgentSetup, CommandSpec, Signal,
    };
    use vessel_proto::conversation::{AnswerValue, FieldKind};
    use vessel_proto::{
        AgentEvent, ContentBlock, Question, QuestionAnswer, QuestionId, QuestionOutcome,
    };
    use vessel_testkit::Fake;

    pub(super) fn setup(questions: bool) -> AgentSetup {
        AgentSetup {
            cwd: std::env::temp_dir(),
            model: None,
            resume: None,
            options: Vec::new(),
            mcp_servers: vessel_proto::ResolvedMcpServers::new(),
            handshake_timeout: vessel_agent::DEFAULT_HANDSHAKE_TIMEOUT,
            questions,
            fork: None,
            system_prompt: None,
            private_dir: None,
        }
    }

    /// A ready `fake-acp`, its events and stderr lines as they come.
    pub(super) struct Agent {
        pub(super) process: AgentProcess,
        pub(super) input: AgentInput,
        outputs: AgentOutputs,
        pub(super) events: Vec<AgentEvent>,
        pub(super) log: Vec<String>,
    }

    impl Agent {
        pub(super) async fn ready(questions: bool) -> Self {
            let command = CommandSpec::new(Fake::Acp.path().unwrap());
            let (process, input, outputs) =
                AgentProcess::spawn(&command, setup(questions)).unwrap();
            let mut agent = Self {
                process,
                input,
                outputs,
                events: Vec::new(),
                log: Vec::new(),
            };
            let _ready = agent
                .until(|event| matches!(event, AgentEvent::Ready { .. }))
                .await;
            agent
        }

        /// Read events until one not read yet matches; returns it.
        pub(super) async fn until(&mut self, wanted: impl Fn(&AgentEvent) -> bool) -> AgentEvent {
            loop {
                match self.outputs.next().await {
                    Some(AgentOutput::Event(AgentEvent::HarnessLog { line })) => {
                        self.log.push(line);
                    }
                    Some(AgentOutput::Event(event)) => {
                        self.events.push(event.clone());
                        if wanted(&event) {
                            return event;
                        }
                    }
                    Some(AgentOutput::InputEcho(echo)) => {
                        let event = echo.into_event();
                        self.events.push(event.clone());
                        if wanted(&event) {
                            return event;
                        }
                    }
                    Some(_) => {}
                    None => panic!("outputs ended first: {:?}", self.events),
                }
            }
        }

        pub(super) async fn prompt(&self, text: &str) {
            self.input
                .prompt(vec![ContentBlock::Text {
                    text: text.to_owned(),
                }])
                .await
                .unwrap();
        }

        /// Prompt and read until the question is asked.
        pub(super) async fn asked(&mut self, prompt: &str) -> Question {
            self.prompt(prompt).await;
            let AgentEvent::QuestionAsked(question) = self
                .until(|event| matches!(event, AgentEvent::QuestionAsked(_)))
                .await
            else {
                panic!("matched above");
            };
            question
        }

        /// Read the turn to its end; its message text and how it ended.
        pub(super) async fn turn_end(&mut self) -> (String, AgentEvent) {
            let from = self.events.len();
            let end = self
                .until(|event| matches!(event, AgentEvent::TurnEnded { .. }))
                .await;
            let text = self.events[from..]
                .iter()
                .filter_map(|event| {
                    if let AgentEvent::MessageChunk {
                        content: ContentBlock::Text { text },
                    } = event
                    {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect();
            (text, end)
        }

        pub(super) async fn stop(mut self) {
            self.process.signal(Signal::Terminate).unwrap();
            let _code = self.process.wait().await;
        }
    }

    /// How question `id` was settled, each time it was.
    pub(super) fn resolved(events: &[AgentEvent], id: QuestionId) -> Vec<&QuestionOutcome> {
        events
            .iter()
            .filter_map(|event| {
                if let AgentEvent::QuestionResolved { id: of, outcome } = event
                    && *of == id
                {
                    Some(outcome)
                } else {
                    None
                }
            })
            .collect()
    }

    pub(super) fn accept(values: &[(&str, AnswerValue)]) -> QuestionAnswer {
        QuestionAnswer::Accept {
            values: values
                .iter()
                .map(|(name, value)| (name.parse().unwrap(), value.clone()))
                .collect::<BTreeMap<_, _>>(),
        }
    }

    /// The fake's form, as the person sees it.
    pub(super) fn check_form(question: &Question) {
        assert_eq!(question.message, "Which way?");
        let names: Vec<&str> = question.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            ["name", "sure", "way"],
            "by key, as ACP's crate keeps them"
        );
        assert!(matches!(
            &question.fields[0].kind,
            FieldKind::Text { default: Some(name), .. } if name == "vessel"
        ));
        assert!(matches!(question.fields[1].kind, FieldKind::Boolean { .. }));
        assert!(question.fields[2].required);
        assert!(matches!(
            &question.fields[2].kind,
            FieldKind::Choice { options, multiple: false, .. } if options.len() == 2
        ));
    }
}

const SAFETY_NET: Duration = Duration::from_secs(30);

/// Asked, refused when the answer does not fit, answered once: the agent
/// gets the values, the timeline the outcome, a second answer is refused.
#[tokio::test]
async fn a_question_is_asked_and_answered_once() {
    vessel_testkit::within(SAFETY_NET, async {
        let mut agent = Agent::ready(true).await;
        let question = agent.asked("ask").await;
        check_form(&question);

        let missing = agent
            .input
            .answer_question(question.id, accept(&[("sure", AnswerValue::Boolean(true))]))
            .await
            .unwrap_err();
        assert!(
            matches!(
                &missing,
                InputError::InvalidAnswer { id, reason: InvalidAnswer::Missing(field) }
                    if *id == question.id && field.as_str() == "way"
            ),
            "{missing:?}"
        );
        let wrong = agent
            .input
            .answer_question(
                question.id,
                accept(&[("way", AnswerValue::Choice("up".to_owned()))]),
            )
            .await
            .unwrap_err();
        assert!(
            matches!(wrong, InputError::InvalidAnswer { .. }),
            "{wrong:?}"
        );

        let answer = accept(&[
            ("way", AnswerValue::Choice("left".to_owned())),
            ("sure", AnswerValue::Boolean(true)),
        ]);
        agent
            .input
            .answer_question(question.id, answer.clone())
            .await
            .unwrap();
        let (text, end) = agent.turn_end().await;
        assert_eq!(text, r#"answered: accept {"sure":true,"way":"left"}"#);
        assert!(matches!(
            end,
            AgentEvent::TurnEnded {
                reason: StopReason::EndTurn,
                ..
            }
        ));
        let QuestionAnswer::Accept { values } = answer else {
            panic!();
        };
        assert_eq!(
            resolved(&agent.events, question.id),
            [&QuestionOutcome::Answered { values }]
        );

        let again = agent
            .input
            .answer_question(question.id, QuestionAnswer::Decline)
            .await
            .unwrap_err();
        assert!(matches!(again, InputError::QuestionSettled(id) if id == question.id));
        let unknown = QuestionId::random();
        let never = agent
            .input
            .answer_question(unknown, QuestionAnswer::Decline)
            .await
            .unwrap_err();
        assert!(matches!(never, InputError::QuestionNotFound(id) if id == unknown));
        agent.stop().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_declined_question_is_declined_to_the_agent() {
    vessel_testkit::within(SAFETY_NET, async {
        let mut agent = Agent::ready(true).await;
        let question = agent.asked("ask").await;
        agent
            .input
            .answer_question(question.id, QuestionAnswer::Decline)
            .await
            .unwrap();
        let (text, _end) = agent.turn_end().await;
        assert_eq!(text, "answered: decline");
        assert_eq!(
            resolved(&agent.events, question.id),
            [&QuestionOutcome::Declined]
        );
        agent.stop().await;
    })
    .await
    .unwrap();
}

/// The agent cancels its own request: the question is resolved
/// `cancelled` before the turn goes on, and the agent still gets its one
/// answer (`cancel`); a late answer hears "already settled".
#[tokio::test]
async fn a_question_the_agent_withdraws_is_cancelled() {
    vessel_testkit::within(SAFETY_NET, async {
        let mut agent = Agent::ready(true).await;
        let question = agent.asked("ask withdraw").await;
        let (text, end) = agent.turn_end().await;
        assert_eq!(text, "answered: cancel");
        assert!(matches!(
            end,
            AgentEvent::TurnEnded {
                reason: StopReason::EndTurn,
                ..
            }
        ));
        assert_eq!(
            resolved(&agent.events, question.id),
            [&QuestionOutcome::Cancelled]
        );
        let late = agent
            .input
            .answer_question(question.id, QuestionAnswer::Decline)
            .await
            .unwrap_err();
        assert!(matches!(late, InputError::QuestionSettled(_)), "{late:?}");
        agent.stop().await;
    })
    .await
    .unwrap();
}

/// A cancelled turn withdraws the question waiting in it, as it does a
/// permission request.
#[tokio::test]
async fn a_cancelled_turn_withdraws_its_question() {
    vessel_testkit::within(SAFETY_NET, async {
        let mut agent = Agent::ready(true).await;
        let question = agent.asked("ask").await;
        agent.input.cancel_turn().await.unwrap();
        let (text, end) = agent.turn_end().await;
        assert_eq!(text, "answered: cancel");
        assert!(matches!(
            end,
            AgentEvent::TurnEnded {
                reason: StopReason::Cancelled,
                ..
            }
        ));
        assert_eq!(
            resolved(&agent.events, question.id),
            [&QuestionOutcome::Cancelled],
            "once"
        );
        agent.stop().await;
    })
    .await
    .unwrap();
}

/// Without questions advertised the agent does not ask (Claude's adapter
/// then has no question tool); the fake says so.
#[tokio::test]
async fn an_agent_not_offered_questions_does_not_ask() {
    vessel_testkit::within(SAFETY_NET, async {
        let mut agent = Agent::ready(false).await;
        agent.prompt("ask").await;
        let (text, _end) = agent.turn_end().await;
        assert_eq!(text, "no questions");
        assert!(
            !agent
                .events
                .iter()
                .any(|event| matches!(event, AgentEvent::QuestionAsked(_))),
            "{:?}",
            agent.events
        );
        assert!(
            !agent
                .log
                .iter()
                .any(|line| line.contains("takes questions"))
        );
        agent.stop().await;
    })
    .await
    .unwrap();
}

/// What Vessel did not advertise is refused at once, without a
/// question: a link to open, and a form in a session that takes no
/// questions. The log says why; the agent gets an error.
#[tokio::test]
async fn what_was_not_advertised_is_refused() {
    vessel_testkit::within(SAFETY_NET, async {
        for questions in [true, false] {
            let mut agent = Agent::ready(questions).await;
            agent.prompt("ask link").await;
            let (text, _end) = agent.turn_end().await;
            assert!(text.starts_with("answered: error "), "{text}");
            assert!(
                !agent
                    .events
                    .iter()
                    .any(|event| matches!(event, AgentEvent::QuestionAsked(_))),
                "{:?}",
                agent.events
            );
            let refused = agent
                .log
                .iter()
                .find(|line| line.starts_with("vessel: "))
                .unwrap_or_else(|| panic!("no line says why: {:?}", agent.log));
            assert!(refused.contains("url elicitation"), "{refused}");
            assert!(
                !refused.contains("example.com"),
                "never the link: {refused}"
            );
            agent.stop().await;
        }
    })
    .await
    .unwrap();
}

/// A probe never advertises questions, whatever its setup says: the fake,
/// stuck after `initialize`, would have said so on stderr.
#[tokio::test]
async fn a_probe_never_offers_questions() {
    vessel_testkit::within(SAFETY_NET, async {
        let command =
            CommandSpec::new(Fake::Acp.path().unwrap()).env("FAKE_ACP_HANG", "session/new");
        let stuck = vessel_agent::probe(
            &command,
            AgentSetup {
                handshake_timeout: Duration::from_secs(2),
                ..setup(true)
            },
        )
        .await
        .unwrap_err();
        let ProbeError::TimedOut { log, .. } = &stuck else {
            panic!("{stuck:?}");
        };
        assert!(
            log.iter()
                .any(|line| line == "fake-acp: stuck at session/new"),
            "past initialize: {log:?}"
        );
        assert!(
            !log.iter().any(|line| line.contains("takes questions")),
            "{log:?}"
        );

        // The same setup in a session does offer them.
        let (mut process, _input, mut outputs) =
            AgentProcess::spawn(&command, setup(true)).unwrap();
        let mut said = Vec::new();
        loop {
            let Some(output) = outputs.next().await else {
                panic!("ended without saying it takes questions: {said:?}");
            };
            if let AgentOutput::Event(AgentEvent::HarnessLog { line }) = output {
                if line == "fake-acp: the client takes questions" {
                    break;
                }
                said.push(line);
            }
        }
        process.signal(Signal::Kill).unwrap();
        let _code = process.wait().await;
    })
    .await
    .unwrap();
}
