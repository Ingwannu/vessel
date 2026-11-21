//! The link in a browser: the same [`Link`], [`Command`]s and [`Event`]s
//! as on the desktop, answered from the [`Demo`] fleet.
//!
//! [Decision Log]
//! - Purpose: draw and type into the app's own screens from WebAssembly
//!   (docs/plans/backend-m12-browser.md) without changing a screen.
//! - Constraint: a browser runs one thread here (the single-threaded web
//!   platform), has no tokio runtime and no `vessel-client`; the shell
//!   awaits a `tokio::sync::mpsc::Receiver<Event>`, which needs no
//!   runtime (its waker is the executor's own).
//! - Alternatives: an executor task answering commands (needs a timer and
//!   a spawn the link does not own here); a `Link` trait with two
//!   implementations (a trait for one implementation per target is what
//!   `AGENTS.md` section 2 asks not to write).
//! - Choice: commands answered at once, inside [`Link::send`], with
//!   `try_send` into the bounded channel.
//! - Full channel: a command is taken only while the channel has room
//!   for its answers (two at most), else it comes back from
//!   [`Link::submit`] as the desktop link gives back a command its queue
//!   does not take; the channel holds 256 events and a person answers one
//!   prompt at a time, so only a stuck screen fills it.

use std::cell::RefCell;
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use tokio::sync::mpsc;
use vessel_app_view::machine_book::Name;
use vessel_proto::PairingLink;

use crate::attachments::Files;
use crate::demo::Demo;
use crate::{CHANNEL, Command, Event, Rejected, Unsent};

mod refuse;

/// The most events one command is answered with (a start: the fleet and
/// the card; a prompt: the records and the delivery).
const ANSWER_ROOM: usize = 2;

/// The link: the demo fleet and the channel its answers go out on.
#[derive(Debug)]
pub struct Link {
    demo: RefCell<Demo>,
    events: mpsc::Sender<Event>,
    presence: RefCell<Option<vessel_app_view::presence::Snapshot>>,
}

impl Link {
    /// Start the link; `_label` is the build's label (no node hears it
    /// in the demo). The fleet is on the receiver at once.
    ///
    /// # Errors
    /// None in a browser; the signature is the desktop's.
    pub fn start(_label: &str) -> std::io::Result<(Self, mpsc::Receiver<Event>)> {
        let (events, outbox) = mpsc::channel(CHANNEL);
        let link = Self {
            demo: RefCell::new(Demo::new(Utc::now())),
            events,
            presence: RefCell::new(None),
        };
        link.emit(Event::Fleet(link.demo.borrow().fleet(Utc::now())));
        Ok((link, outbox))
    }

    /// Ask for something and say whether the link took it; answered
    /// before this returns.
    ///
    /// # Errors
    /// The channel has no room for the answers, or the screen stopped
    /// reading it: the command, given back.
    pub fn submit(&self, command: Command) -> Result<(), Unsent> {
        if self.events.is_closed() {
            return Err(Unsent::new(Rejected::Closed, command));
        }
        if self.events.capacity() < ANSWER_ROOM {
            return Err(Unsent::new(Rejected::Full, command));
        }
        self.answer(command);
        Ok(())
    }

    /// This window's presence: the demo keeps the latest and has no machine
    /// to tell it to.
    pub fn presence(&self, snapshot: vessel_app_view::presence::Snapshot) {
        let _earlier = self.presence.replace(Some(snapshot));
    }

    /// Ask for something nothing waits on (a command not taken is dropped
    /// and logged).
    pub fn send(&self, command: Command) {
        if let Err(unsent) = self.submit(command) {
            tracing::warn!(why = %unsent.why, "the link is not taking commands");
        }
    }

    /// Answer `command` at once: the demo's own records, a node's refusal
    /// for what the demo cannot do ([`refuse`]), or nothing.
    ///
    /// One arm per command and no catch-all, so a new command does not
    /// build here until it is answered (`web_dispatch_tests` keeps that).
    fn answer(&self, command: Command) {
        let now = Utc::now();
        match command {
            Command::Reload => self.emit(Event::Fleet(self.demo.borrow().fleet(now))),
            Command::Open { session, .. } => {
                let records = self.demo.borrow().records(&session);
                self.emit(Event::Records { session, records });
            }
            Command::Prompt { session, text, .. } => self.demo_prompt(session, &text, now),
            Command::Start {
                machine,
                harness,
                prompt,
                ..
            } => self.demo_start(&machine, &harness, &prompt, now),
            Command::Options { machine, harness } => self.emit(Event::Options {
                machine,
                harness,
                outcome: Ok(crate::demo::options()),
            }),
            Command::Answer {
                session,
                id,
                option,
                ..
            } => {
                let records = self.demo.borrow_mut().answer(&session, id, option, now);
                self.emit(Event::Records { session, records });
            }
            Command::Files(files) => self.files(files, now),
            // The demo's turns end as they start: nothing runs to stop; a
            // browser keeps no files and holds no connection to retire.
            Command::Stop { .. }
            | Command::SavePrefs(_)
            | Command::KeepCommands { .. }
            | Command::SaveDrafts(_)
            | Command::RetireConnection(_)
            // The demo has no machine whose terminals ask for anyone, and
            // no inbox.
            | Command::ClearTerminalAttention { .. }
            | Command::ReadInbox(_) => {}
            Command::CodeSource(request) => self.emit(refuse::code_source(request)),
            Command::Usage(ask) => self.emit(refuse::usage(ask)),
            Command::SessionAccess(access) => self.emit(refuse::session_access(access)),
            Command::SessionLifecycle(request) => self.emit(refuse::session_lifecycle(&request)),
            Command::Sessions(ask) => self.emit(refuse::sessions(ask)),
            Command::Create(ask) => self.emit(refuse::create(ask)),
            Command::History(ask) => self.emit(refuse::history(ask)),
            Command::Labels(request) => self.emit(refuse::labels(request)),
            Command::Agents {
                machine, request, ..
            } => self.emit(refuse::agents(machine, request)),
            Command::Devices {
                context,
                ticket,
                ask,
            } => self.emit(refuse::devices(context, ticket, ask)),
            Command::Machines(ask) => self.emit(refuse::machines(ask)),
            Command::Sync { ticket, .. } => self.emit(refuse::sync(ticket)),
            Command::Claims(ask) => self.emit(Event::Claims(ask.failed())),
            Command::TerminalClaims(ask) => self.emit(Event::TerminalClaims(ask.failed())),
            Command::NodeSettings(ask) => self.emit(refuse::node_settings(ask)),
            Command::TerminalProfiles(ask) => self.emit(refuse::terminal_profiles(ask)),
            Command::Diagnostics(ask) => self.emit(refuse::diagnostics(ask)),
            Command::Schedules(request) => self.emit(Event::Schedules(
                crate::schedules::Answer::refused(request, vessel_app_view::schedules::load::Failure::NoTransport),
            )),
            Command::Plugins(ask) => self.emit(refuse::plugins(ask)),
            Command::PluginCatalog(ask) => self.emit(refuse::plugin_catalog(ask)),
            Command::Pair { .. } => self.emit(refuse::pair()),
            Command::Act {
                machine,
                session,
                act,
            } => self.emit(refuse::act(machine, session, &act)),
            Command::Resume { session, .. } => self.emit(refuse::resume(session)),
            Command::Workspace { machine, ask } => self.emit(refuse::workspace(machine, ask)),
            Command::Search(ask) => self.emit(refuse::search(ask)),
            Command::Terminals(ask) => {
                if let Some(event) = refuse::terminals(ask) {
                    self.emit(event);
                }
            }
        }
    }

    /// The file commands: a prompt with files is the demo's prompt (a
    /// browser sends none), the rest are refused.
    fn files(&self, files: Files, now: DateTime<Utc>) {
        match files {
            Files::Prompt(with) => self.demo_prompt(with.session, &with.text, now),
            Files::Send(attach) => self.emit(refuse::send(attach)),
            Files::Capabilities(ask) => self.emit(refuse::capabilities(ask)),
            Files::Wake(wake) => self.emit(refuse::wake(wake)),
            Files::Read(sent) => self.emit(refuse::read(sent)),
            Files::Create(_) => self.emit(refuse::create_files()),
            Files::Suggest(ask) => self.emit(refuse::suggest(ask)),
        }
    }

    /// The demo's records for a prompt, with files or without: a browser
    /// sends none, so both commands answer the same way.
    fn demo_prompt(&self, session: vessel_proto::SessionId, text: &str, now: DateTime<Utc>) {
        let records = self.demo.borrow_mut().prompt(&session, text, now);
        self.emit(Event::Records { session, records });
        self.emit(Event::Sent { session });
    }

    /// A new demo conversation: the fleet and its card, or why not.
    fn demo_start(
        &self,
        machine: &str,
        harness: &vessel_proto::HarnessRef,
        prompt: &str,
        now: DateTime<Utc>,
    ) {
        let started = self.demo.borrow_mut().start(machine, harness, prompt, now);
        match started {
            Some(card) => {
                self.emit(Event::Fleet(self.demo.borrow().fleet(now)));
                self.emit(Event::Started(card));
            }
            None => self.emit(Event::Failed(
                "The browser's demo has no room for another conversation".to_owned(),
            )),
        }
    }

    fn emit(&self, event: Event) {
        if let Err(error) = self.events.try_send(event) {
            tracing::warn!(%error, "the screen is not taking the link's events");
        }
    }
}

/// What a pairing link offers, for a person to check before pairing (the
/// desktop's fields).
#[derive(Debug, Clone)]
pub struct Offer {
    /// The key's fingerprint. `vessel-secure` computes it, which the
    /// browser build does not link yet: a placeholder says so.
    pub key: String,
    /// The node's id.
    pub node: String,
    /// Its routes, direct ones first.
    pub routes: vessel_app_view::pairing::OfferRoutes,
    /// When the link stops working, UTC.
    pub expires: String,
}

/// Decode a pasted pairing link (the wire's own decoder).
///
/// # Errors
/// The text is not a pairing link, or it does not decode.
pub fn read_link(text: &str) -> Result<Offer, vessel_proto::LinkError> {
    let offer = text
        .trim()
        .parse::<PairingLink>()
        .and_then(|link| link.decode())?;
    Ok(Offer {
        key: "(not computed in the browser build)".to_owned(),
        node: offer.node.to_string(),
        routes: vessel_app_view::pairing::OfferRoutes::of(&offer),
        expires: offer.expires.format("%Y-%m-%d %H:%M UTC").to_string(),
    })
}

/// A machine's name in the book, as a terminal's opening names it.
///
/// The desktop's is `vessel-client`'s `NodeName` (the book's own parser);
/// the browser build cannot link that crate, so this twin holds the view's
/// validated book name (`machine_book::Name`, the CLI's grammar). Same
/// methods, `FromStr` and `Display`, so a screen that parses or shows one
/// builds for both.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeName(Name);

impl NodeName {
    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl FromStr for NodeName {
    type Err = vessel_app_view::machines::NameProblem;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Name::parse(text).map(Self)
    }
}

impl fmt::Display for NodeName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0.as_str())
    }
}

/// What is wrong with a name typed for a new machine.
///
/// The desktop's twin (`nodes.rs`) checks the name with the node book's
/// own parser, which lives in `vessel-client`; the browser build cannot
/// link it yet, so "not a node name" is left to the node here. Same
/// variants, so a screen matching them builds for both.
pub use vessel_app_view::pairing::NameProblem;

/// What is wrong with `text` as a new machine's name, if anything.
#[must_use]
pub fn name_problem(text: &str, taken: &[String]) -> Option<NameProblem> {
    let text = text.trim();
    if text.is_empty() {
        return Some(NameProblem::Empty);
    }
    if text.starts_with("vessel://") {
        return Some(NameProblem::LooksLikeLink);
    }
    let hex = text.chars().filter(char::is_ascii_hexdigit).count();
    if text.contains(' ') && hex >= 8 && text.chars().all(|c| c.is_ascii_hexdigit() || c == ' ') {
        return Some(NameProblem::LooksLikeKey);
    }
    if taken.iter().any(|name| name == text) {
        return Some(NameProblem::Taken);
    }
    None
}

/// A name for a new machine that the book does not have.
#[must_use]
pub fn default_name(taken: &[String]) -> String {
    let free = |name: &String| !taken.contains(name);
    std::iter::once("machine".to_owned())
        .chain((2_u32..).map(|n| format!("machine-{n}")))
        .find(free)
        .unwrap_or_else(|| "machine".to_owned())
}
