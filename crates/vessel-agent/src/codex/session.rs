//! One session over Codex's app-server: the handshake, turns, steering and
//! cancelling, the mode, and what the agent asks (slices 1 and 2 of the
//! plan's Codex section).
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
//! - Turn ids: the running turn's id is `turn/start`'s answer's, and only
//!   that turn's `turn/completed` ends it: another client may run turns on
//!   the same thread. A cancel before the answer is remembered and sent
//!   once the id is known. A completion of the running turn that does not
//!   read ends it failed, never leaving the session busy for good.
//! - What Codex asks (slice 2): approvals become permission requests on
//!   the item's tool call (announced from the request when Codex has not
//!   announced the item), its questions and its tool servers' forms become
//!   questions (`asks`). A cancel withdraws what is open before it
//!   interrupts, and what Codex asks until the turn ends is withdrawn at
//!   once, as the ACP driver does; the turn's end and the connection's
//!   withdraw the rest, and Codex's `serverRequest/resolved` settles one
//!   it no longer waits on. With questions off (nobody watches the
//!   session) a question is answered empty or declined at once, with a
//!   line in the log. Requests the driver does not serve are answered with
//!   an error, never left waiting.
//! - Approvals of another thread (a sub-agent's) are asked too: unasked,
//!   Codex would wait on them for good, and nothing is allowed unasked.
//! - The mode (`mode`): set with the thread's start or resume, and a change
//!   goes with the next `turn/start`, as codex-acp applies it.
//! - Other threads' notifications (Codex's sub-agents run threads of their
//!   own) are not this session's and are ignored here.

use std::collections::{HashMap, HashSet};

use serde_json::Value;
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::mpsc;
use vessel_proto::{
    AgentAbilities, AgentEvent, ContentBlock, ContextUsage, HarnessSessionId, QuestionOutcome,
    ToolCallUpdate, ToolKind,
};

use super::asks::{self, Approvals, Asking, Questions, Settled};
use super::mapper;
use super::mode::Mode;
use super::protocol::{
    ClientInfo, CommandApproval, DeltaNotification, ErrorNotification, FileChangeApproval,
    InitializeCapabilities, InitializeParams, InitializeResponse, ItemNotification,
    PermissionsApproval, PlanNotification, RequestResolved, ThreadOpened, ThreadResumeParams,
    ThreadStartParams, TokenUsageBreakdown, TurnInterruptParams, TurnNotification, TurnStartParams,
    TurnStartResponse, TurnSteerParams, UsageNotification, UserInputRequest,
};
use super::wire::{self, Ended, Incoming, Outgoing, RpcError};
use crate::error::{Disconnect, InputError};
use crate::handshake::{AgentSetup, ForkRefusal};
use crate::input::{Command, Reply, SteerOutcome, SteerReply};
use crate::introduction::Introduction;
use crate::output::AgentOutput;

mod commands;
mod notifications;
mod replies;
use replies::{answer, handshake_failed, read, version_of};

/// Lines read ahead of the loop.
const INBOUND: usize = 64;

/// JSON-RPC's "method not found".
const METHOD_NOT_FOUND: i64 = -32601;

/// JSON-RPC's "invalid params".
const INVALID_PARAMS: i64 = -32602;

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
    let mut session = Session::new(Outgoing::start(stdin), outputs, setup);
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
    /// The mode the next turn runs in.
    mode: Mode,
    /// What waits on a person.
    approvals: Approvals,
    questions: Questions,
    /// Tool calls announced this turn, by their item or an approval.
    announced: HashSet<String>,
    /// The running turn was cancelled: what Codex asks until it ends is
    /// withdrawn at once.
    cancelling: bool,
    children: super::children::Children,
}

impl Session {
    fn new(out: Outgoing, outputs: mpsc::Sender<AgentOutput>, setup: AgentSetup) -> Self {
        let children = super::children::Children::new(setup.resume.is_some());
        Self {
            out,
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
            mode: Mode::DEFAULT,
            approvals: Approvals::default(),
            questions: Questions::default(),
            announced: HashSet::new(),
            cancelling: false,
            children,
        }
    }

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
            // Commands first: while Codex streams, its lines are always
            // ready, and a cancel (or a permission's answer) must not wait
            // for the whole reply. It is safe because a turn ends only at
            // its `turn/completed`, taken in order with its items (as the
            // ACP driver's loop does).
            tokio::select! {
                biased;
                command = commands.recv(), if ready && commanding => match command {
                    Some(command) => self.command(command).await,
                    // Nobody sends any more; the agent's output is still
                    // read until the owner stops it.
                    None => commanding = false,
                },
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
        self.withdraw().await;
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
                self.asked(id, &method, params).await;
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
        // The mode is the one option until the options slice: another
        // choice fails the handshake, as an ACP agent's unknown option
        // does, rather than run without it unannounced.
        for choice in &self.setup.options {
            match Mode::chosen(choice) {
                Ok(mode) => self.mode = mode,
                Err(error) => return Some(handshake_failed(error.to_string())),
            }
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
                approval_policy: self.mode.approval(),
                approvals_reviewer: self.mode.reviewer(),
                sandbox: self.mode.sandbox(),
                exclude_turns: true,
            };
            self.request("thread/resume", &params, Pending::Thread)
                .await
        } else {
            let params = ThreadStartParams {
                cwd,
                model,
                approval_policy: self.mode.approval(),
                approvals_reviewer: self.mode.reviewer(),
                sandbox: self.mode.sandbox(),
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
        // The mode alone until slice 3 (`model/list`).
        self.output(AgentOutput::Options(vec![self.mode.option()]))
            .await;
        self.emit(AgentEvent::Ready {
            harness_session: HarnessSessionId::new(opened.thread.id),
            abilities: Some(abilities),
        })
        .await;
        // Nor tool servers (the thread's `config`, slice 3): said, so a
        // session never lacks them unannounced.
        if !self.setup.mcp_servers.is_empty() {
            let names: Vec<String> = self
                .setup
                .mcp_servers
                .keys()
                .map(ToString::to_string)
                .collect();
            let line = format!(
                "vessel: Codex's own protocol does not pass tool servers yet; this session runs \
                 without {}",
                names.join(", ")
            );
            self.emit(AgentEvent::HarnessLog { line }).await;
        }
        None
    }

    /// `turn/start` answered. The turn's id comes only from here, so an
    /// answer that fails or does not read ends the turn failed: no
    /// `turn/completed` could match it, and a cancel would wait for good.
    async fn turn_started(&mut self, answer: Result<Value, RpcError>) {
        let message = match answer.map(serde_json::from_value::<TurnStartResponse>) {
            Ok(Ok(started)) => return self.turn_is(started.turn.id).await,
            Ok(Err(error)) => format!("Codex's answer to turn/start did not read: {error}"),
            Err(error) => error.message,
        };
        if self.turn != TurnState::Idle {
            self.turn = TurnState::Idle;
            self.withdraw().await;
            self.emit(AgentEvent::TurnFailed { message }).await;
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;
    use tokio::sync::oneshot;
    use vessel_proto::ResolvedMcpServers;

    use super::*;

    /// A command is taken before the agent's lines already queued: while
    /// Codex streams its lines are always ready, and a cancel must not wait
    /// for the reply's end. Here the outputs hold one item and are not
    /// read, so a loop that took the queued deltas first would block on
    /// the second and never take the cancel.
    #[tokio::test]
    async fn a_command_is_taken_before_queued_agent_lines() {
        let (out, mut written) = Outgoing::detached();
        let (outputs, _unread) = mpsc::channel(1);
        let setup = AgentSetup {
            cwd: std::env::temp_dir(),
            model: None,
            resume: None,
            options: Vec::new(),
            mcp_servers: ResolvedMcpServers::new(),
            handshake_timeout: Duration::from_secs(60),
            questions: false,
            fork: None,
            system_prompt: None,
            private_dir: None,
        };
        let mut session = Session::new(out, outputs, setup);
        session.thread = Some("thread".to_owned());
        session.turn = TurnState::Running {
            id: "turn".to_owned(),
        };
        let (lines, incoming) = mpsc::channel(256);
        for _ in 0..100 {
            let delta = Incoming::Notification {
                method: "item/agentMessage/delta".to_owned(),
                params: json!({"threadId": "thread", "turnId": "turn", "itemId": "m",
                    "delta": "x"}),
            };
            lines.send(Ok(delta)).await.unwrap();
        }
        let (commands, taken) = mpsc::channel(4);
        let (reply, answered) = oneshot::channel();
        commands.send(Command::CancelTurn { reply }).await.unwrap();
        let _driving = tokio::spawn(async move { session.drive(incoming, taken).await });
        tokio::time::timeout(Duration::from_secs(10), answered)
            .await
            .expect("the cancel waited behind the agent's lines")
            .unwrap()
            .unwrap();
        let initialize = written.recv().await.unwrap();
        assert!(String::from_utf8_lossy(&initialize).contains("\"initialize\""));
        let interrupt = written.recv().await.unwrap();
        assert!(String::from_utf8_lossy(&interrupt).contains("\"turn/interrupt\""));
        drop((lines, commands));
    }

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
