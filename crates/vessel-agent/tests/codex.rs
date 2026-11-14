//! The Codex app-server driver (ADR 0057, Codex slice 1) against
//! `fake-codex-app-server`, whose messages follow Codex 0.166.0's schema.
//!
//! Tests wait on outputs, never on time; the timeout only turns a hang into
//! a failure.

use vessel_agent::{
    AgentOutput, Disconnect, Driver, ForkFrom, ForkRefusal, InputError, SteerOutcome,
};
use vessel_proto::{
    AgentEvent, ContentBlock, ContextUsage, HarnessSessionId, PlanStatus, StopReason, TokenCount,
    ToolCallContent, ToolCallId, ToolCallStatus, ToolKind,
};

use support::{command, events, ready, said, setup, spawn, text, turn, until, within};

#[cfg(test)]
mod support {
    use std::future::Future;
    use std::time::Duration;

    use vessel_agent::{
        AgentInput, AgentOutput, AgentOutputs, AgentProcess, AgentSetup, CommandSpec, Driver,
    };
    use vessel_proto::{AgentEvent, ContentBlock, HarnessSessionId};
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
}

/// The handshake says who the agent is, its (no) options, then ready on a
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
        assert_eq!(handshake[1], AgentOutput::Options(Vec::new()));
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

/// Until the permissions slice, an approval is declined: the command ends
/// failed, and the session's log says why.
#[tokio::test]
async fn an_approval_is_declined_and_the_log_says_why() {
    within(async {
        let (_agent, input, mut outputs) = spawn(setup(None));
        let _ready = ready(&mut outputs).await;
        input.prompt(text("approve")).await.unwrap();
        let events = turn(&mut outputs).await;
        assert!(
            events.iter().any(|event| matches!(
                event,
                AgentEvent::HarnessLog { line } if line.contains("declined Codex's request to run a command")
            )),
            "{events:?}"
        );
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::ToolCallUpdate(update) if update.status == Some(ToolCallStatus::Failed)
        )));
        assert_eq!(said(&events), "decision decline");
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

/// A probe reads the agent's introduction and abilities and stops it.
#[tokio::test]
async fn a_probe_reads_the_introduction() {
    within(async {
        let probed = vessel_agent::probe_with(Driver::CodexAppServer, &command(), setup(None))
            .await
            .unwrap();
        assert!(probed.options.is_empty());
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
    })
    .await;
}

/// Until the options slice, a session asked for a choice is not started
/// without it: the handshake fails as for an option an agent does not
/// offer. Tool servers are said to be left out.
#[tokio::test]
async fn choices_fail_the_handshake_and_tool_servers_are_said_to_be_left_out() {
    within(async {
        let mut choosing = setup(None);
        choosing.options = vec![vessel_proto::OptionChoice {
            id: vessel_proto::OptionId::new("mode"),
            value: vessel_proto::ChoiceValue::Select {
                value: vessel_proto::OptionValueId::new("read-only"),
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
                    if message.contains("mode")
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
