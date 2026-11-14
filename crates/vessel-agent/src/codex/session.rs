//! One session over Codex's app-server: the handshake, turns, steering and
//! cancelling, and what the agent asks (slice 1 of the plan's Codex
//! section).
//!
//! [Decision Log]
//! - Purpose: the session actors see what the ACP driver gives them, in
//!   the same order: `Introduced`, then `Options`, then `Ready`; a prompt
//!   answered once sent, with `TurnStarted`; a turn's events; its
//!   `TurnEnded` or `TurnFailed`; `Disconnected` last.
//! - Choice: one loop owns the state and never awaits a response in
//!   place. A request is written with an id and what to do with its answer
//!   (`Pending`); the answer arrives through the same ordered stream as the
//!   agent's notifications. Commands are taken only once the session is
//!   ready, so those sent during the handshake wait for it (as with ACP).
//! - Turn ids: `turn/start` answers with the turn's id, and `turn/started`
//!   says it too; whichever comes first is kept. A cancel before either
//!   is remembered and sent as soon as the id is known.
//! - Approvals: declined, each with a line in the session's log, until the
//!   permissions slice turns them into permission requests. That is why
//!   the node cannot select this driver yet (`vessel_harness::direct`).
//!   Requests the driver does not serve are answered with an error, never
//!   left waiting.
//! - Other threads' notifications (Codex's sub-agents run threads of their
//!   own) are not this session's and are ignored here.

use std::collections::{HashMap, HashSet};

use serde_json::Value;
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::mpsc;
use vessel_proto::agents::AgentVersion;
use vessel_proto::{
    AgentAbilities, AgentEvent, ContentBlock, ContextUsage, HarnessSessionId, OptionChoice,
};

use super::mapper;
use super::protocol::{
    AskForApproval, ClientInfo, Declined, DeltaNotification, ElicitationDeclined,
    ErrorNotification, InitializeCapabilities, InitializeParams, InitializeResponse,
    ItemNotification, NoPermissions, PlanNotification, SandboxMode, ThreadOpened,
    ThreadResumeParams, ThreadStartParams, TokenUsageBreakdown, TurnInterruptParams,
    TurnNotification, TurnStartParams, TurnStartResponse, TurnSteerParams, UsageNotification,
};
use super::wire::{self, Ended, Incoming, Outgoing, RpcError};
use crate::error::{Disconnect, InputError};
use crate::handshake::{AgentSetup, ForkRefusal};
use crate::input::{Command, Reply, SteerOutcome, SteerReply};
use crate::introduction::Introduction;
use crate::options::OptionError;
use crate::output::AgentOutput;

/// Lines read ahead of the loop.
const INBOUND: usize = 64;

/// JSON-RPC's "method not found".
const METHOD_NOT_FOUND: i64 = -32601;

/// What the driver says of itself in `initialize`.
const CLIENT: ClientInfo = ClientInfo {
    name: "vessel",
    title: "Vessel",
    version: env!("CARGO_PKG_VERSION"),
};

/// What an answer to one of the driver's requests is for.
#[derive(Debug)]
enum Pending {
    Initialize,
    Thread,
    TurnStart,
    Interrupt,
    Steer {
        content: Vec<ContentBlock>,
        reply: SteerReply,
    },
}

/// Where the session's turn is.
#[derive(Debug, Clone, PartialEq, Eq)]
enum TurnState {
    Idle,
    /// `turn/start` sent, its id not known yet; `cancel` when one was
    /// asked for meanwhile.
    Starting {
        cancel: bool,
    },
    Running {
        id: String,
    },
}

/// Drive one session over `codex app-server`'s stdio.
pub(super) async fn run(
    stdin: ChildStdin,
    stdout: ChildStdout,
    commands: mpsc::Receiver<Command>,
    outputs: mpsc::Sender<AgentOutput>,
    setup: AgentSetup,
) {
    let (lines, incoming) = mpsc::channel(INBOUND);
    drop(tokio::spawn(wire::read(stdout, lines)));
    let mut session = Session {
        out: Outgoing::start(stdin),
        outputs,
        setup,
        next: 0,
        pending: HashMap::new(),
        thread: None,
        turn: TurnState::Idle,
        streamed: HashSet::new(),
        total: TokenUsageBreakdown::default(),
        before: TokenUsageBreakdown::default(),
        context: None,
    };
    let disconnect = session.drive(incoming, commands).await;
    session.close(disconnect).await;
}

struct Session {
    out: Outgoing,
    outputs: mpsc::Sender<AgentOutput>,
    setup: AgentSetup,
    /// The next request id.
    next: u64,
    pending: HashMap<u64, Pending>,
    /// The thread, once the handshake is done.
    thread: Option<String>,
    turn: TurnState,
    /// Items of the turn whose text arrived in deltas.
    streamed: HashSet<String>,
    /// The thread's token total as last reported, and as it stood when the
    /// turn started.
    total: TokenUsageBreakdown,
    before: TokenUsageBreakdown,
    /// The context's fullness as last reported to the session.
    context: Option<ContextUsage>,
}

impl Session {
    /// Run until the connection ends; why it did.
    async fn drive(
        &mut self,
        mut incoming: mpsc::Receiver<Result<Incoming, Ended>>,
        mut commands: mpsc::Receiver<Command>,
    ) -> Disconnect {
        let params = InitializeParams {
            client_info: CLIENT,
            capabilities: InitializeCapabilities {
                experimental_api: true,
            },
        };
        if !self
            .request("initialize", &params, Pending::Initialize)
            .await
        {
            return Disconnect::Closed;
        }
        let handshake = tokio::time::sleep(self.setup.handshake_timeout);
        tokio::pin!(handshake);
        let mut commanding = true;
        loop {
            let ready = self.thread.is_some();
            tokio::select! {
                biased;
                item = incoming.recv() => match item {
                    Some(Ok(message)) => {
                        if let Some(disconnect) = self.incoming(message).await {
                            return disconnect;
                        }
                    }
                    Some(Err(Ended::Protocol(message))) => {
                        return self.failed(message);
                    }
                    Some(Err(Ended::Closed)) | None => {
                        return self.failed("the agent closed its connection".to_owned());
                    }
                },
                command = commands.recv(), if ready && commanding => match command {
                    Some(command) => self.command(command).await,
                    // Nobody sends any more; the agent's output is still
                    // read until the owner stops it.
                    None => commanding = false,
                },
                () = &mut handshake, if !ready => {
                    return handshake_failed(format!(
                        "Codex did not start a thread within {} s",
                        self.setup.handshake_timeout.as_secs()
                    ));
                }
            }
        }
    }

    /// The connection ended for `message`: before the handshake finished,
    /// the handshake failed.
    fn failed(&self, message: String) -> Disconnect {
        if self.thread.is_none() {
            handshake_failed(message)
        } else if message == "the agent closed its connection" {
            Disconnect::Closed
        } else {
            Disconnect::Protocol { message }
        }
    }

    /// Settle what is open, then say why the connection ended.
    async fn close(&mut self, disconnect: Disconnect) {
        if self.turn != TurnState::Idle {
            self.turn = TurnState::Idle;
            self.emit(AgentEvent::TurnFailed {
                message: "the agent exited during the turn".to_owned(),
            })
            .await;
        }
        for (_id, pending) in self.pending.drain() {
            if let Pending::Steer { reply, .. } = pending {
                let _gone = reply.send(Err(InputError::Closed));
            }
        }
        let _gone = self
            .outputs
            .send(AgentOutput::Disconnected(disconnect))
            .await;
    }

    async fn incoming(&mut self, message: Incoming) -> Option<Disconnect> {
        match message {
            Incoming::Response { id, result } => self.answered(id, Ok(result)).await,
            Incoming::Failure { id, error } => self.answered(id, Err(error)).await,
            Incoming::Notification { method, params } => {
                self.notification(&method, params).await;
                None
            }
            Incoming::Request { id, method, params } => {
                self.asked(&id, &method, &params).await;
                None
            }
        }
    }

    /// The answer to request `id`.
    async fn answered(&mut self, id: u64, answer: Result<Value, RpcError>) -> Option<Disconnect> {
        let Some(pending) = self.pending.remove(&id) else {
            tracing::debug!(id, "an answer to no request of the driver's");
            return None;
        };
        match pending {
            Pending::Initialize => return self.initialized(answer).await,
            Pending::Thread => return self.opened(answer).await,
            Pending::TurnStart => self.turn_started(answer).await,
            Pending::Interrupt => {
                if let Err(error) = answer {
                    tracing::info!(message = %error.message, "Codex refused to interrupt the turn");
                }
            }
            Pending::Steer { content, reply } => {
                let outcome = match answer {
                    Ok(_) => {
                        self.emit(AgentEvent::Steered { content }).await;
                        Ok(SteerOutcome::Injected)
                    }
                    Err(error) => Err(InputError::SteerRefused {
                        message: error.message,
                    }),
                };
                let _gone = reply.send(outcome);
            }
        }
        None
    }

    /// `initialize` answered: say who the agent is, then open the thread.
    async fn initialized(&mut self, answer: Result<Value, RpcError>) -> Option<Disconnect> {
        let response: InitializeResponse = match answer.map(serde_json::from_value) {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => return Some(handshake_failed(format!("initialize: {error}"))),
            Err(error) => return Some(handshake_failed(format!("initialize: {}", error.message))),
        };
        let introduction = Introduction {
            version: response.user_agent.as_deref().and_then(version_of),
            methods: Vec::new(),
        };
        self.output(AgentOutput::Introduced(Box::new(introduction)))
            .await;
        if !self.out.notify("initialized", &serde_json::json!({})).await {
            return Some(Disconnect::Closed);
        }
        if self.setup.fork.is_some() {
            // Forking a thread is the history slice's (`thread/fork`).
            return Some(Disconnect::Handshake {
                message: "this driver does not fork Codex threads yet".to_owned(),
                refused: None,
                fork_refused: Some(ForkRefusal::CannotFork),
            });
        }
        let cwd = self.setup.cwd.display().to_string();
        let model = self.setup.model.as_ref().map(ToString::to_string);
        let sent = if let Some(thread) = &self.setup.resume {
            let params = ThreadResumeParams {
                thread_id: thread.to_string(),
                cwd,
                model,
                approval_policy: AskForApproval::OnRequest,
                sandbox: SandboxMode::WorkspaceWrite,
                exclude_turns: true,
            };
            self.request("thread/resume", &params, Pending::Thread)
                .await
        } else {
            let params = ThreadStartParams {
                cwd,
                model,
                approval_policy: AskForApproval::OnRequest,
                sandbox: SandboxMode::WorkspaceWrite,
            };
            self.request("thread/start", &params, Pending::Thread).await
        };
        (!sent).then_some(Disconnect::Closed)
    }

    /// The thread is open: the session is ready.
    async fn opened(&mut self, answer: Result<Value, RpcError>) -> Option<Disconnect> {
        let opened: ThreadOpened = match answer.map(serde_json::from_value) {
            Ok(Ok(opened)) => opened,
            Ok(Err(error)) => return Some(handshake_failed(format!("the thread: {error}"))),
            Err(error) => return Some(handshake_failed(error.message)),
        };
        let abilities = AgentAbilities {
            steer: true,
            resume: true,
            image: false,
            audio: false,
            fork: false,
        };
        self.thread = Some(opened.thread.id.clone());
        // No options yet: they are slice 3's (`model/list`).
        self.output(AgentOutput::Options(Vec::new())).await;
        self.emit(AgentEvent::Ready {
            harness_session: HarnessSessionId::new(opened.thread.id),
            abilities: Some(abilities),
        })
        .await;
        None
    }

    /// `turn/start` answered.
    async fn turn_started(&mut self, answer: Result<Value, RpcError>) {
        match answer.map(serde_json::from_value::<TurnStartResponse>) {
            Ok(Ok(started)) => self.turn_is(started.turn.id).await,
            Ok(Err(error)) => {
                tracing::warn!(%error, "Codex's answer to turn/start did not read");
            }
            Err(error) => {
                if self.turn != TurnState::Idle {
                    self.turn = TurnState::Idle;
                    self.emit(AgentEvent::TurnFailed {
                        message: error.message,
                    })
                    .await;
                }
            }
        }
    }

    /// The running turn's id is `id`; a cancel asked for before it was
    /// known is sent now.
    async fn turn_is(&mut self, id: String) {
        let TurnState::Starting { cancel } = self.turn else {
            return;
        };
        self.turn = TurnState::Running { id };
        if cancel {
            self.interrupt().await;
        }
    }

    async fn notification(&mut self, method: &str, params: Value) {
        let Some(thread) = self.thread.clone() else {
            return;
        };
        let ours = |of: &str| of == thread;
        match method {
            "turn/started" => {
                if let Some(turn) = read::<TurnNotification>(method, params)
                    .filter(|notification| ours(&notification.thread_id))
                {
                    self.turn_is(turn.turn.id).await;
                }
            }
            "turn/completed" => {
                if let Some(done) = read::<TurnNotification>(method, params)
                    .filter(|notification| ours(&notification.thread_id))
                {
                    self.turn_completed(&done).await;
                }
            }
            "item/started" | "item/completed" => {
                if let Some(item) = read::<ItemNotification>(method, params)
                    .filter(|notification| ours(&notification.thread_id))
                {
                    self.item(method == "item/started", item).await;
                }
            }
            "item/agentMessage/delta" | "item/reasoning/summaryTextDelta" => {
                if let Some(delta) = read::<DeltaNotification>(method, params)
                    .filter(|notification| ours(&notification.thread_id))
                {
                    let content = ContentBlock::Text { text: delta.delta };
                    let _new = self.streamed.insert(delta.item_id);
                    self.emit(if method == "item/agentMessage/delta" {
                        AgentEvent::MessageChunk { content }
                    } else {
                        AgentEvent::ThoughtChunk { content }
                    })
                    .await;
                }
            }
            "turn/plan/updated" => {
                if let Some(plan) = read::<PlanNotification>(method, params)
                    .filter(|notification| ours(&notification.thread_id))
                {
                    self.emit(mapper::plan(&plan.plan)).await;
                }
            }
            "thread/tokenUsage/updated" => {
                if let Some(usage) = read::<UsageNotification>(method, params)
                    .filter(|notification| ours(&notification.thread_id))
                {
                    self.total = usage.token_usage.total;
                    let context = mapper::context(&usage.token_usage);
                    if context.is_some() && context != self.context {
                        self.context.clone_from(&context);
                        if let Some(context) = context {
                            self.output(AgentOutput::Usage(context)).await;
                        }
                    }
                }
            }
            "error" => {
                if let Some(error) = read::<ErrorNotification>(method, params)
                    .filter(|notification| ours(&notification.thread_id))
                {
                    let line = if error.will_retry {
                        format!("Codex will retry: {}", error.error.message)
                    } else {
                        format!("Codex: {}", error.error.message)
                    };
                    self.emit(AgentEvent::HarnessLog { line }).await;
                }
            }
            _ => {}
        }
    }

    async fn item(&mut self, started: bool, notification: ItemNotification) {
        let item = notification.item;
        if started {
            if let Some(call) = mapper::tool_call(&item) {
                self.emit(AgentEvent::ToolCall(call)).await;
            }
            return;
        }
        if let Some(update) = mapper::tool_call_done(&item) {
            self.emit(AgentEvent::ToolCallUpdate(update)).await;
        }
        if let Some((id, whole)) = mapper::whole_text(&item)
            && !self.streamed.contains(id)
        {
            self.emit(whole).await;
        }
    }

    async fn turn_completed(&mut self, done: &TurnNotification) {
        if self.turn == TurnState::Idle {
            // A turn the session did not start (none is started otherwise
            // today); its items were shown, its end is not the session's.
            return;
        }
        self.turn = TurnState::Idle;
        self.streamed.clear();
        let usage = mapper::turn_usage(self.before, self.total);
        self.emit(mapper::turn_end(&done.turn, usage)).await;
    }

    /// A request of the agent's: approvals are declined; nothing else is
    /// served yet.
    async fn asked(&mut self, id: &Value, method: &str, _params: &Value) {
        let what = match method {
            "item/commandExecution/requestApproval" => {
                let _sent = self
                    .out
                    .answer(
                        id,
                        &Declined {
                            decision: "decline",
                        },
                    )
                    .await;
                "to run a command"
            }
            "item/fileChange/requestApproval" => {
                let _sent = self
                    .out
                    .answer(
                        id,
                        &Declined {
                            decision: "decline",
                        },
                    )
                    .await;
                "to change files"
            }
            "item/permissions/requestApproval" => {
                let none = NoPermissions {
                    permissions: serde_json::Map::new(),
                };
                let _sent = self.out.answer(id, &none).await;
                "for more permissions"
            }
            "mcpServer/elicitation/request" => {
                let declined = ElicitationDeclined { action: "decline" };
                let _sent = self.out.answer(id, &declined).await;
                "for a tool server's input"
            }
            _ => {
                let message = format!("Vessel does not serve {method} yet");
                let _sent = self.out.refuse(id, METHOD_NOT_FOUND, &message).await;
                tracing::info!(method, "a Codex request Vessel does not serve, refused");
                return;
            }
        };
        let line = format!(
            "Vessel declined Codex's request {what}: this driver does not ask for permissions yet"
        );
        tracing::info!(method, "Codex's approval request declined");
        self.emit(AgentEvent::HarnessLog { line }).await;
    }

    async fn command(&mut self, command: Command) {
        match command {
            Command::Prompt { content, reply, .. } => self.prompt(content, reply).await,
            Command::CancelTurn { reply } => {
                self.cancel().await;
                answer(reply, Ok(()));
            }
            Command::Steer { content, reply, .. } => self.steer(content, reply).await,
            Command::Answer { id, reply, .. } => {
                answer(reply, Err(InputError::PermissionNotFound(id)));
            }
            Command::AnswerQuestion { id, reply, .. } => {
                answer(reply, Err(InputError::QuestionNotFound(id)));
            }
            Command::TurnRunning { reply } => {
                let _gone = reply.send(self.turn != TurnState::Idle);
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
        if self.turn != TurnState::Idle {
            answer(reply, Err(InputError::TurnInProgress));
            return;
        }
        let Some(thread) = self.thread.clone() else {
            answer(reply, Err(InputError::Closed));
            return;
        };
        let input = match mapper::user_input(&content) {
            Ok(input) => input,
            Err(error) => {
                answer(reply, Err(error));
                return;
            }
        };
        let params = TurnStartParams {
            thread_id: thread,
            input,
        };
        if !self
            .request("turn/start", &params, Pending::TurnStart)
            .await
        {
            answer(reply, Err(InputError::Closed));
            return;
        }
        self.turn = TurnState::Starting { cancel: false };
        self.streamed.clear();
        self.before = self.total;
        self.emit(AgentEvent::TurnStarted { content }).await;
        answer(reply, Ok(()));
    }

    async fn cancel(&mut self) {
        match &mut self.turn {
            TurnState::Idle => {}
            TurnState::Starting { cancel } => *cancel = true,
            TurnState::Running { .. } => self.interrupt().await,
        }
    }

    async fn interrupt(&mut self) {
        let (Some(thread), TurnState::Running { id }) = (self.thread.clone(), &self.turn) else {
            return;
        };
        let params = TurnInterruptParams {
            thread_id: thread,
            turn_id: id.clone(),
        };
        let _sent = self
            .request("turn/interrupt", &params, Pending::Interrupt)
            .await;
    }

    async fn steer(&mut self, content: Vec<ContentBlock>, reply: SteerReply) {
        let (thread, turn) = match (&self.thread, &self.turn) {
            (_, TurnState::Idle) => {
                let _gone = reply.send(Ok(SteerOutcome::PromptRequired));
                return;
            }
            (_, TurnState::Starting { .. }) => {
                let _gone = reply.send(Err(InputError::Steering(
                    "the turn has not started yet; send it again".to_owned(),
                )));
                return;
            }
            (None, TurnState::Running { .. }) => {
                let _gone = reply.send(Err(InputError::Closed));
                return;
            }
            (Some(thread), TurnState::Running { id }) => (thread.clone(), id.clone()),
        };
        let input = match mapper::user_input(&content) {
            Ok(input) => input,
            Err(error) => {
                let _gone = reply.send(Err(error));
                return;
            }
        };
        let params = TurnSteerParams {
            thread_id: thread,
            expected_turn_id: turn,
            input,
        };
        let id = self.next_id();
        if self.out.request(id, "turn/steer", &params).await {
            let _none = self.pending.insert(id, Pending::Steer { content, reply });
        } else {
            let _gone = reply.send(Err(InputError::Closed));
        }
    }

    fn next_id(&mut self) -> u64 {
        // A session never makes 2^64 requests; wrapping would only reuse
        // ids long answered.
        self.next = self.next.wrapping_add(1);
        self.next
    }

    /// Send a request whose answer `pending` handles; false once the agent
    /// closed its input.
    async fn request(
        &mut self,
        method: &str,
        params: &impl serde::Serialize,
        pending: Pending,
    ) -> bool {
        let id = self.next_id();
        let sent = self.out.request(id, method, params).await;
        if sent {
            let _none = self.pending.insert(id, pending);
        }
        sent
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

/// A notification's parameters as `T`, or `None` (logged) when they do
/// not read.
fn read<T: serde::de::DeserializeOwned>(method: &str, params: Value) -> Option<T> {
    match serde_json::from_value(params) {
        Ok(read) => Some(read),
        Err(error) => {
            tracing::warn!(method, %error, "a Codex notification that does not read");
            None
        }
    }
}

fn handshake_failed(message: String) -> Disconnect {
    Disconnect::Handshake {
        message,
        refused: None,
        fork_refused: None,
    }
}

/// The version in Codex's user agent (`codex_cli_rs/0.166.0 (...)`).
fn version_of(user_agent: &str) -> Option<AgentVersion> {
    let first = user_agent.split_whitespace().next()?;
    let version = first.split_once('/').map_or(first, |(_, version)| version);
    version.parse().ok()
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
mod tests {
    use super::*;

    #[test]
    fn the_version_is_the_user_agents_first_word_after_its_name() {
        assert_eq!(
            version_of("codex_cli_rs/0.166.0 (Linux; x86_64) xterm")
                .map(|version| version.to_string()),
            Some("0.166.0".to_owned())
        );
        assert_eq!(version_of(""), None);
    }
}
