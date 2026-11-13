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
//! - Full channel: an answer that finds no room is dropped and logged, as
//!   the desktop link drops a command; the channel holds 256 events and a
//!   person answers one prompt at a time, so only a stuck screen fills it.

use std::cell::RefCell;

use chrono::Utc;
use tokio::sync::mpsc;
use vessel_proto::PairingLink;

use crate::demo::Demo;
use crate::{CHANNEL, Command, Event, LinkError};

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

    /// Ask for something; answered before this returns.
    pub fn send(&self, command: Command) {
        let now = Utc::now();
        match command {
            Command::Devices { ticket, ask, .. } => self.emit(Event::Devices(ask.refused(
                ticket,
                vessel_app_view::devices::Problem::Refused(vessel_proto::ErrorCode::Unsupported),
            ))),
            Command::Machines(_) => self.emit(Event::Machines(crate::machines::Answer::Read(Err(
                vessel_app_view::machine_book::Failure::Unavailable,
            )))),
            Command::Reload => self.emit(Event::Fleet(self.demo.borrow().fleet(now))),
            Command::Open { session, .. } => {
                let records = self.demo.borrow().records(&session);
                self.emit(Event::Records { session, records });
            }
            Command::Prompt { session, text, .. } => {
                let records = self.demo.borrow_mut().prompt(&session, &text, now);
                self.emit(Event::Records { session, records });
                self.emit(Event::Sent { session });
            }
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
            // The demo's turns end as they start: nothing runs to stop.
            Command::Stop { .. } | Command::SavePrefs(_) => {}
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
                "Pairing from a browser is not built: the browser has no secure channel \
                 to a node yet (docs/plans/backend-m12-browser.md, question 8)"
                    .to_owned(),
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
    pub routes: Vec<String>,
    /// When the link stops working, UTC.
    pub expires: String,
}

/// Decode a pasted pairing link (the wire's own decoder).
///
/// # Errors
/// The text is not a pairing link, or it does not decode.
pub fn read_link(text: &str) -> Result<Offer, LinkError> {
    let offer = text
        .trim()
        .parse::<PairingLink>()
        .and_then(|link| link.decode())
        .map_err(|error| LinkError::PairingLink(error.to_string()))?;
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

/// What is wrong with a name typed for a new machine.
///
/// The desktop's twin (`nodes.rs`) checks the name with the node book's
/// own parser, which lives in `vessel-client`; the browser build cannot
/// link it yet, so "not a node name" is left to the node here. Same
/// variants, so a screen matching them builds for both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameProblem {
    /// Nothing typed.
    Empty,
    /// It is the node's key, typed where a name goes.
    LooksLikeKey,
    /// It is a pairing link.
    LooksLikeLink,
    /// Not a node name (never reported in a browser: see above).
    Invalid,
    /// The book already has a machine of that name.
    Taken,
}

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
