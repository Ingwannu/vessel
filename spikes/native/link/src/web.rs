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

use chrono::Utc;
use tokio::sync::mpsc;
use vessel_app_view::machine_book::Name;
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::PairingLink;

use crate::demo::Demo;
use crate::{ActAnswer, ActFailed, CHANNEL, Command, Event, Rejected, Unsent};

/// The most events one command is answered with (a start: the fleet and
/// the card; a prompt: the records and the delivery).
const ANSWER_ROOM: usize = 2;

/// The link: the demo fleet and the channel its answers go out on.
#[derive(Debug)]
pub struct Link {
    demo: RefCell<Demo>,
    events: mpsc::Sender<Event>,
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

    /// Ask for something nothing waits on (a command not taken is dropped
    /// and logged).
    pub fn send(&self, command: Command) {
        if let Err(unsent) = self.submit(command) {
            tracing::warn!(why = %unsent.why, "the link is not taking commands");
        }
    }

    /// The demo's records for a prompt, with files or without: a browser
    /// sends none, so both commands answer the same way.
    fn demo_prompt(
        &self,
        session: vessel_proto::SessionId,
        text: &str,
        now: chrono::DateTime<Utc>,
    ) {
        let records = self.demo.borrow_mut().prompt(&session, text, now);
        self.emit(Event::Records { session, records });
        self.emit(Event::Sent { session });
    }

    fn answer(&self, command: Command) {
        let now = Utc::now();
        match command {
            Command::CodeSource(request) => {
                self.emit(Event::CodeSource(Box::new(request.refused(Problem::Older))))
            }
            Command::SessionAccess(access) => self.emit(access.refused(ActFailed {
                code: Some(vessel_proto::ErrorCode::Unsupported),
                message: "Authenticated session navigation requires a native connection".to_owned(),
            })),
            Command::SessionLifecycle(request) => self.emit(Event::SessionLifecycle(
                crate::sessions::lifecycle::Answer {
                    ticket: request.ticket,
                    result: Err(crate::sessions::lifecycle::Failure::Unsent),
                },
            )),
            Command::Sessions(ask) => self.emit(Event::Sessions(
                ask.refused(crate::sessions::Failure::Offline),
            )),
            Command::Create(ask) => self.emit(Event::Creation(
                ask.refused(vessel_app_view::creation::Failure::Unsupported),
            )),
            Command::History(ask) => self.emit(Event::History(crate::history::Answer::refused(
                ask,
                vessel_app_view::history::Problem::Unsupported,
            ))),
            Command::Labels(request) => self.emit(Event::Labels(
                request.refused(vessel_app_view::labels::Failure::Unsupported),
            )),
            Command::Agents {
                machine, request, ..
            } => self.emit(Event::Agents {
                machine,
                request,
                answer: Err(crate::agents::AgentsError::NoTransport),
            }),

            Command::Devices {
                context,
                ticket,
                ask,
            } => self.emit(Event::Devices(ask.rejected(
                context,
                ticket,
                vessel_app_view::devices::Problem::Refused(vessel_proto::ErrorCode::Unsupported),
            ))),
            Command::Machines(ask) => self.emit(Event::Machines(
                ask.refused(vessel_app_view::machine_book::Failure::Unavailable),
            )),
            Command::Sync { ticket, .. } => self.emit(Event::Sync(crate::sync::Answer {
                ticket,
                result: Err(vessel_app_view::sync::Problem::Refused(
                    vessel_proto::ErrorCode::Unsupported,
                )),
            })),
            Command::NodeSettings(ask) => self.emit(Event::NodeSettings(ask.refused(
                vessel_app_view::idle_release::Problem::Refused(
                    vessel_proto::ErrorCode::Unsupported,
                ),
            ))),
            Command::RetireConnection(_tag) => {}
            Command::Reload => self.emit(Event::Fleet(self.demo.borrow().fleet(now))),
            Command::Open { session, .. } => {
                let records = self.demo.borrow().records(&session);
                self.emit(Event::Records { session, records });
            }
            Command::Prompt { session, text, .. } => self.demo_prompt(session, &text, now),
            Command::Files(crate::attachments::Files::Prompt(with)) => {
                self.demo_prompt(with.session, &with.text, now);
            }
            // A browser has no files to send yet (track P's row c).
            Command::Files(crate::attachments::Files::Send(attach)) => {
                self.emit(Event::Files(crate::attachments::Answer::Upload {
                    message: attach.message,
                    session: attach.session,
                    key: attach.key,
                    step: crate::attachments::Step::Failed {
                        problem: vessel_app_view::composer::tray::Problem::Node(
                            "The browser's demo takes no files".to_owned(),
                        ),
                        again: vessel_app_view::composer::tray::Again::Never,
                    },
                }));
            }
            // Nor any sent file to read back.
            Command::Files(crate::attachments::Files::Capabilities(ask)) => {
                self.emit(Event::Files(crate::attachments::Answer::Capabilities {
                    ask,
                    result: Err(crate::attachments::CapabilityFailure::Offline),
                }))
            }
            Command::Files(crate::attachments::Files::Wake(wake)) => {
                self.emit(Event::Files(crate::attachments::Answer::Woke {
                    message: wake.message,
                    session: wake.session,
                    outcome: crate::attachments::WakeOutcome::Refused,
                }));
            }
            Command::Files(crate::attachments::Files::Read(sent)) => {
                self.emit(Event::Files(crate::attachments::Answer::Read {
                    sent,
                    bytes: None,
                }));
            }
            Command::Files(crate::attachments::Files::Create(_)) => self.emit(Event::Failed(
                "The browser's demo takes no files".to_owned(),
            )),
            Command::Start {
                machine,
                harness,
                prompt,
                ..
            } => {
                let started = self
                    .demo
                    .borrow_mut()
                    .start(&machine, &harness, &prompt, now);
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
            Command::Options { machine, harness } => self.emit(Event::Options {
                machine,
                harness,
                outcome: Ok(crate::demo::options()),
            }),
            // The demo's turns end as they start: nothing runs to stop; a
            // browser keeps no files.
            Command::Stop { .. }
            | Command::SavePrefs(_)
            | Command::KeepCommands { .. }
            | Command::SaveDrafts(_) => {}
            Command::Answer {
                session,
                id,
                option,
                ..
            } => {
                let records = self.demo.borrow_mut().answer(&session, id, option, now);
                self.emit(Event::Records { session, records });
            }
            Command::Pair { .. } => self.emit(Event::PairFailed(
                vessel_app_view::pairing::PairProblem::Browser,
            )),
            // The demo's sessions are its own records, not a node's: an act
            // on one, starting one again or a workspace ask is refused as a
            // node refuses what it does not support, on the control it is
            // about.
            Command::Act {
                machine,
                session,
                act,
            } => self.emit(Event::Acted(ActAnswer {
                terminal: act.terminal_opening().cloned(),
                machine,
                session,
                kind: act.kind(),
                result: Err(ActFailed {
                    code: Some(vessel_proto::ErrorCode::Unsupported),
                    message: "The browser's demo takes no session actions".to_owned(),
                }),
            })),
            Command::Resume { session, .. } => self.emit(Event::ResumeFailed {
                session,
                why: "The browser's demo cannot start a session again".to_owned(),
            }),
            Command::Workspace { machine, ask } => self.emit(Event::Workspace {
                machine,
                answer: ask.refused(&Problem::Node {
                    code: vessel_proto::ErrorCode::Unsupported,
                    message: "The browser's demo has no projects".to_owned(),
                }),
            }),
            // The demo fleet keeps no search index: every machine finds
            // nothing.
            Command::Files(crate::attachments::Files::Suggest(crate::mentions::Ask::Root(ask))) => {
                self.emit(Event::Files(crate::attachments::Answer::Root(
                    crate::mentions::RootAnswer {
                        ask,
                        result: Err(vessel_app_view::composer::mentions::Failure::Unsupported),
                    },
                )))
            }
            Command::Files(crate::attachments::Files::Suggest(
                crate::mentions::Ask::Suggestions(request),
            )) => self.emit(Event::Files(crate::attachments::Answer::Suggestions(
                crate::mentions::Answer {
                    request,
                    result: Err(vessel_app_view::composer::mentions::Failure::Unsupported),
                },
            ))),
            Command::Search(ask) => self.emit(Event::Searched(crate::search::Answer {
                machine: ask.machine,
                ticket: ask.ticket,
                result: Ok(vessel_proto::search::SearchPage {
                    hits: Vec::new(),
                    next: None,
                    reached: None,
                }),
            })),
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
    pub routes: Vec<String>,
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
        routes: offer
            .direct
            .iter()
            .chain(&offer.relays)
            .map(ToString::to_string)
            .collect(),
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
