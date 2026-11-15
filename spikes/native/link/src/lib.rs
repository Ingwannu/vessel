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
//! - Full channels: a command is offered without waiting (the UI thread
//!   must not block), and one the link does not take comes back typed
//!   ([`Link::submit`], [`Unsent`]) so what waits on its answer can
//!   settle; the events channel is drained by the UI as fast as it draws,
//!   and a sender waits for room, so a burst of records slows the stream,
//!   never grows memory.

mod command;
// History observed replies validate original weak delivery at UI application.
mod delivery;
pub use delivery::DeliveryGuard;
#[cfg(not(target_arch = "wasm32"))]
mod connections;
#[cfg(not(target_arch = "wasm32"))]
use connections::Connected;
#[cfg(not(target_arch = "wasm32"))]
mod dispatch;
mod error;
mod event;
pub mod machines;
#[cfg(not(target_arch = "wasm32"))]
mod nodes;
#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod testing;
// The agents' lists kept between runs (owner word 28); a browser keeps
// none yet (no files).
#[cfg(not(target_arch = "wasm32"))]
mod offers;
#[cfg(not(target_arch = "wasm32"))]
mod options;
// The composers' drafts kept between runs (track P's row e).
#[cfg(not(target_arch = "wasm32"))]
pub mod drafts;
// A composer's files sent to their session (track P's row c).
pub mod attachments;
// What is sent to a session: a conversation started, a prompt, its
// files, a session started again, an input.
#[cfg(not(target_arch = "wasm32"))]
mod pairing;
mod prefs;
#[cfg(not(target_arch = "wasm32"))]
mod refresh;
#[cfg(not(target_arch = "wasm32"))]
mod sending;
mod session;
// A browser has no `vessel-client` yet (no tokio network, no files): the
// same `Link`, `Command` and `Event`, answered from a demo fleet, so the
// screens can be drawn and typed into from WebAssembly
// (docs/plans/backend-m12-browser.md, questions 1 and 8).
pub mod agents;
#[cfg(any(target_arch = "wasm32", test))]
mod demo;
pub mod devices;
pub mod history;
pub mod node_settings;
pub mod search;
pub mod sync;
#[cfg(target_arch = "wasm32")]
mod web;
pub mod workspace;

#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::mpsc;
#[cfg(not(target_arch = "wasm32"))]
use vessel_app_view::connection::ConnectionEpochs;
#[cfg(not(target_arch = "wasm32"))]
use vessel_client::NodeClient;
use vessel_proto::SessionId;

pub use command::Command;
pub use error::LinkError;
pub use event::{Event, KeptLists};
#[cfg(not(target_arch = "wasm32"))]
pub use nodes::{NameProblem, Offer, default_name, name_problem, read_link};
pub use prefs::{Kept, Prefs, UpdateNotes};
pub use session::{Act, ActAnswer, ActFailed, ActKind, Acted, NodeName, TerminalOpening};
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

/// Why the link did not take a command ([`Link::submit`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Rejected {
    /// Its queue is full: the link's task is behind (only a stuck link
    /// fills 256 commands).
    #[error("the link's queue is full")]
    Full,
    /// The link stopped: the app is closing, or its task ended.
    #[error("the link has stopped")]
    Closed,
}

/// A command the link did not take, given back with why: nothing of it
/// reached a machine, so no answer will come for it.
///
/// [Decision Log]
/// - Purpose: [`Link::send`] offers a command with `try_send` and only
///   logged a full or closed queue, so a screen that set a key pending
///   (a card's act, a pairing, a workspace ask) waited for an answer that
///   never came: a spinner that stays (track G1's review, 2026-11-13).
/// - Constraint: the UI must not wait for room (it runs on the UI
///   thread), and the existing callers that wait on nothing keep
///   [`Link::send`].
/// - Alternatives: an awaiting send (blocks or needs every caller async);
///   a boolean (the caller then still holds a moved command and cannot
///   tell full from stopped).
/// - Choice: [`Link::submit`] returns the command with a typed reason;
///   the caller settles what waits on it in its own words.
/// - Cost: the command is boxed on the failure path only.
#[derive(Debug)]
pub struct Unsent {
    /// Why.
    pub why: Rejected,
    /// The command, as it was given.
    pub command: Box<Command>,
}

impl Unsent {
    /// `command`, not taken for `why`.
    #[must_use]
    pub fn new(why: Rejected, command: Command) -> Self {
        Self {
            why,
            command: Box::new(command),
        }
    }
}

/// Offer `command` on `commands` without waiting.
#[cfg(not(target_arch = "wasm32"))]
fn offer(commands: &mpsc::Sender<Command>, command: Command) -> Result<(), Unsent> {
    commands.try_send(command).map_err(|error| match error {
        mpsc::error::TrySendError::Full(command) => Unsent::new(Rejected::Full, command),
        mpsc::error::TrySendError::Closed(command) => Unsent::new(Rejected::Closed, command),
    })
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
            commands: commands.downgrade(),
            events,
            clients: HashMap::new(),
            follow: None,
            follow_machine: None,
            book_path: machines::BookPath::discover(),
            workspace: workspace::Hub::default(),
            agents: agents::Hub::default(),

            admissions: ConnectionEpochs::default(),
            device_calls: devices::Hub::default(),
            history_calls: history::Hub::default(),
            sync_calls: sync::Hub::default(),
            offers: std::sync::Arc::default(),
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

    /// Ask for something and say whether the link took it; returns at
    /// once. A caller that waits on the answer settles what it set
    /// pending when the command comes back.
    ///
    /// # Errors
    /// The link's queue is full, or the link stopped: the command, given
    /// back.
    pub fn submit(&self, command: Command) -> Result<(), Unsent> {
        offer(&self.commands, command)
    }

    /// Ask for something nothing waits on; returns at once (a command the
    /// link does not take is dropped and logged).
    pub fn send(&self, command: Command) {
        if let Err(unsent) = self.submit(command) {
            tracing::warn!(why = %unsent.why, "the link is not taking commands");
        }
    }

    /// A link with no task behind its queue, `room` commands deep, and
    /// the queue's other end: for a test that fills the queue.
    ///
    /// # Errors
    /// The runtime could not be started.
    #[cfg(feature = "testing")]
    pub fn detached(room: usize) -> std::io::Result<(Self, mpsc::Receiver<Command>)> {
        let (commands, inbox) = mpsc::channel(room);
        let runtime = tokio::runtime::Builder::new_current_thread().build()?;
        Ok((
            Self {
                commands,
                _runtime: runtime,
            },
            inbox,
        ))
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// The link's own state, owned by its task.
struct State {
    label: String,
    // Weak: internal closure reports must not keep the actor inbox alive.
    commands: mpsc::WeakSender<Command>,
    events: mpsc::Sender<Event>,
    clients: HashMap<String, Connected>,
    follow: Option<tokio::task::JoinHandle<()>>,
    follow_machine: Option<String>,
    book_path: machines::BookPath,
    workspace: workspace::Hub,
    agents: agents::Hub,

    admissions: ConnectionEpochs,
    device_calls: devices::Hub,
    history_calls: history::Hub,
    sync_calls: sync::Hub,
    /// The agents' lists kept between runs, by node: read before the first
    /// connection, written when an answer differs (on the blocking pool;
    /// the lock is held only there and for a lookup at connect).
    offers: std::sync::Arc<std::sync::Mutex<offers::Store>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl State {
    async fn run(mut self, mut inbox: mpsc::Receiver<Command>) {
        // The kept lists first, so the first connection hands them over.
        let path = offers::Store::default_path();
        match tokio::task::spawn_blocking(move || offers::Store::load(path)).await {
            Ok(store) => {
                if let Ok(mut kept) = self.offers.lock() {
                    *kept = store;
                }
            }
            Err(error) => tracing::warn!(%error, "the kept agent lists were not read"),
        }
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

    async fn emit(&self, event: Event) {
        // The receiver is gone only when the app is closing.
        let _closed = self.events.send(event).await;
    }

    fn client(&self, machine: &str) -> Result<&NodeClient, LinkError> {
        self.clients
            .get(machine)
            .map(|connected| &connected.client)
            .ok_or_else(|| LinkError::NotConnected(machine.to_owned()))
    }

    /// Follow `session` from its first record, replacing the one followed.
    fn open(&mut self, machine: &str, session: SessionId) {
        self.follow_machine = None;
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
                let why = LinkError::NotConnected(machine).to_string();
                let _closed = events.send(Event::SessionLost { session, why }).await;
            }));
            return;
        };
        let events = self.events.clone();
        self.follow_machine = Some(machine.to_owned());
        self.follow = Some(tokio::spawn(follow(client, session, events)));
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
}

#[cfg(not(target_arch = "wasm32"))]
/// A session's records to the events channel, one event per record (the
/// screen folds whatever has queued up in one update).
async fn follow(client: NodeClient, session: SessionId, events: mpsc::Sender<Event>) {
    let lost = |why: String| Event::SessionLost { session, why };
    let mut stream = match client.follow(session, vessel_proto::EventSeq::new(0)).await {
        Ok(stream) => stream,
        Err(error) => {
            let _closed = events.send(lost(error.to_string())).await;
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
                let _closed = events.send(lost(error.to_string())).await;
                return;
            }
        }
    }
}
