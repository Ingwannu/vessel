//! The Codex app-server driver (ADR 0057, Codex slices 1 and 2) against
//! `fake-codex-app-server`, whose messages follow Codex 0.166.0's schema.
//!
//! Tests wait on outputs, never on time; the timeout only turns a hang into
//! a failure.

use vessel_agent::{
    AgentOutput, Disconnect, Driver, ForkFrom, ForkRefusal, InputError, SteerOutcome,
};
use vessel_proto::conversation::{AnswerValue, FieldKind};
use vessel_proto::{
    AgentEvent, ChoiceValue, ContentBlock, ContextUsage, HarnessSessionId, OptionChoice, OptionId,
    OptionKind, OptionValueId, PermissionOptionId, PermissionOutcome, PlanStatus, QuestionAnswer,
    SessionOption, StopReason, TokenCount, ToolCallContent, ToolCallId, ToolCallStatus, ToolKind,
};

use support::{
    asked, command, events, mode, question, ready, said, setup, spawn, text, turn, until, within,
};

#[cfg(test)]
mod support {
    use std::future::Future;
    use std::time::Duration;

    use vessel_agent::{
        AgentInput, AgentOutput, AgentOutputs, AgentProcess, AgentSetup, CommandSpec, Driver,
    };
    use vessel_proto::{
        AgentEvent, ChoiceValue, ContentBlock, HarnessSessionId, OptionChoice, OptionId,
        OptionValueId, PermissionRequest, Question,
    };
    use vessel_testkit::Fake;

    const SAFETY_NET: Duration = Duration::from_secs(30);

    pub(super) async fn within<F: Future>(test: F) -> F::Output {
        vessel_testkit::within(SAFETY_NET, test)
            .await
            .expect("test timed out")
    }

    pub(super) fn setup(resume: Option<&str>) -> AgentSetup {
        AgentSetup {
            cwd: std::env::temp_dir(),
            model: None,
            resume: resume.map(HarnessSessionId::new),
            options: Vec::new(),
            mcp_servers: vessel_proto::ResolvedMcpServers::new(),
            handshake_timeout: vessel_agent::DEFAULT_HANDSHAKE_TIMEOUT,
            questions: false,
            fork: None,
            system_prompt: None,
            private_dir: None,
        }
    }

    pub(super) fn command() -> CommandSpec {
        CommandSpec::new(Fake::CodexAppServer.path().unwrap()).arg("app-server")
    }

    pub(super) fn spawn(setup: AgentSetup) -> (AgentProcess, AgentInput, AgentOutputs) {
        AgentProcess::spawn_with(Driver::CodexAppServer, &command(), setup).unwrap()
    }

    pub(super) fn text(text: &str) -> Vec<ContentBlock> {
        vec![ContentBlock::Text {
            text: text.to_owned(),
        }]
    }

    /// Outputs until one `done` accepts, that one included.
    pub(super) async fn until(
        outputs: &mut AgentOutputs,
        done: impl Fn(&AgentOutput) -> bool,
    ) -> Vec<AgentOutput> {
        let mut seen = Vec::new();
        while let Some(output) = outputs.next().await {
            let last = done(&output);
            seen.push(output);
            if last {
                return seen;
            }
        }
        panic!("the outputs ended first: {seen:?}");
    }

    /// The events among `outputs`.
    pub(super) fn events(outputs: Vec<AgentOutput>) -> Vec<AgentEvent> {
        outputs
            .into_iter()
            .filter_map(|output| {
                if let AgentOutput::Event(event) = output {
                    Some(event)
                } else {
                    None
                }
            })
            .collect()
    }

    /// The events of a turn, through its end.
    pub(super) async fn turn(outputs: &mut AgentOutputs) -> Vec<AgentEvent> {
        events(
            until(outputs, |output| {
                matches!(
                    output,
                    AgentOutput::Event(
                        AgentEvent::TurnEnded { .. } | AgentEvent::TurnFailed { .. }
                    )
                )
            })
            .await,
        )
    }

    /// The text of the message chunks in `events`.
    pub(super) fn said(events: &[AgentEvent]) -> String {
        events
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
            .collect()
    }

    pub(super) async fn ready(outputs: &mut AgentOutputs) -> Vec<AgentOutput> {
        until(outputs, |output| {
            matches!(output, AgentOutput::Event(AgentEvent::Ready { .. }))
        })
        .await
    }

    /// The events through the next permission request, and the request.
    pub(super) async fn asked(outputs: &mut AgentOutputs) -> (Vec<AgentEvent>, PermissionRequest) {
        let mut seen = events(
            until(outputs, |output| {
                matches!(
                    output,
                    AgentOutput::Event(AgentEvent::PermissionRequested(_))
                )
            })
            .await,
        );
        let Some(AgentEvent::PermissionRequested(request)) = seen.pop() else {
            panic!("no permission request");
        };
        (seen, request)
    }

    /// The next question asked.
    pub(super) async fn question(outputs: &mut AgentOutputs) -> Question {
        let seen = until(outputs, |output| {
            matches!(output, AgentOutput::Event(AgentEvent::QuestionAsked(_)))
        })
        .await;
        let Some(AgentOutput::Event(AgentEvent::QuestionAsked(question))) = seen.last() else {
            panic!("no question: {seen:?}");
        };
        question.clone()
    }

    /// A choice of the mode.
    pub(super) fn mode(value: &str) -> OptionChoice {
        OptionChoice {
            id: OptionId::new("mode"),
            value: ChoiceValue::Select {
                value: OptionValueId::new(value),
            },
        }
    }
}

/// The current value of the one option, the mode, in `options`.
fn current(options: &[SessionOption]) -> Option<&str> {
    match options {
        [
            SessionOption {
                kind: OptionKind::Select { current, .. },
                ..
            },
        ] => Some(current.as_str()),
        _ => None,
    }
}

/// The handshake says who the agent is, its options (the mode), then ready on a
/// new thread; a prompt sent before it is ready waits for it; the reply
/// streams in deltas (another thread's delta is not this session's), then
/// usage and the turn's end with its tokens.
#[tokio::test]
async fn a_thread_starts_and_a_turn_streams_its_reply() {
    within(async {
        let (_agent, input, mut outputs) = spawn(setup(None));
        input.prompt(text("plain hello")).await.unwrap();
        let handshake = ready(&mut outputs).await;
        let AgentOutput::Introduced(introduction) = &handshake[0] else {
            panic!("introduced first: {handshake:?}");
        };
        assert_eq!(
            introduction.version.as_ref().map(ToString::to_string),
            Some("0.166.0".to_owned())
        );
        let AgentOutput::Options(options) = &handshake[1] else {
            panic!("options second: {handshake:?}");
        };
        assert_eq!(current(options), Some("agent"));
        let AgentOutput::Event(AgentEvent::Ready {
            harness_session,
            abilities,
        }) = &handshake[2]
        else {
            panic!("ready third: {handshake:?}");
        };
        assert!(harness_session.as_str().starts_with("fake-thread-"));
        assert!(abilities.is_some_and(|abilities| abilities.steer && abilities.resume));

        let outputs_of_turn = until(&mut outputs, |output| {
            matches!(output, AgentOutput::Event(AgentEvent::TurnEnded { .. }))
        })
        .await;
        assert!(outputs_of_turn.contains(&AgentOutput::Usage(ContextUsage {
            observation: vessel_proto::conversation::UsageObservation::Current,
            used: TokenCount::new(100),
            size: TokenCount::new(1000),
            cost: None,
        })));
        let events = events(outputs_of_turn);
        assert_eq!(
            events[0],
            AgentEvent::TurnStarted {
                content: text("plain hello")
            }
        );
        assert_eq!(said(&events), "fake reply: plain hello");
        let Some(AgentEvent::TurnEnded {
            reason: StopReason::EndTurn,
            usage: Some(usage),
            ..
        }) = events.last()
        else {
            panic!("ended: {events:?}");
        };
        assert_eq!(usage.total, TokenCount::new(100));
    })
    .await;
}

/// Reasoning, a command, a plan and a message without deltas become the
/// timeline's thought, tool call, plan and message.
#[tokio::test]
async fn items_plans_and_reasoning_become_events() {
    within(async {
        let (_agent, input, mut outputs) = spawn(setup(None));
        let _ready = ready(&mut outputs).await;

        input.prompt(text("think")).await.unwrap();
        let events = turn(&mut outputs).await;
        assert!(events.contains(&AgentEvent::ThoughtChunk {
            content: ContentBlock::Text {
                text: "thinking it over".to_owned()
            }
        }));

        input.prompt(text("tool")).await.unwrap();
        let events = turn(&mut outputs).await;
        let Some(AgentEvent::ToolCall(call)) = events
            .iter()
            .find(|event| matches!(event, AgentEvent::ToolCall(_)))
        else {
            panic!("a tool call: {events:?}");
        };
        assert_eq!(call.id, ToolCallId::new("cmd-1"));
        assert_eq!(call.title, "echo hi");
        assert_eq!(
            (call.kind, call.status),
            (ToolKind::Execute, ToolCallStatus::InProgress)
        );
        let Some(AgentEvent::ToolCallUpdate(update)) = events
            .iter()
            .find(|event| matches!(event, AgentEvent::ToolCallUpdate(_)))
        else {
            panic!("its update: {events:?}");
        };
        assert_eq!(update.status, Some(ToolCallStatus::Completed));
        assert_eq!(
            update.content,
            Some(vec![ToolCallContent::Content {
                content: ContentBlock::Text {
                    text: "hi\n".to_owned()
                }
            }])
        );

        input.prompt(text("plan")).await.unwrap();
        let events = turn(&mut outputs).await;
        let Some(AgentEvent::Plan { entries }) = events
            .iter()
            .find(|event| matches!(event, AgentEvent::Plan { .. }))
        else {
            panic!("a plan: {events:?}");
        };
        let steps: Vec<_> = entries
            .iter()
            .map(|entry| (entry.content.as_str(), entry.status))
            .collect();
        assert_eq!(
            steps,
            [
                ("read", PlanStatus::Completed),
                ("write", PlanStatus::InProgress)
            ]
        );

        input.prompt(text("quiet")).await.unwrap();
        let events = turn(&mut outputs).await;
        assert_eq!(said(&events), "said all at once");
    })
    .await;
}

/// An approval is asked with Codex's own choices, on its tool call
/// (announced from the request, then updated by the item); the answer goes
/// back as the decision chosen, a rule and all, and only once.
#[tokio::test]
async fn an_approval_is_asked_with_codexs_choices_and_answered() {
    within(async {
        let (_agent, input, mut outputs) = spawn(setup(None));
        let _ready = ready(&mut outputs).await;
        input.prompt(text("approve")).await.unwrap();
        let (before, request) = asked(&mut outputs).await;
        assert!(
            before.iter().any(|event| matches!(
                event,
                AgentEvent::ToolCall(call) if call.id == ToolCallId::new("cmd-1")
                    && call.kind == ToolKind::Execute
                    && call.status == ToolCallStatus::Pending
            )),
            "{before:?}"
        );
        assert_eq!(request.tool_call, ToolCallId::new("cmd-1"));
        let offered: Vec<&str> = request
            .options
            .iter()
            .map(|option| option.id.as_str())
            .collect();
        assert_eq!(
            offered,
            [
                "accept",
                "acceptForSession",
                "acceptWithExecpolicyAmendment",
                "decline",
                "cancel"
            ]
        );
        let outcome = PermissionOutcome::Selected {
            option: PermissionOptionId::new("acceptWithExecpolicyAmendment"),
        };
        input.answer(request.id, outcome.clone()).await.unwrap();
        assert!(matches!(
            input.answer(request.id, outcome.clone()).await,
            Err(InputError::PermissionSettled(_))
        ));
        let events = turn(&mut outputs).await;
        assert!(events.contains(&AgentEvent::PermissionResolved {
            id: request.id,
            outcome
        }));
        let announced = events
            .iter()
            .filter(|event| matches!(event, AgentEvent::ToolCall(_)))
            .count();
        assert_eq!(announced, 0, "announced twice: {events:?}");
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::ToolCallUpdate(update) if update.status == Some(ToolCallStatus::InProgress)
        )));
        assert_eq!(
            said(&events),
            r#"decision {"acceptWithExecpolicyAmendment":{"execpolicy_amendment":["echo"]}}"#
        );
    })
    .await;
}

/// A cancel withdraws the open approval (Codex hears `cancel`) and ends
/// the turn cancelled; Codex resolving a request settles it too.
#[tokio::test]
async fn a_cancel_or_codex_withdraws_an_open_approval() {
    within(async {
        let (_agent, input, mut outputs) = spawn(setup(None));
        let _ready = ready(&mut outputs).await;
        input.prompt(text("approve")).await.unwrap();
        let (_before, request) = asked(&mut outputs).await;
        input.cancel_turn().await.unwrap();
        let events = turn(&mut outputs).await;
        assert!(events.contains(&AgentEvent::PermissionResolved {
            id: request.id,
            outcome: PermissionOutcome::Cancelled
        }));
        assert_eq!(said(&events), r#"decision "cancel""#);
        assert!(matches!(
            events.last(),
            Some(AgentEvent::TurnEnded {
                reason: StopReason::Cancelled,
                ..
            })
        ));

        input.prompt(text("resolved")).await.unwrap();
        let (_before, request) = asked(&mut outputs).await;
        let events = turn(&mut outputs).await;
        assert_eq!(
            events.first(),
            Some(&AgentEvent::PermissionResolved {
                id: request.id,
                outcome: PermissionOutcome::Cancelled
            })
        );
        assert!(matches!(
            input.answer(request.id, PermissionOutcome::Cancelled).await,
            Err(InputError::PermissionSettled(_))
        ));
    })
    .await;
}

/// A file change offers its four decisions on its item's tool call; more
/// permissions are granted as asked, for the turn or the session.
#[tokio::test]
async fn a_file_change_and_more_permissions_are_asked() {
    within(async {
        let (_agent, input, mut outputs) = spawn(setup(None));
        let _ready = ready(&mut outputs).await;
        input.prompt(text("edit")).await.unwrap();
        let (_before, request) = asked(&mut outputs).await;
        assert_eq!(request.tool_call, ToolCallId::new("edit-1"));
        assert_eq!(request.options.len(), 4);
        let decline = PermissionOutcome::Selected {
            option: PermissionOptionId::new("decline"),
        };
        input.answer(request.id, decline).await.unwrap();
        let events = turn(&mut outputs).await;
        assert_eq!(said(&events), r#"decision "decline""#);
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::ToolCallUpdate(update) if update.status == Some(ToolCallStatus::Failed)
        )));

        input.prompt(text("permit")).await.unwrap();
        let (before, request) = asked(&mut outputs).await;
        assert!(before.iter().any(|event| matches!(
            event,
            AgentEvent::ToolCall(call) if call.id == ToolCallId::new("perm-1")
                && call.title == "Fetch the docs"
        )));
        let session = PermissionOutcome::Selected {
            option: PermissionOptionId::new("session"),
        };
        input.answer(request.id, session).await.unwrap();
        let events = turn(&mut outputs).await;
        assert_eq!(
            said(&events),
            r#"granted session {"network":{"enabled":true}}"#
        );
    })
    .await;
}

/// With someone watching, Codex's questions and a tool server's form are
/// asked, checked and answered; a secret is never asked.
#[tokio::test]
async fn questions_are_asked_when_someone_watches() {
    within(async {
        let mut watched = setup(None);
        watched.questions = true;
        let (_agent, input, mut outputs) = spawn(watched);
        let _ready = ready(&mut outputs).await;
        input.prompt(text("ask")).await.unwrap();
        let asked = question(&mut outputs).await;
        let kinds: Vec<bool> = asked
            .fields
            .iter()
            .map(|field| matches!(field.kind, FieldKind::Choice { .. }))
            .collect();
        assert_eq!(kinds, [true, false]);
        let wrong = [(
            "color".parse().unwrap(),
            AnswerValue::Choice("green".to_owned()),
        )]
        .into();
        assert!(matches!(
            input
                .answer_question(asked.id, QuestionAnswer::Accept { values: wrong })
                .await,
            Err(InputError::InvalidAnswer { .. })
        ));
        let values = [
            (
                "color".parse().unwrap(),
                AnswerValue::Choice("blue".to_owned()),
            ),
            ("name".parse().unwrap(), AnswerValue::Text("Ann".to_owned())),
        ]
        .into();
        input
            .answer_question(asked.id, QuestionAnswer::Accept { values })
            .await
            .unwrap();
        let events = turn(&mut outputs).await;
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::QuestionResolved { id, .. } if *id == asked.id
        )));
        assert_eq!(
            said(&events),
            r#"answers {"color":{"answers":["blue"]},"name":{"answers":["Ann"]}}"#
        );

        input.prompt(text("form")).await.unwrap();
        let asked = question(&mut outputs).await;
        assert_eq!(asked.message, "Who?");
        let values = [("name".parse().unwrap(), AnswerValue::Text("Ann".to_owned()))].into();
        input
            .answer_question(asked.id, QuestionAnswer::Accept { values })
            .await
            .unwrap();
        assert_eq!(
            said(&turn(&mut outputs).await),
            r#"action accept {"name":"Ann"}"#
        );

        input.prompt(text("secret")).await.unwrap();
        let events = turn(&mut outputs).await;
        assert!(
            events.iter().any(|event| matches!(
                event,
                AgentEvent::HarnessLog { line } if line.contains("secret")
            )),
            "{events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, AgentEvent::QuestionAsked(_)))
        );
        assert_eq!(said(&events), "answers {}");
    })
    .await;
}

/// With nobody watching, a question is answered empty and a form declined
/// at once, each with a line in the log.
#[tokio::test]
async fn with_nobody_watching_questions_are_not_asked() {
    within(async {
        let (_agent, input, mut outputs) = spawn(setup(None));
        let _ready = ready(&mut outputs).await;
        input.prompt(text("ask")).await.unwrap();
        let events = turn(&mut outputs).await;
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::HarnessLog { line } if line.contains("nobody watches")
        )));
        assert_eq!(said(&events), "answers {}");
        input.prompt(text("form")).await.unwrap();
        assert_eq!(said(&turn(&mut outputs).await), "action decline null");
    })
    .await;
}

/// The mode chosen goes with the thread's start and with every turn; a
/// change goes with the next turn and is reported; another option is
/// refused. By default Codex's reviewer looks first (`agent`).
#[tokio::test]
async fn the_mode_goes_with_the_thread_and_each_turn() {
    within(async {
        let (_agent, input, mut outputs) = spawn(setup(None));
        let _ready = ready(&mut outputs).await;
        input.prompt(text("policy")).await.unwrap();
        assert_eq!(
            said(&turn(&mut outputs).await),
            "turn on-request auto_review workspaceWrite thread on-request auto_review \
             workspace-write"
        );

        let mut reading = setup(None);
        reading.options = vec![mode("read-only")];
        let (_agent, input, mut outputs) = spawn(reading);
        let handshake = ready(&mut outputs).await;
        assert!(
            matches!(&handshake[1], AgentOutput::Options(options) if current(options) == Some("read-only"))
        );
        input.prompt(text("policy")).await.unwrap();
        assert_eq!(
            said(&turn(&mut outputs).await),
            "turn on-request user readOnly thread on-request user read-only"
        );
        let options = input.set_option(mode("agent-full-access")).await.unwrap();
        assert_eq!(current(&options), Some("agent-full-access"));
        assert_eq!(current(&input.options().await.unwrap()), Some("agent-full-access"));
        let refused = OptionChoice {
            id: OptionId::new("model"),
            value: ChoiceValue::Select {
                value: OptionValueId::new("o"),
            },
        };
        assert!(matches!(
            input.set_option(refused).await,
            Err(InputError::Option(_))
        ));
        input.prompt(text("policy")).await.unwrap();
        let events = until(&mut outputs, |output| {
            matches!(output, AgentOutput::Event(AgentEvent::TurnEnded { .. }))
        })
        .await;
        assert!(
            events.iter().any(|output| matches!(
                output,
                AgentOutput::Options(options) if current(options) == Some("agent-full-access")
            )),
            "{events:?}"
        );
        assert_eq!(
            said(&support::events(events)),
            "turn never user dangerFullAccess thread on-request user read-only"
        );
    })
    .await;
}

/// A running turn takes a steer (said back by the agent), refuses a second
/// prompt, and ends cancelled when cancelled; with no turn, a steer is a
/// prompt to send.
#[tokio::test]
async fn a_running_turn_is_steered_then_cancelled() {
    within(async {
        let (_agent, input, mut outputs) = spawn(setup(None));
        let _ready = ready(&mut outputs).await;
        assert_eq!(
            input.steer(text("early")).await.unwrap(),
            SteerOutcome::PromptRequired
        );
        input.prompt(text("wait")).await.unwrap();
        let _started = until(&mut outputs, |output| {
            matches!(output, AgentOutput::Event(AgentEvent::TurnStarted { .. }))
        })
        .await;
        // The turn's id comes with `turn/start`'s answer; a steer before it
        // is refused, so ask until the turn is known.
        let outcome = loop {
            match input.steer(text("more")).await {
                Err(InputError::Steering(_)) => tokio::task::yield_now().await,
                other => break other,
            }
        };
        assert_eq!(outcome.unwrap(), SteerOutcome::Injected);
        assert!(input.turn_running().await.unwrap());
        assert!(matches!(
            input.prompt(text("another")).await,
            Err(InputError::TurnInProgress)
        ));
        input.cancel_turn().await.unwrap();
        let events = turn(&mut outputs).await;
        assert!(events.contains(&AgentEvent::Steered {
            content: text("more")
        }));
        assert_eq!(said(&events), "steered: more");
        assert!(matches!(
            events.last(),
            Some(AgentEvent::TurnEnded {
                reason: StopReason::Cancelled,
                ..
            })
        ));
        assert!(!input.turn_running().await.unwrap());
    })
    .await;
}

/// A failed turn says the agent's error; an agent that exits mid-turn
/// fails the turn, then the connection ends.
#[tokio::test]
async fn a_failed_turn_and_an_agent_that_exits() {
    within(async {
        let (mut agent, input, mut outputs) = spawn(setup(None));
        let _ready = ready(&mut outputs).await;
        input.prompt(text("fail")).await.unwrap();
        let events = turn(&mut outputs).await;
        assert_eq!(
            events.last(),
            Some(&AgentEvent::TurnFailed {
                message: "fake failure".to_owned()
            })
        );
        input.prompt(text("exit")).await.unwrap();
        let rest = until(&mut outputs, |output| {
            matches!(output, AgentOutput::Disconnected(_))
        })
        .await;
        assert!(rest.contains(&AgentOutput::Event(AgentEvent::TurnFailed {
            message: "the agent exited during the turn".to_owned()
        })));
        assert_eq!(
            rest.last(),
            Some(&AgentOutput::Disconnected(Disconnect::Closed))
        );
        assert_eq!(agent.wait().await, Some(0));
    })
    .await;
}

/// A resume opens the thread asked for; one the agent does not have fails
/// the handshake, and so does a fork (the history slice's).
#[tokio::test]
async fn a_thread_is_resumed_and_what_cannot_open_fails_the_handshake() {
    within(async {
        let (_agent, _input, mut outputs) = spawn(setup(Some("thread-7")));
        let handshake = ready(&mut outputs).await;
        assert!(matches!(
            handshake.last(),
            Some(AgentOutput::Event(AgentEvent::Ready { harness_session, .. }))
                if harness_session.as_str() == "thread-7"
        ));

        let (_agent, _input, mut outputs) = spawn(setup(Some("missing-1")));
        let failed = until(&mut outputs, |output| {
            matches!(output, AgentOutput::Disconnected(_))
        })
        .await;
        assert!(
            matches!(
                failed.last(),
                Some(AgentOutput::Disconnected(Disconnect::Handshake { message, .. }))
                    if message.contains("no thread missing-1")
            ),
            "{failed:?}"
        );

        let mut forking = setup(None);
        forking.fork = Some(Box::new(ForkFrom {
            session: HarnessSessionId::new("thread-7"),
            point: None,
        }));
        let (_agent, _input, mut outputs) = spawn(forking);
        let refused = until(&mut outputs, |output| {
            matches!(output, AgentOutput::Disconnected(_))
        })
        .await;
        assert!(matches!(
            refused.last(),
            Some(AgentOutput::Disconnected(Disconnect::Handshake {
                fork_refused: Some(ForkRefusal::CannotFork),
                ..
            }))
        ));
    })
    .await;
}

/// A probe reads the agent's introduction, abilities and options (the
/// mode, for a new-session form) and stops it.
#[tokio::test]
async fn a_probe_reads_the_introduction() {
    within(async {
        let probed = vessel_agent::probe_with(Driver::CodexAppServer, &command(), setup(None))
            .await
            .unwrap();
        assert_eq!(current(&probed.options), Some("agent"));
        assert!(probed.abilities.steer);
        assert_eq!(
            probed
                .introduction
                .version
                .map(|version| version.to_string()),
            Some("0.166.0".to_owned())
        );
    })
    .await;
}

/// Codex's completions are matched to the running turn: another turn of the
/// thread does not end it, and one that does not read ends it failed
/// rather than leave the session busy.
#[tokio::test]
async fn only_the_running_turns_completion_ends_it() {
    within(async {
        let (_agent, input, mut outputs) = spawn(setup(None));
        let _ready = ready(&mut outputs).await;
        input.prompt(text("other")).await.unwrap();
        let events = turn(&mut outputs).await;
        assert_eq!(said(&events), "fake reply: other");
        input.prompt(text("garbled")).await.unwrap();
        let events = turn(&mut outputs).await;
        assert!(
            matches!(
                events.last(),
                Some(AgentEvent::TurnFailed { message }) if message.contains("did not read")
            ),
            "{events:?}"
        );
        assert!(!input.turn_running().await.unwrap());
        // The turn's id comes only with `turn/start`'s answer: one that
        // does not read ends the turn failed rather than leave it starting.
        input.prompt(text("badstart")).await.unwrap();
        let events = turn(&mut outputs).await;
        assert!(
            matches!(
                events.last(),
                Some(AgentEvent::TurnFailed { message })
                    if message.contains("answer to turn/start did not read")
            ),
            "{events:?}"
        );
        assert!(!input.turn_running().await.unwrap());
    })
    .await;
}

/// Until the options slice, a session asked for a choice other than the
/// mode is not started without it: the handshake fails as for an option
/// an agent does not offer. Tool servers are said to be left out.
#[tokio::test]
async fn other_choices_fail_the_handshake_and_tool_servers_are_said_to_be_left_out() {
    within(async {
        let mut choosing = setup(None);
        choosing.options = vec![OptionChoice {
            id: OptionId::new("effort"),
            value: ChoiceValue::Select {
                value: OptionValueId::new("high"),
            },
        }];
        let (_agent, _input, mut outputs) = spawn(choosing);
        let refused = until(&mut outputs, |output| {
            matches!(output, AgentOutput::Disconnected(_))
        })
        .await;
        assert!(
            matches!(
                refused.last(),
                Some(AgentOutput::Disconnected(Disconnect::Handshake { message, .. }))
                    if message.contains("effort")
            ),
            "{refused:?}"
        );

        let mut serving = setup(None);
        let _none = serving.mcp_servers.insert(
            "vessel".parse().unwrap(),
            vessel_proto::McpTransport::Stdio {
                command: "vessel".into(),
                args: Vec::new(),
                env: std::collections::BTreeMap::new(),
            },
        );
        let (_agent, _input, mut outputs) = spawn(serving);
        let said = until(&mut outputs, |output| {
            matches!(output, AgentOutput::Event(AgentEvent::HarnessLog { .. }))
        })
        .await;
        assert!(
            matches!(
                said.last(),
                Some(AgentOutput::Event(AgentEvent::HarnessLog { line }))
                    if line.contains("without vessel")
            ),
            "{said:?}"
        );
    })
    .await;
}

/// A cancel ends a turn while its reply streams, end to end: the fake
/// streams 200,000 deltas without a pause unless interrupted. (That the
/// loop takes a command before the agent's queued lines is its own unit
/// test, `a_command_is_taken_before_queued_agent_lines`; here the reader
/// may fall behind and let either order through.)
#[tokio::test]
async fn a_cancel_lands_while_the_reply_streams() {
    within(async {
        let (_agent, input, mut outputs) = spawn(setup(None));
        let _ready = ready(&mut outputs).await;
        input.prompt(text("stream")).await.unwrap();
        let _first = until(&mut outputs, |output| {
            matches!(output, AgentOutput::Event(AgentEvent::MessageChunk { .. }))
        })
        .await;
        // From a task of its own: the outputs must keep being read, or the
        // driver waits on them.
        let cancelling = tokio::spawn(async move { input.cancel_turn().await });
        let events = turn(&mut outputs).await;
        cancelling.await.unwrap().unwrap();
        assert!(
            matches!(
                events.last(),
                Some(AgentEvent::TurnEnded {
                    reason: StopReason::Cancelled,
                    ..
                })
            ),
            "the stream ended before the cancel arrived"
        );
    })
    .await;
}

/// A side question's context (ADR 0045) is refused, typed, before anything
/// is sent, until the driver carries it (A2's next commit): never dropped.
/// The session goes on and its next prompt is a turn of its own.
#[tokio::test]
async fn a_side_questions_context_is_refused_not_dropped() {
    within(async {
        let (_agent, input, mut outputs) = spawn(setup(None));
        let _handshake = ready(&mut outputs).await;
        let context = vessel_agent::PromptContext::new("Earlier: the plan".to_owned()).unwrap();
        let refused = input
            .prompt_context("plain what now?".parse().unwrap(), context)
            .await;
        assert!(
            matches!(refused, Err(InputError::ContextNotTaken)),
            "{refused:?}"
        );
        input.prompt(text("plain hello")).await.unwrap();
        let events = turn(&mut outputs).await;
        assert_eq!(
            events[0],
            AgentEvent::TurnStarted {
                content: text("plain hello")
            },
            "no turn started for the refused question"
        );
    })
    .await;
}
