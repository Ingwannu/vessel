//! The app's link to its nodes, through `vessel-client`, with nothing of a
//! UI toolkit's in it: the screens send [`Command`]s and draw what comes
//! back as [`Event`]s.
//!
//! [Decision Log]
//! - Purpose: the app is a client of its nodes as the CLI is (ADR 0035):
//!   the same book, routes, tokens and device credentials, no bridge.
//! - Constraint: `vessel-client` runs on tokio; GPUI runs its own executor
//!   on the main thread. A UI must not block on the network.
//! - Alternatives: a tokio runtime entered from GPUI's executor per call
//!   (Zed's `gpui_tokio` does this, but it is Zed's, GPL, and ties the
//!   link to GPUI's types); the network on GPUI's executor (it has no
//!   tokio reactor, and `vessel-client` needs one).
//! - Choice: one small multi-threaded tokio runtime owned by [`Link`],
//!   commands in through a bounded channel, events out through another.
//!   Both ends are plain tokio channels, which any executor can await, so
//!   the UI side needs no tokio of its own and the link no UI.
//! - Full channels: a command waits for room (a person cannot type faster
//!   than the link drains); the events channel is drained by the UI as
//!   fast as it draws, and a sender waits for room, so a burst of records
//!   slows the stream, never grows memory.

#[cfg(not(target_arch = "wasm32"))]
mod nodes;
mod prefs;
mod session;
// A browser has no `vessel-client` yet (no tokio network, no files): the
// same `Link`, `Command` and `Event`, answered from a demo fleet, so the
// screens can be drawn and typed into from WebAssembly
// (docs/plans/backend-m12-browser.md, questions 1 and 8).
#[cfg(any(target_arch = "wasm32", test))]
mod demo;
pub mod devices;
pub mod sync;
#[cfg(target_arch = "wasm32")]
mod web;
pub mod workspace;

#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use chrono::Utc;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::mpsc;
#[cfg(not(target_arch = "wasm32"))]
use vessel_app_view::{Agent, Machine, Reach};
use vessel_app_view::{Card, Fleet};
#[cfg(not(target_arch = "wasm32"))]
use vessel_client::NodeClient;
#[cfg(not(target_arch = "wasm32"))]
use vessel_proto::{
    ContentBlock, Delivery, PermissionOutcome, ProtocolVersion, SessionInfo, SessionInput,
    SessionMode, SessionSpec,
};
use vessel_proto::{
    EventRecord, HarnessRef, OptionChoice, PermissionId, PermissionOptionId, SessionId,
    SessionOption,
};

#[cfg(not(target_arch = "wasm32"))]
pub use nodes::{NameProblem, Offer, default_name, name_problem, read_link};
pub use prefs::{Kept, Prefs, UpdateNotes};
pub use session::{Act, ActAnswer, ActFailed, ActKind, Acted};
#[cfg(target_arch = "wasm32")]
pub use web::{Link, NameProblem, Offer, default_name, name_problem, read_link};

/// How often the sessions of every machine are listed again. The page
/// reads its list on the same cadence (`docs/PERFORMANCE.md`); a pushed
/// list is a protocol change for later.
#[cfg(not(target_arch = "wasm32"))]
const REFRESH: Duration = Duration::from_secs(2);

/// Room in each channel. Records come in pages of up to a few hundred; a
/// screen drains them every frame.
const CHANNEL: usize = 256;

/// Something the link could not do.
#[derive(Debug, Clone, thiserror::Error)]
pub enum LinkError {
    /// No home or configuration directory.
    #[error("no configuration directory")]
    NoHome,
    /// The book could not be read or written.
    #[error("the node book: {0}")]
    Book(String),
    /// A websocket node without a token file.
    #[error("no token file in the book for this node")]
    NoToken,
    /// The token file does not hold a token.
    #[error("the token: {0}")]
    Token(String),
    /// A paired node without a credential file.
    #[error("no device credential in the book for this node")]
    NoCredential,
    /// The credential could not be read or written.
    #[error("the device credential: {0}")]
    Credential(String),
    /// Reading a file failed.
    #[error("reading a file: {0}")]
    Io(String),
    /// The node could not be reached.
    #[error("{0}")]
    Connect(String),
    /// An SSH node (not in this build).
    #[error("SSH routes are not in this build yet; pair the node instead")]
    SshNotYet,
    /// The pasted text is not a pairing link.
    #[error("not a pairing link: {0}")]
    PairingLink(String),
    /// The name is not a valid machine name.
    #[error("the name: {0}")]
    Name(String),
    /// A call to the node failed.
    #[error("{0}")]
    Call(String),
    /// The machine is not connected.
    #[error("{0} is not connected")]
    NotConnected(String),
}

/// What a screen asks of the link.
#[derive(Debug)]
pub enum Command {
    /// Read the book again and connect to every node in it.
    Reload,
    /// Ask about a machine's paired devices.
    Devices {
        /// Its name in the book.
        machine: vessel_app_view::devices::MachineKey,
        /// The page request identity.
        ticket: vessel_app_view::devices::Ticket,
        /// The device operation.
        ask: devices::Ask,
    },
    /// Read-only sharing status for one selected machine.
    Sync {
        /// Its parsed node-book identity.
        machine: vessel_app_view::devices::MachineKey,
        /// Correlation identity reserved by the page.
        ticket: vessel_app_view::devices::Ticket,
    },
    /// Show a session's conversation: its records from the start, then as
    /// they arrive. Replaces the session followed before.
    Open {
        /// The machine.
        machine: String,
        /// The session.
        session: SessionId,
    },
    /// Send a prompt to a session (queued when a turn runs).
    Prompt {
        /// The machine.
        machine: String,
        /// The session.
        session: SessionId,
        /// What was typed.
        text: String,
    },
    /// Start a conversation: create the session, send its first prompt,
    /// and report it [`Event::Started`].
    Start {
        /// The machine.
        machine: String,
        /// The agent.
        harness: HarnessRef,
        /// The folder on the machine; `None` for its home.
        folder: Option<String>,
        /// The first prompt.
        prompt: String,
        /// The agent's options to start with (only those that change
        /// something, in the agent's order).
        options: Vec<OptionChoice>,
    },
    /// Ask what `harness` offers on `machine` before any session of it
    /// (`harness.options`); answered [`Event::Options`]. Takes seconds the
    /// first time (the node starts the agent to ask it), so it runs beside
    /// the other commands.
    Options {
        /// The machine.
        machine: String,
        /// The agent.
        harness: HarnessRef,
    },
    /// Start an ended or interrupted session again with its conversation
    /// (`session.resume`, protocol 10), then send it `text`; answered
    /// [`Event::Resumed`] or [`Event::ResumeFailed`].
    Resume {
        /// The machine.
        machine: String,
        /// The session.
        session: SessionId,
        /// What was typed.
        text: String,
    },
    /// Stop the running turn.
    Stop {
        /// The machine.
        machine: String,
        /// The session.
        session: SessionId,
    },
    /// Answer a permission the agent asked for.
    Answer {
        /// The machine.
        machine: String,
        /// The session.
        session: SessionId,
        /// The request.
        id: PermissionId,
        /// The answer chosen.
        option: PermissionOptionId,
    },
    /// Remember the person's choices (machine, agent, folders).
    SavePrefs(Prefs),
    /// Act on a session: rename, mark, start again, delete, kill, move,
    /// terminal input (answered by `Event::Acted`).
    Act {
        /// The machine.
        machine: String,
        /// The session.
        session: SessionId,
        /// What to do.
        act: Act,
    },
    /// Pair with a node from a pairing link, as `name` in the book.
    Pair {
        /// The decoded link.
        offer: Offer,
        /// The machine's name in the book.
        name: String,
        /// The name is the one the app filled in: use the node's own name
        /// instead when it makes a free book name.
        prefer_node_name: bool,
    },
    /// Ask a machine about its projects, worktrees, git or files; answered
    /// by [`Event::Workspace`] (`workspace`'s module says how).
    Workspace {
        /// The machine.
        machine: String,
        /// What is asked.
        ask: workspace::Ask,
    },
}

/// What the link reports.
#[derive(Debug)]
pub enum Event {
    /// The machines and their sessions, read again.
    Fleet(Fleet),
    /// A Devices request answered, with its identity.
    Devices(devices::Answer),
    /// Safe, correlated sharing status.
    Sync(sync::Answer),
    /// Records of the session being followed.
    Records {
        /// The session.
        session: SessionId,
        /// The records, in order.
        records: Vec<EventRecord>,
    },
    /// A session started from the app, to open.
    Started(Card),
    /// A prompt was delivered.
    Sent {
        /// The session.
        session: SessionId,
    },
    /// Pairing worked.
    Paired {
        /// The node's own name for itself.
        node: String,
        /// The route the pairing went through.
        route: String,
    },
    /// Pairing failed; the words to show beside the form.
    PairFailed(String),
    /// A session act's answer.
    Acted(ActAnswer),
    /// An ended session runs again, and the prompt was sent to it.
    Resumed {
        /// The session.
        session: SessionId,
    },
    /// An ended session could not be started again; the node's words.
    ResumeFailed {
        /// The session.
        session: SessionId,
        /// Why.
        why: String,
    },
    /// What an agent offers, or why the node could not say.
    Options {
        /// The machine.
        machine: String,
        /// The agent.
        harness: HarnessRef,
        /// Its options in its order, or the words of the failure.
        outcome: Result<Vec<SessionOption>, String>,
    },
    /// Something failed; the words to show.
    Failed(String),
    /// A machine answered a workspace ask.
    Workspace {
        /// The machine.
        machine: String,
        /// The answer, with what was asked.
        answer: workspace::Answer,
    },
}

#[cfg(not(target_arch = "wasm32"))]
/// The link: a tokio runtime and the two channels.
#[derive(Debug)]
pub struct Link {
    commands: mpsc::Sender<Command>,
    _runtime: tokio::runtime::Runtime,
}

#[cfg(not(target_arch = "wasm32"))]
impl Link {
    /// Start the link; `label` is the build's label, sent as this client's
    /// version. Events arrive on the receiver returned.
    ///
    /// # Errors
    /// The runtime could not be started.
    pub fn start(label: &str) -> std::io::Result<(Self, mpsc::Receiver<Event>)> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("vessel-link")
            .enable_all()
            .build()?;
        let (commands, inbox) = mpsc::channel(CHANNEL);
        let (events, outbox) = mpsc::channel(CHANNEL);
        let state = State {
            label: label.to_owned(),
            events,
            clients: HashMap::new(),
            follow: None,
            workspace: workspace::Hub::default(),
            device_calls: devices::Hub::default(),
            sync_calls: sync::Hub::default(),
        };
        drop(runtime.spawn(state.run(inbox)));
        Ok((
            Self {
                commands,
                _runtime: runtime,
            },
            outbox,
        ))
    }

    /// Ask for something; returns at once (a full queue drops the command
    /// and says so, which only a stuck link would cause).
    pub fn send(&self, command: Command) {
        if let Err(error) = self.commands.try_send(command) {
            tracing::warn!(%error, "the link is not taking commands");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// The link's own state, owned by its task.
struct State {
    label: String,
    events: mpsc::Sender<Event>,
    clients: HashMap<String, Connected>,
    follow: Option<tokio::task::JoinHandle<()>>,
    workspace: workspace::Hub,
    device_calls: devices::Hub,
    sync_calls: sync::Hub,
}

#[cfg(not(target_arch = "wasm32"))]
/// A machine connected, and what it can start (read once per connection:
/// what is installed changes rarely, and a new connection reads it again).
#[derive(Clone)]
struct Connected {
    client: NodeClient,
    agents: Vec<Agent>,
}

#[cfg(not(target_arch = "wasm32"))]
impl State {
    async fn run(mut self, mut inbox: mpsc::Receiver<Command>) {
        self.reload().await;
        let mut tick = tokio::time::interval(REFRESH);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                command = inbox.recv() => match command {
                    Some(command) => self.handle(command).await,
                    None => break,
                },
                _instant = tick.tick() => self.refresh().await,
            }
        }
    }

    async fn handle(&mut self, command: Command) {
        match command {
            Command::Reload => self.reload().await,
            Command::Sync { machine, ticket } => self.sync(&machine, ticket).await,
            Command::Devices {
                machine,
                ticket,
                ask,
            } => self.devices(&machine, ticket, ask).await,
            Command::Open { machine, session } => self.open(&machine, session),
            Command::Prompt {
                machine,
                session,
                text,
            } => self.prompt(&machine, session, text).await,
            Command::Start {
                machine,
                harness,
                folder,
                prompt,
                options,
            } => {
                self.start(&machine, harness, folder, prompt, options).await;
            }
            Command::Options { machine, harness } => self.options(machine, harness),
            Command::Resume {
                machine,
                session,
                text,
            } => self.resume(&machine, session, text).await,
            Command::Stop { machine, session } => {
                let input = SessionInput::CancelTurn { turn: None };
                self.input(&machine, session, input).await;
            }
            Command::Answer {
                machine,
                session,
                id,
                option,
            } => {
                let input = SessionInput::AnswerPermission {
                    id,
                    outcome: PermissionOutcome::Selected { option },
                };
                self.input(&machine, session, input).await;
            }
            Command::Pair {
                offer,
                name,
                prefer_node_name,
            } => match nodes::pair(&offer, &name, prefer_node_name).await {
                Ok((node, route)) => {
                    self.emit(Event::Paired { node, route }).await;
                    self.reload().await;
                }
                Err(error) => self.emit(Event::PairFailed(error.to_string())).await,
            },
            Command::Act {
                machine,
                session,
                act,
            } => self.act(&machine, session, act),
            Command::Workspace { machine, ask } => self.workspace(machine, ask),
            Command::SavePrefs(prefs) => {
                let saved = tokio::task::spawn_blocking(move || prefs.save()).await;
                match saved {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => tracing::warn!(%error, "could not save the app's choices"),
                    Err(error) => tracing::warn!(%error, "could not save the app's choices"),
                }
            }
        }
    }

    async fn emit(&self, event: Event) {
        // The receiver is gone only when the app is closing.
        let _closed = self.events.send(event).await;
    }

    /// Connect to every node in the book (again), then list sessions.
    async fn reload(&mut self) {
        self.clients.clear();
        self.refresh().await;
    }

    /// List every machine's sessions, connecting to those not connected.
    async fn refresh(&mut self) {
        let book = match nodes::read_book() {
            Ok(book) => book,
            Err(error) => {
                self.emit(Event::Failed(error.to_string())).await;
                return;
            }
        };
        let mut machines = Vec::with_capacity(book.len());
        for (name, entry) in book {
            let name = name.as_str().to_owned();
            let reach = match self.clients.get(&name) {
                Some(connected) => Ok(connected.clone()),
                None => self.connect(&entry).await,
            };
            let machine = match reach {
                Ok(connected) => match connected.client.sessions().await {
                    Ok(sessions) => {
                        let machine = Machine {
                            sessions: sessions
                                .iter()
                                .map(|session| Card::from_session(&name, session))
                                .collect(),
                            agents: connected.agents.clone(),
                            protocol: Some(connected.client.protocol()),
                            name: name.clone(),
                            reach: Reach::Online,
                        };
                        let _earlier = self.clients.insert(name, connected);
                        machine
                    }
                    Err(error) => {
                        // A dropped connection: dial again next time.
                        let _gone = self.clients.remove(&name);
                        unreachable(name, &error.to_string())
                    }
                },
                Err(error) => unreachable(name, &error.to_string()),
            };
            machines.push(machine);
        }
        self.emit(Event::Fleet(Fleet {
            machines,
            read_at: Some(Utc::now()),
        }))
        .await;
    }

    async fn connect(
        &self,
        entry: &vessel_client::book::BookEntry,
    ) -> Result<Connected, LinkError> {
        let client = nodes::connect(entry, nodes::client_info(&self.label)).await?;
        let harnesses = match client.harnesses().await {
            Ok(harnesses) => harnesses,
            Err(error) => {
                tracing::warn!(%error, "could not list the machine's agents");
                Vec::new()
            }
        };
        // `agent.list` (protocol 11) names every agent the node knows, the
        // ones not installed too; `harness.list` only the installed.
        let agents = if client.protocol() >= ProtocolVersion::new(11) {
            match client.call(vessel_proto::agents::ListAgents {}).await {
                Ok(known) => Agent::known(&harnesses, &known),
                Err(error) => {
                    tracing::warn!(%error, "could not read the node's agent list");
                    Agent::all(&harnesses)
                }
            }
        } else {
            Agent::all(&harnesses)
        };
        Ok(Connected { client, agents })
    }

    fn client(&self, machine: &str) -> Result<&NodeClient, LinkError> {
        self.clients
            .get(machine)
            .map(|connected| &connected.client)
            .ok_or_else(|| LinkError::NotConnected(machine.to_owned()))
    }

    /// Create a conversation and send its first prompt: `session.prompt`
    /// where the node speaks protocol 10, `session.send` before it.
    async fn start(
        &mut self,
        machine: &str,
        harness: HarnessRef,
        folder: Option<String>,
        prompt: String,
        options: Vec<OptionChoice>,
    ) {
        let started = async {
            let client = self.client(machine)?.clone();
            let mode = SessionMode::Structured {
                model: None,
                resume: None,
                options,
            };
            let mut spec = SessionSpec::new(harness, mode);
            if let Some(folder) = folder {
                spec = spec.with_cwd(folder);
            }
            let info: SessionInfo = client
                .create_session(spec)
                .await
                .map_err(|error| LinkError::Call(error.to_string()))?;
            let content = vec![ContentBlock::Text { text: prompt }];
            let sent = if client.protocol() >= ProtocolVersion::new(10) {
                client
                    .prompt(info.id, content, Delivery::Start)
                    .await
                    .map(|_delivered| ())
            } else {
                client.send(info.id, SessionInput::Prompt { content }).await
            };
            sent.map_err(|error| LinkError::Call(error.to_string()))?;
            Ok::<_, LinkError>(Card::from_session(machine, &info))
        }
        .await;
        match started {
            Ok(card) => {
                self.emit(Event::Started(card)).await;
                self.refresh().await;
            }
            Err(error) => self.emit(Event::Failed(error.to_string())).await,
        }
    }

    /// Ask `machine` what `harness` offers, beside the other commands (the
    /// node may start the agent to answer). A machine not connected is
    /// answered at once with why.
    fn options(&self, machine: String, harness: HarnessRef) {
        let client = self.client(&machine).cloned();
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let outcome = match client {
                Ok(client) => client
                    .harness_options(harness.clone(), None)
                    .await
                    .map(|offered| offered.options)
                    .map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            let _closed = events
                .send(Event::Options {
                    machine,
                    harness,
                    outcome,
                })
                .await;
        }));
    }

    /// Follow `session` from its first record, replacing the one followed.
    fn open(&mut self, machine: &str, session: SessionId) {
        if let Some(earlier) = self.follow.take() {
            earlier.abort();
        }
        let Some(client) = self
            .clients
            .get(machine)
            .map(|connected| connected.client.clone())
        else {
            let events = self.events.clone();
            let machine = machine.to_owned();
            drop(tokio::spawn(async move {
                let _closed = events
                    .send(Event::Failed(LinkError::NotConnected(machine).to_string()))
                    .await;
            }));
            return;
        };
        let events = self.events.clone();
        self.follow = Some(tokio::spawn(follow(client, session, events)));
    }

    /// Start `session` again, then send it `text`, and list the fleet
    /// again (its card is live once more).
    async fn resume(&mut self, machine: &str, session: SessionId, text: String) {
        let resumed = async {
            let client = self.client(machine)?.clone();
            let _info = client
                .resume_session(session)
                .await
                .map_err(|error| LinkError::Call(error.to_string()))?;
            let content = vec![ContentBlock::Text { text }];
            client
                .prompt(session, content, Delivery::Queue)
                .await
                .map(|_delivered| ())
                .map_err(|error| LinkError::Call(error.to_string()))
        }
        .await;
        match resumed {
            Ok(()) => {
                self.emit(Event::Resumed { session }).await;
                self.refresh().await;
            }
            Err(error) => {
                self.emit(Event::ResumeFailed {
                    session,
                    why: error.to_string(),
                })
                .await;
            }
        }
    }

    async fn prompt(&self, machine: &str, session: SessionId, text: String) {
        let Some(client) = self.clients.get(machine).map(|connected| &connected.client) else {
            self.emit(Event::Failed(
                LinkError::NotConnected(machine.to_owned()).to_string(),
            ))
            .await;
            return;
        };
        let content = vec![ContentBlock::Text { text }];
        match client.prompt(session, content, Delivery::Queue).await {
            Ok(_delivered) => self.emit(Event::Sent { session }).await,
            Err(error) => {
                self.emit(Event::Failed(
                    LinkError::Call(error.to_string()).to_string(),
                ))
                .await;
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl State {
    /// Hand a workspace ask to its own task, with the machine's connection.
    fn workspace(&self, machine: String, ask: workspace::Ask) {
        let events = self.events.clone();
        match self.client(&machine) {
            Ok(client) => self.workspace.ask(client.clone(), machine, ask, events),
            Err(error) => {
                let problem = vessel_app_view::workspace::problem::Problem::Gone(error.to_string());
                workspace::Hub::refuse(machine, ask, problem, events);
            }
        }
    }

    async fn input(&self, machine: &str, session: SessionId, input: SessionInput) {
        let Some(client) = self.clients.get(machine).map(|connected| &connected.client) else {
            self.emit(Event::Failed(
                LinkError::NotConnected(machine.to_owned()).to_string(),
            ))
            .await;
            return;
        };
        if let Err(error) = client.send(session, input).await {
            self.emit(Event::Failed(
                LinkError::Call(error.to_string()).to_string(),
            ))
            .await;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn unreachable(name: String, why: &str) -> Machine {
    Machine {
        name,
        reach: Reach::Unreachable(why.to_owned()),
        sessions: Vec::new(),
        agents: Vec::new(),
        protocol: None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// A session's records to the events channel, one event per record (the
/// screen folds whatever has queued up in one update).
async fn follow(client: NodeClient, session: SessionId, events: mpsc::Sender<Event>) {
    let mut stream = match client.follow(session, vessel_proto::EventSeq::new(0)).await {
        Ok(stream) => stream,
        Err(error) => {
            let _closed = events.send(Event::Failed(error.to_string())).await;
            return;
        }
    };
    while let Some(item) = stream.next().await {
        match item {
            Ok(record) => {
                let event = Event::Records {
                    session,
                    records: vec![record],
                };
                if events.send(event).await.is_err() {
                    return;
                }
            }
            Err(error) => {
                let _closed = events.send(Event::Failed(error.to_string())).await;
                return;
            }
        }
    }
}
