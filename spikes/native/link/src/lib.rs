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

mod nodes;
mod prefs;

use std::collections::HashMap;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::mpsc;
use vessel_app_view::{Agent, Card, Fleet, Machine, Reach};
use vessel_client::NodeClient;
use vessel_proto::{
    ContentBlock, Delivery, EventRecord, HarnessRef, OptionChoice, PermissionId,
    PermissionOptionId, PermissionOutcome, ProtocolVersion, SessionId, SessionInfo, SessionInput,
    SessionMode, SessionOption, SessionSpec,
};

pub use nodes::{NameProblem, Offer, default_name, name_problem, read_link};
pub use prefs::{Kept, Prefs};

/// How often the sessions of every machine are listed again. The page
/// reads its list on the same cadence (`docs/PERFORMANCE.md`); a pushed
/// list is a protocol change for later.
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
}

/// What the link reports.
#[derive(Debug)]
pub enum Event {
    /// The machines and their sessions, read again.
    Fleet(Fleet),
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
}

/// The link: a tokio runtime and the two channels.
#[derive(Debug)]
pub struct Link {
    commands: mpsc::Sender<Command>,
    _runtime: tokio::runtime::Runtime,
}

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

/// The link's own state, owned by its task.
struct State {
    label: String,
    events: mpsc::Sender<Event>,
    clients: HashMap<String, Connected>,
    follow: Option<tokio::task::JoinHandle<()>>,
}

/// A machine connected, and what it can start (read once per connection:
/// what is installed changes rarely, and a new connection reads it again).
#[derive(Clone)]
struct Connected {
    client: NodeClient,
    agents: Vec<Agent>,
}

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
        let agents = match client.harnesses().await {
            Ok(harnesses) => Agent::conversational(&harnesses),
            Err(error) => {
                tracing::warn!(%error, "could not list the machine's agents");
                Vec::new()
            }
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

impl State {
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

fn unreachable(name: String, why: &str) -> Machine {
    Machine {
        name,
        reach: Reach::Unreachable(why.to_owned()),
        sessions: Vec::new(),
        agents: Vec::new(),
    }
}

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
