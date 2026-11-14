//! One session over Claude Code's stream-json: the control handshake,
//! turns, cancelling, and what the agent asks (slice 1 of the plan's
//! Claude Code section).
//!
//! [Decision Log]
//! - Purpose: the session actors see what the ACP driver gives them, in
//!   the same order: `Introduced`, `Options`, `Ready`; a prompt answered
//!   once sent, with `TurnStarted`; the turn's events; its `TurnEnded` or
//!   `TurnFailed`; `Disconnected` last.
//! - Handshake: the control protocol's `initialize`, the Agent SDK's own
//!   first request, answered before any prompt, within the setup's
//!   handshake time. The session is ready with the conversation its
//!   launch named (`Launched`): Claude Code writes its `init` frame only
//!   once a turn starts. A CLI that does not answer `initialize` fails the
//!   handshake with that reason; the fallback to the documented MCP
//!   permission tool for exactly that case is slice 2's.
//! - One loop owns the state and never awaits an answer in place; the
//!   agent's frames and the client's commands arrive through channels.
//!   Commands are polled first, as in the ACP driver: the agent can stream
//!   faster than the loop takes frames, and a cancel must not wait for
//!   the stream to pause. Taking a command before frames already queued is
//!   safe: a turn ends only at its `result`, handled in order with its
//!   frames.
//! - The floor (ADR 0057): every tool request is answered (denied, until
//!   the permissions slice turns them into permission requests, with a
//!   line in the session's log); a control request the driver does not
//!   serve or cannot read is refused and the running turn is stopped and
//!   ends failed, saying why. Nothing is ever allowed by default, and
//!   nothing the agent asks is left waiting.
//! - A turn the driver ends itself (a refusal) still ends at its `result`
//!   (the CLI writes one after the interrupt), so the next prompt never
//!   meets the previous turn's end; the failure replaces what the result
//!   would have said.

use std::collections::HashSet;
use std::time::Duration;

use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::mpsc;
use vessel_proto::{AgentAbilities, AgentEvent, ContentBlock, HarnessSessionId, OptionChoice};

use super::io::{self, Ended, Read, Writer};
use super::launch::Launched;
use super::map::Translator;
use super::wire::{
    AgentRequest, Answer, ControlRequest, ControlResponse, Frame, HostAnswer, HostRequest,
    Outgoing, OutgoingBlock, OutgoingMessage, Permission, System, TurnResult,
};
use crate::error::{Disconnect, InputError};
use crate::handshake::AgentSetup;
use crate::input::{Command, Reply};
use crate::introduction::Introduction;
use crate::options::OptionError;
use crate::output::AgentOutput;

/// Frames read ahead of the loop.
const INBOUND: usize = 64;

/// What the session can do over this driver, as its handshake says.
const ABILITIES: AgentAbilities = AgentAbilities {
    steer: false,
    resume: true,
    image: false,
    audio: false,
    fork: false,
};

/// Where the session's turn is.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Turn {
    Idle,
    Running {
        /// The client asked to stop it.
        cancelled: bool,
        /// The driver stopped it, for this reason.
        failure: Option<String>,
    },
}

/// Drive one session over Claude Code's stdio.
pub(super) async fn run(
    stdin: ChildStdin,
    stdout: ChildStdout,
    commands: mpsc::Receiver<Command>,
    outputs: mpsc::Sender<AgentOutput>,
    setup: AgentSetup,
    launched: Launched,
) {
    let (lines, incoming) = mpsc::channel(INBOUND);
    drop(tokio::spawn(io::read(stdout, lines)));
    let mut session = Session {
        out: Writer::start(stdin),
        outputs,
        handshake_timeout: setup.handshake_timeout,
        conversation: launched.conversation.clone(),
        _launched: launched,
        translator: Translator::default(),
        next: 0,
        initialize: None,
        interrupts: HashSet::new(),
        ready: false,
        turn: Turn::Idle,
    };
    let end = session.drive(incoming, commands).await;
    session.close(end).await;
}

struct Session {
    out: Writer,
    outputs: mpsc::Sender<AgentOutput>,
    handshake_timeout: Duration,
    /// The conversation, as the launch named it.
    conversation: HarnessSessionId,
    /// Kept for the session's life: its MCP file goes with it.
    _launched: Launched,
    translator: Translator,
    /// The next request id's number.
    next: u64,
    /// `initialize`'s id while it is unanswered.
    initialize: Option<String>,
    /// Interrupts sent and not answered.
    interrupts: HashSet<String>,
    ready: bool,
    turn: Turn,
}

impl Session {
    /// Run until the connection ends; why it did.
    async fn drive(
        &mut self,
        mut incoming: mpsc::Receiver<Result<Read, Ended>>,
        mut commands: mpsc::Receiver<Command>,
    ) -> Disconnect {
        let Some(id) = self.request(HostRequest::Initialize {}).await else {
            return handshake_failed("Claude Code closed its input".to_owned());
        };
        self.initialize = Some(id);
        let handshake = tokio::time::sleep(self.handshake_timeout);
        tokio::pin!(handshake);
        let mut commanding = true;
        loop {
            tokio::select! {
                biased;
                command = commands.recv(), if self.ready && commanding => match command {
                    Some(command) => self.command(command).await,
                    // Nobody sends any more; the agent's output is still
                    // read until its owner stops it.
                    None => commanding = false,
                },
                item = incoming.recv() => match item {
                    Some(Ok(read)) => {
                        if let Some(end) = self.read(read).await {
                            return end;
                        }
                    }
                    Some(Err(Ended::Protocol(message))) => return self.failed(message),
                    Some(Err(Ended::Closed)) | None => {
                        return if self.ready {
                            Disconnect::Closed
                        } else {
                            handshake_failed("Claude Code exited before it was ready".to_owned())
                        };
                    }
                },
                () = &mut handshake, if !self.ready => {
                    return handshake_failed(format!(
                        "Claude Code did not answer the control protocol's initialize within {} s",
                        self.handshake_timeout.as_secs()
                    ));
                }
            }
        }
    }

    /// The connection failed for `message`.
    fn failed(&self, message: String) -> Disconnect {
        if self.ready {
            Disconnect::Protocol { message }
        } else {
            handshake_failed(message)
        }
    }

    /// Settle what is open, then say why the connection ended.
    async fn close(&mut self, end: Disconnect) {
        if self.turn != Turn::Idle {
            self.turn = Turn::Idle;
            self.emit(AgentEvent::TurnFailed {
                message: "the agent exited during the turn".to_owned(),
            })
            .await;
        }
        self.output(AgentOutput::Disconnected(end)).await;
    }

    async fn read(&mut self, read: Read) -> Option<Disconnect> {
        match read {
            Read::Frame(frame) => return self.frame(*frame).await,
            Read::UnreadableRequest { request_id, error } => {
                tracing::warn!(%error, "a Claude Code control request that does not read, refused");
                if let Some(request_id) = request_id {
                    self.refuse(request_id, "Vessel could not read this request")
                        .await;
                }
                self.stop_turn(format!(
                    "Claude Code asked Vessel something it could not read ({error}); it was \
                     refused and the turn stopped"
                ))
                .await;
            }
            Read::UnreadableResult(error) => {
                if self.turn != Turn::Idle {
                    self.turn = Turn::Idle;
                    self.emit(AgentEvent::TurnFailed {
                        message: format!("Claude Code's end of the turn did not read: {error}"),
                    })
                    .await;
                }
            }
        }
        None
    }

    async fn frame(&mut self, frame: Frame) -> Option<Disconnect> {
        match frame {
            Frame::System(System::Init(init)) => {
                if init.session_id != self.conversation.as_str() {
                    tracing::warn!(
                        launched = %self.conversation,
                        reported = %init.session_id,
                        "Claude Code reports another conversation than the one it was launched with"
                    );
                }
            }
            Frame::StreamEvent(event) => {
                let events = self.translator.stream(&event);
                self.emit_all(events).await;
            }
            Frame::Assistant(message) => {
                let events = self.translator.assistant(&message);
                self.emit_all(events).await;
            }
            Frame::User(message) => {
                let events = self.translator.user(&message);
                self.emit_all(events).await;
            }
            Frame::Result(result) => self.ended(&result).await,
            Frame::ControlRequest(request) => self.asked(request).await,
            Frame::ControlResponse(ControlResponse { response }) => {
                return self.answered(response).await;
            }
            // Nothing the driver asks waits for a withdrawal yet: every
            // request is answered as it arrives.
            Frame::ControlCancelRequest(_)
            | Frame::System(_)
            | Frame::KeepAlive
            | Frame::Unknown => {}
        }
        None
    }

    /// A turn's `result`.
    async fn ended(&mut self, result: &TurnResult) {
        let Turn::Running { cancelled, failure } = std::mem::replace(&mut self.turn, Turn::Idle)
        else {
            tracing::debug!("a result outside a turn of the session's");
            return;
        };
        let event = match failure {
            Some(message) => AgentEvent::TurnFailed { message },
            None => Translator::end(result, cancelled),
        };
        self.emit(event).await;
    }

    /// A request of the agent's. Tool requests are denied until the
    /// permissions slice; anything else is refused and stops the turn.
    async fn asked(&mut self, request: ControlRequest) {
        let ControlRequest {
            request_id,
            request,
        } = request;
        match request {
            AgentRequest::CanUseTool(asked) => {
                let denied = Permission::Deny {
                    message: "Vessel's Claude Code driver does not ask for permissions yet"
                        .to_owned(),
                    tool_use_id: asked.tool_use_id,
                };
                let answer = match serde_json::to_value(denied) {
                    Ok(value) => HostAnswer::Success {
                        request_id,
                        response: value,
                    },
                    Err(error) => HostAnswer::Error {
                        request_id,
                        error: error.to_string(),
                    },
                };
                let _sent = self
                    .out
                    .send(&Outgoing::ControlResponse { response: answer })
                    .await;
                tracing::info!(tool = %asked.tool_name, "Claude Code's tool request denied");
                self.emit(AgentEvent::HarnessLog {
                    line: format!(
                        "Vessel declined Claude Code's request to use {}: this driver does not \
                         ask for permissions yet",
                        asked.tool_name
                    ),
                })
                .await;
            }
            AgentRequest::Unknown => {
                self.refuse(request_id, "Vessel does not serve this request")
                    .await;
                self.stop_turn(
                    "Claude Code asked Vessel for something this driver does not serve; it was \
                     refused and the turn stopped"
                        .to_owned(),
                )
                .await;
            }
        }
    }

    /// The answer to one of the driver's requests.
    async fn answered(&mut self, answer: Answer) -> Option<Disconnect> {
        let (request_id, refusal) = match answer {
            Answer::Success { request_id, .. } => (request_id, None),
            Answer::Error { request_id, error } => (request_id, Some(error)),
        };
        if self.initialize.as_deref() == Some(request_id.as_str()) {
            self.initialize = None;
            if let Some(error) = refusal {
                return Some(handshake_failed(format!(
                    "Claude Code refused the control protocol's initialize: {error}"
                )));
            }
            self.ready().await;
        } else if self.interrupts.remove(&request_id) {
            if let Some(error) = refusal {
                tracing::info!(%error, "Claude Code refused to interrupt the turn");
            }
        } else {
            tracing::debug!(request_id, "an answer to no request of the driver's");
        }
        None
    }

    /// The handshake is done: say who the agent is, its options, ready.
    async fn ready(&mut self) {
        self.ready = true;
        let introduction = Introduction {
            version: None,
            methods: Vec::new(),
        };
        self.output(AgentOutput::Introduced(Box::new(introduction)))
            .await;
        // No options yet: they are slice 3's.
        self.output(AgentOutput::Options(Vec::new())).await;
        self.emit(AgentEvent::Ready {
            harness_session: self.conversation.clone(),
            abilities: Some(ABILITIES),
        })
        .await;
    }

    async fn command(&mut self, command: Command) {
        match command {
            Command::Prompt { content, reply, .. } => self.prompt(content, reply).await,
            Command::CancelTurn { reply } => {
                self.cancel().await;
                answer(reply, Ok(()));
            }
            Command::Steer { reply, .. } => {
                let _gone = reply.send(Err(InputError::CannotSteer));
            }
            Command::Answer { id, reply, .. } => {
                answer(reply, Err(InputError::PermissionNotFound(id)));
            }
            Command::AnswerQuestion { id, reply, .. } => {
                answer(reply, Err(InputError::QuestionNotFound(id)));
            }
            Command::TurnRunning { reply } => {
                let _gone = reply.send(self.turn != Turn::Idle);
            }
            Command::Options { reply } => {
                let _gone = reply.send(Vec::new());
            }
            Command::SetOption { choice, reply } => {
                let _gone = reply.send(Err(InputError::Option(unknown(choice))));
            }
        }
    }

    async fn prompt(&mut self, content: Vec<ContentBlock>, reply: Reply) {
        if self.turn != Turn::Idle {
            answer(reply, Err(InputError::TurnInProgress));
            return;
        }
        let blocks = match prompt_blocks(&content) {
            Ok(blocks) => blocks,
            Err(error) => {
                answer(reply, Err(error));
                return;
            }
        };
        let frame = Outgoing::User {
            message: OutgoingMessage {
                role: "user",
                content: blocks,
            },
            parent_tool_use_id: None,
            session_id: self.conversation.to_string(),
        };
        if !self.out.send(&frame).await {
            answer(reply, Err(InputError::Closed));
            return;
        }
        self.turn = Turn::Running {
            cancelled: false,
            failure: None,
        };
        self.emit(AgentEvent::TurnStarted { content }).await;
        answer(reply, Ok(()));
    }

    /// The client stops the running turn (a no-op without one).
    async fn cancel(&mut self) {
        let Turn::Running { cancelled, .. } = &mut self.turn else {
            return;
        };
        if *cancelled {
            return;
        }
        *cancelled = true;
        self.interrupt().await;
    }

    /// The driver stops the running turn, which then ends failed for
    /// `reason` (a no-op without one: the refusal was answered).
    async fn stop_turn(&mut self, reason: String) {
        let Turn::Running { failure, cancelled } = &mut self.turn else {
            return;
        };
        if failure.is_some() {
            return;
        }
        *failure = Some(reason);
        if !*cancelled {
            *cancelled = true;
            self.interrupt().await;
        }
    }

    async fn interrupt(&mut self) {
        if let Some(id) = self.request(HostRequest::Interrupt {}).await {
            let _new = self.interrupts.insert(id);
        }
    }

    /// Refuse the agent's request `request_id`.
    async fn refuse(&mut self, request_id: String, why: &str) {
        let answer = HostAnswer::Error {
            request_id,
            error: why.to_owned(),
        };
        let _sent = self
            .out
            .send(&Outgoing::ControlResponse { response: answer })
            .await;
    }

    /// Send a request of the driver's; its id, or `None` once the agent's
    /// input is closed.
    async fn request(&mut self, request: HostRequest) -> Option<String> {
        self.next = self.next.wrapping_add(1);
        let request_id = format!("vessel-{}", self.next);
        let frame = Outgoing::ControlRequest {
            request_id: request_id.clone(),
            request,
        };
        self.out.send(&frame).await.then_some(request_id)
    }

    async fn emit_all(&self, events: Vec<AgentEvent>) {
        for event in events {
            self.emit(event).await;
        }
    }

    async fn emit(&self, event: AgentEvent) {
        self.output(AgentOutput::Event(event)).await;
    }

    async fn output(&self, output: AgentOutput) {
        // The consumer gone means the session is over; the owner stops the
        // agent.
        let _gone = self.outputs.send(output).await;
    }
}

/// A prompt's content as Claude Code's message: text as text, a link as
/// its text.
///
/// # Errors
/// [`InputError::AttachmentNotTaken`] for an attachment (this driver takes
/// none yet), [`InputError::InvalidContent`] for an opaque block.
fn prompt_blocks(content: &[ContentBlock]) -> Result<Vec<OutgoingBlock>, InputError> {
    content
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => Ok(OutgoingBlock::Text { text: text.clone() }),
            ContentBlock::ResourceLink { uri, name } => Ok(OutgoingBlock::Text {
                text: format!("[{name}]({uri})"),
            }),
            ContentBlock::Attachment { attachment } => Err(InputError::AttachmentNotTaken {
                id: attachment.id,
                kind: attachment.kind,
            }),
            ContentBlock::Opaque { .. } => Err(InputError::InvalidContent(
                serde::de::Error::custom("Claude Code's driver takes text and links"),
            )),
        })
        .collect()
}

fn handshake_failed(message: String) -> Disconnect {
    Disconnect::Handshake {
        message,
        refused: None,
        fork_refused: None,
    }
}

fn unknown(choice: OptionChoice) -> OptionError {
    OptionError::Unknown {
        id: choice.id,
        offered: Vec::new(),
    }
}

fn answer(reply: Reply, result: Result<(), InputError>) {
    if reply.send(result).is_err() {
        tracing::debug!("agent command reply dropped");
    }
}

#[cfg(test)]
mod tests;
