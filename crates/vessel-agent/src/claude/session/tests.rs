//! A session over `fake-claude-stream`: the driver's launch and loop
//! driven directly (the driver is not `BUILT`, so `AgentProcess` refuses
//! it), with the fake's frames shaped as the Agent SDK declares them.

use std::process::Stdio;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use vessel_proc::CommandSpec;
use vessel_proto::{
    AgentEvent, AttachmentId, AttachmentKind, AttachmentRef, ContentBlock, HarnessSessionId,
    OptionChoice, OptionId, OptionValueId, RawJson, StopReason, ToolCallContent, ToolCallStatus,
};
use vessel_testkit::Fake;

use crate::attached::Attachments;
use crate::error::{Disconnect, InputError};
use crate::handshake::AgentSetup;
use crate::input::Command;
use crate::output::AgentOutput;

/// Longest any one of these tests may take.
const SAFETY_NET: Duration = Duration::from_secs(30);

async fn within<F: Future>(test: F) -> F::Output {
    vessel_testkit::within(SAFETY_NET, Box::pin(test))
        .await
        .expect("test timed out")
}

fn setup() -> AgentSetup {
    AgentSetup {
        cwd: std::env::temp_dir(),
        model: None,
        resume: None,
        fork: None,
        options: Vec::new(),
        mcp_servers: vessel_proto::ResolvedMcpServers::new(),
        handshake_timeout: Duration::from_secs(20),
        questions: false,
        system_prompt: None,
        private_dir: None,
    }
}

/// The command `vessel_harness::launch_direct` gives the driver, running
/// the fake; `env` for the fake's own switches.
fn command(env: &[(&str, &str)]) -> CommandSpec {
    let mut command = CommandSpec::new(Fake::ClaudeStream.path().unwrap()).args([
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
    ]);
    for (name, value) in env {
        command = command.env(*name, *value);
    }
    command
}

struct Running {
    commands: mpsc::Sender<Command>,
    outputs: mpsc::Receiver<AgentOutput>,
    _child: tokio::process::Child,
}

/// Launch the fake as the driver would and run the session loop on it.
fn start(setup: AgentSetup, env: &[(&str, &str)]) -> Running {
    let (shaped, launched) = super::super::launch(&command(env), &setup).unwrap();
    let mut child = tokio::process::Command::from(shaped.to_command())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (commands, commands_in) = mpsc::channel(16);
    let (outputs_out, outputs) = mpsc::channel(64);
    drop(tokio::spawn(super::run(
        stdin,
        stdout,
        commands_in,
        outputs_out,
        setup,
        launched,
    )));
    Running {
        commands,
        outputs,
        _child: child,
    }
}

fn text(text: &str) -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        text: text.to_owned(),
    }]
}

impl Running {
    async fn next(&mut self) -> AgentOutput {
        self.outputs
            .recv()
            .await
            .expect("the session ended without saying why")
    }

    /// Outputs up to and including `Ready`.
    async fn ready(&mut self) -> Vec<AgentOutput> {
        let mut outputs = Vec::new();
        loop {
            let output = self.next().await;
            let ready = matches!(output, AgentOutput::Event(AgentEvent::Ready { .. }));
            if let AgentOutput::Disconnected(end) = &output {
                panic!("not ready: {end:?}");
            }
            outputs.push(output);
            if ready {
                return outputs;
            }
        }
    }

    async fn prompt(&self, content: Vec<ContentBlock>) -> Result<(), InputError> {
        self.prompt_with(content, None).await
    }

    /// A prompt with a side question's context, as `AgentInput::prompt_context`
    /// sends it.
    async fn prompt_with(
        &self,
        content: Vec<ContentBlock>,
        side: Option<crate::PromptContext>,
    ) -> Result<(), InputError> {
        let (reply, answer) = oneshot::channel();
        self.commands
            .send(Command::Prompt {
                content,
                attachments: Attachments::new(),
                context: side,
                reply,
            })
            .await
            .unwrap();
        answer.await.unwrap()
    }

    async fn cancel(&self) {
        let (reply, answer) = oneshot::channel();
        self.commands
            .send(Command::CancelTurn { reply })
            .await
            .unwrap();
        answer.await.unwrap().unwrap();
    }

    /// The events of a turn, through its end.
    async fn turn(&mut self) -> Vec<AgentEvent> {
        let mut events = Vec::new();
        loop {
            match self.next().await {
                AgentOutput::Event(event) => {
                    let end = matches!(
                        event,
                        AgentEvent::TurnEnded { .. } | AgentEvent::TurnFailed { .. }
                    );
                    events.push(event);
                    if end {
                        return events;
                    }
                }
                AgentOutput::Disconnected(end) => panic!("disconnected in a turn: {end:?}"),
                AgentOutput::Options(_)
                | AgentOutput::Commands(_)
                | AgentOutput::Usage(_)
                | AgentOutput::Introduced(_) => {}
            }
        }
    }

    /// The connection's end, skipping what comes before it.
    async fn disconnected(&mut self) -> Disconnect {
        loop {
            if let AgentOutput::Disconnected(end) = self.next().await {
                return end;
            }
        }
    }
}

/// The text of the chunks of `kind` in `events`.
fn said(events: &[AgentEvent], thought: bool) -> String {
    events
        .iter()
        .filter_map(|event| {
            if let AgentEvent::MessageChunk {
                content: ContentBlock::Text { text },
            } = event
                && !thought
            {
                return Some(text.as_str());
            }
            if let AgentEvent::ThoughtChunk {
                content: ContentBlock::Text { text },
            } = event
                && thought
            {
                return Some(text.as_str());
            }
            None
        })
        .collect()
}

#[tokio::test]
async fn a_session_is_ready_with_its_conversation_and_a_turn_streams_its_reply() {
    within(async {
        let mut session = start(setup(), &[]);
        let ready = session.ready().await;
        assert!(matches!(ready[0], AgentOutput::Introduced(_)), "{ready:?}");
        assert_eq!(ready[1], AgentOutput::Options(Vec::new()));
        let AgentOutput::Event(AgentEvent::Ready {
            harness_session,
            abilities: Some(abilities),
        }) = &ready[2]
        else {
            panic!("{ready:?}");
        };
        assert!(uuid::Uuid::parse_str(harness_session.as_str()).is_ok());
        assert!(abilities.resume && !abilities.steer && !abilities.fork);

        session.prompt(text("hello")).await.unwrap();
        let events = session.turn().await;
        assert_eq!(
            events[0],
            AgentEvent::TurnStarted {
                content: text("hello")
            }
        );
        assert_eq!(
            said(&events, false),
            "Hello there",
            "streamed once, not repeated"
        );
        let Some(AgentEvent::TurnEnded {
            reason: StopReason::EndTurn,
            usage: Some(usage),
            ..
        }) = events.last()
        else {
            panic!("{events:?}");
        };
        assert_eq!(usage.total.get(), 5);

        // The next turn starts from the end of this one.
        session.prompt(text("think")).await.unwrap();
        let events = session.turn().await;
        assert_eq!(said(&events, true), "Look first.");
        assert_eq!(said(&events, false), "Thought about it.");
    })
    .await;
}

#[tokio::test]
async fn a_resumed_session_is_ready_with_the_conversation_it_resumes() {
    within(async {
        let id = "5d0c3c1e-8f3a-4e8e-9c55-2f0f3a1b7c42";
        let mut session = start(
            AgentSetup {
                resume: Some(HarnessSessionId::new(id)),
                ..setup()
            },
            &[],
        );
        let ready = session.ready().await;
        assert!(ready.iter().any(|output| matches!(
            output,
            AgentOutput::Event(AgentEvent::Ready { harness_session, .. }) if harness_session.as_str() == id
        )));
        session.prompt(text("hello")).await.unwrap();
        assert!(matches!(
            session.turn().await.last(),
            Some(AgentEvent::TurnEnded { .. })
        ));
    })
    .await;
}

/// A session parked before its first turn is woken with `--resume` of a
/// conversation Claude Code has not written yet (it writes one once a turn
/// starts). Whatever the real CLI does then is to be verified on a real
/// turn; the driver's part is to fail the handshake at once, naming the
/// resume, so the wake fails (typed, the session stays parked to be tried
/// again) instead of waiting for the handshake's time.
#[tokio::test]
async fn a_resume_of_a_conversation_never_written_fails_the_handshake_at_once() {
    within(async {
        let id = "5d0c3c1e-8f3a-4e8e-9c55-2f0f3a1b7c42";
        let mut session = start(
            AgentSetup {
                resume: Some(HarnessSessionId::new(id)),
                ..setup()
            },
            &[("FAKE_CLAUDE_NO_TRANSCRIPTS", "1")],
        );
        let started = std::time::Instant::now();
        assert!(matches!(
            session.disconnected().await,
            Disconnect::Handshake { message, .. }
                if message.contains("resuming conversation") && message.contains(id)
        ));
        assert!(
            started.elapsed() < setup().handshake_timeout,
            "not the handshake's time running out"
        );
    })
    .await;
}

/// A side question's context (ADR 0045) reaches the agent first, the
/// timeline shows the question alone, and nothing of the context is kept:
/// the next prompt carries none. Past the request bound the prompt is
/// refused before anything is sent, no turn starts, and the session goes
/// on as before.
#[tokio::test]
async fn a_side_questions_context_reaches_the_agent_first_and_is_never_kept() {
    within(async {
        let mut session = start(setup(), &[]);
        let _ready = session.ready().await;
        let context = crate::PromptContext::new("Earlier: the plan".to_owned()).unwrap();
        session
            .prompt_with(text("echo what now?"), Some(context))
            .await
            .unwrap();
        let events = session.turn().await;
        assert_eq!(
            events[0],
            AgentEvent::TurnStarted {
                content: text("echo what now?")
            },
            "the timeline shows the question alone"
        );
        assert_eq!(said(&events, false), "Earlier: the plan | echo what now?");

        session.prompt(text("echo again")).await.unwrap();
        assert_eq!(said(&session.turn().await, false), "echo again");

        let large =
            crate::PromptContext::new("x".repeat(vessel_proto::side_questions::SNAPSHOT_BYTES))
                .unwrap();
        let long_question = format!("echo {}", "y".repeat(30 * 1024));
        assert!(matches!(
            session.prompt_with(text(&long_question), Some(large)).await,
            Err(InputError::ContextLimit)
        ));
        session.prompt(text("echo after")).await.unwrap();
        let events = session.turn().await;
        assert_eq!(
            events[0],
            AgentEvent::TurnStarted {
                content: text("echo after")
            },
            "no turn started for the refused question"
        );
        assert_eq!(said(&events, false), "echo after");
    })
    .await;
}

#[tokio::test]
async fn a_tool_request_is_denied_and_the_log_says_why() {
    within(async {
        let mut session = start(setup(), &[]);
        let _ready = session.ready().await;
        session.prompt(text("tool")).await.unwrap();
        let events = session.turn().await;
        let call = events
            .iter()
            .find_map(|event| {
                if let AgentEvent::ToolCall(call) = event {
                    Some(call)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(call.title, "Bash: ls");
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::HarnessLog { line } if line.contains("declined") && line.contains("Bash")
        )));
        let settled = events
            .iter()
            .find_map(|event| {
                if let AgentEvent::ToolCallUpdate(update) = event {
                    Some(update)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(settled.id, call.id);
        assert_eq!(settled.status, Some(ToolCallStatus::Failed));
        let Some(content) = &settled.content else {
            panic!("{settled:?}");
        };
        assert!(matches!(
            &content[0],
            ToolCallContent::Content { content: ContentBlock::Text { text } }
                if text.contains("does not ask for permissions yet")
        ));
        assert_eq!(said(&events, false), "Done.");
        assert!(matches!(events.last(), Some(AgentEvent::TurnEnded { .. })));
    })
    .await;
}

#[tokio::test]
async fn a_cancel_interrupts_the_turn_and_it_ends_cancelled() {
    within(async {
        let mut session = start(setup(), &[]);
        let _ready = session.ready().await;
        session.prompt(text("wait")).await.unwrap();
        // The turn is under way: its first words arrived.
        loop {
            if let AgentOutput::Event(AgentEvent::MessageChunk { .. }) = session.next().await {
                break;
            }
        }
        assert!(matches!(
            session.prompt(text("hello")).await,
            Err(InputError::TurnInProgress)
        ));
        session.cancel().await;
        let events = session.turn().await;
        assert!(matches!(
            events.last(),
            Some(AgentEvent::TurnEnded {
                reason: StopReason::Cancelled,
                ..
            })
        ));
        // A cancel with no turn is a no-op.
        session.cancel().await;
        session.prompt(text("hello")).await.unwrap();
        assert!(matches!(
            session.turn().await.last(),
            Some(AgentEvent::TurnEnded { .. })
        ));
    })
    .await;
}

#[tokio::test]
async fn a_failed_turn_ends_in_the_agents_words() {
    within(async {
        let mut session = start(setup(), &[]);
        let _ready = session.ready().await;
        for (prompt, words) in [("fail", "boom"), ("limit", "You've hit your usage limit")] {
            session.prompt(text(prompt)).await.unwrap();
            assert_eq!(
                session.turn().await.last(),
                Some(&AgentEvent::TurnFailed {
                    message: words.to_owned()
                })
            );
        }
    })
    .await;
}

#[tokio::test]
async fn what_the_driver_does_not_serve_or_cannot_read_is_refused_and_stops_the_turn() {
    within(async {
        let mut session = start(setup(), &[]);
        let _ready = session.ready().await;
        for (prompt, words) in [("ask", "does not serve"), ("garbled", "could not read")] {
            session.prompt(text(prompt)).await.unwrap();
            let events = session.turn().await;
            assert!(
                matches!(
                    events.last(),
                    Some(AgentEvent::TurnFailed { message }) if message.contains(words)
                ),
                "{prompt}: {events:?}"
            );
        }
        // The session goes on.
        session.prompt(text("hello")).await.unwrap();
        assert!(matches!(
            session.turn().await.last(),
            Some(AgentEvent::TurnEnded { .. })
        ));
    })
    .await;
}

#[tokio::test]
async fn an_agent_that_exits_in_a_turn_fails_it_then_the_session_ends() {
    within(async {
        let mut session = start(setup(), &[]);
        let _ready = session.ready().await;
        session.prompt(text("exit")).await.unwrap();
        let events = session.turn().await;
        assert!(matches!(
            events.last(),
            Some(AgentEvent::TurnFailed { message }) if message.contains("exited")
        ));
        assert_eq!(session.disconnected().await, Disconnect::Closed);
    })
    .await;
}

#[tokio::test]
async fn a_line_that_is_not_a_frame_ends_the_connection() {
    within(async {
        let mut session = start(setup(), &[]);
        let _ready = session.ready().await;
        session.prompt(text("garbage")).await.unwrap();
        assert!(matches!(
            session.disconnected().await,
            Disconnect::Protocol { message } if message.contains("not JSON")
        ));
    })
    .await;
}

#[tokio::test]
async fn a_cli_without_the_control_protocol_fails_the_handshake() {
    within(async {
        let mut refused = start(setup(), &[("FAKE_CLAUDE_INITIALIZE", "refuse")]);
        assert!(matches!(
            refused.disconnected().await,
            Disconnect::Handshake { message, .. } if message.contains("refused the control protocol")
        ));
        let mut silent = start(
            AgentSetup {
                handshake_timeout: Duration::from_millis(300),
                ..setup()
            },
            &[("FAKE_CLAUDE_INITIALIZE", "silent")],
        );
        assert!(matches!(
            silent.disconnected().await,
            Disconnect::Handshake { message, .. } if message.contains("did not answer")
        ));
    })
    .await;
}

#[tokio::test]
async fn what_this_slice_does_not_offer_is_answered_as_such() {
    within(async {
        let mut session = start(setup(), &[]);
        let _ready = session.ready().await;
        let (reply, answer) = oneshot::channel();
        session
            .commands
            .send(Command::Steer {
                content: text("also"),
                attachments: Attachments::new(),
                reply,
            })
            .await
            .unwrap();
        assert!(matches!(
            answer.await.unwrap(),
            Err(InputError::CannotSteer)
        ));

        let (reply, answer) = oneshot::channel();
        session
            .commands
            .send(Command::SetOption {
                choice: OptionChoice {
                    id: OptionId::new("mode"),
                    value: vessel_proto::ChoiceValue::Select {
                        value: OptionValueId::new("plan"),
                    },
                },
                reply,
            })
            .await
            .unwrap();
        assert!(matches!(answer.await.unwrap(), Err(InputError::Option(_))));

        let attached = vec![ContentBlock::Attachment {
            attachment: AttachmentRef {
                id: AttachmentId::random(),
                name: "a.png".parse().unwrap(),
                kind: AttachmentKind::Image,
                mime: "image/png".parse().unwrap(),
                size: 4,
            },
        }];
        assert!(matches!(
            session.prompt(attached).await,
            Err(InputError::AttachmentNotTaken { .. })
        ));
        let opaque = vec![ContentBlock::Opaque {
            raw: RawJson::from_text("{\"type\":\"image\"}".to_owned()).unwrap(),
        }];
        assert!(matches!(
            session.prompt(opaque).await,
            Err(InputError::InvalidContent(_))
        ));
        // Nothing was sent: the session still takes a prompt.
        session.prompt(text("hello")).await.unwrap();
        assert!(matches!(
            session.turn().await.last(),
            Some(AgentEvent::TurnEnded { .. })
        ));
    })
    .await;
}
