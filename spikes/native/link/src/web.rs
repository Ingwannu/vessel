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
            Command::Reload => self.emit(Event::Fleet(self.demo.borrow().fleet(now))),
            Command::Open { session, .. } => {
                let records = self.demo.borrow().records(&session);
                self.emit(Event::Records { session, records });
            }
            Command::Prompt { session, text, .. } => {
                let records = self.demo.borrow_mut().prompt(&session, &text, now);
                self.emit(Event::Records {
                    session: session.clone(),
                    records,
                });
                self.emit(Event::Sent { session });
            }
            Command::Answer {
                session,
                id,
                option,
                ..
            } => {
                let records = self.demo.borrow_mut().answer(&session, id, option, now);
                self.emit(Event::Records { session, records });
            }
            Command::Pair { .. } => self.emit(Event::Failed(
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

/// What a pairing link offers, for a person to check before pairing.
#[derive(Debug, Clone)]
pub struct Offer {
    /// A line to show: id, routes, expiry.
    pub summary: String,
}

/// Decode a pasted pairing link (the wire's own decoder). The key's
/// fingerprint is computed by `vessel-secure`, which the browser build
/// does not link yet, so the line shows the routes and expiry only.
///
/// # Errors
/// The text is not a pairing link, or it does not decode.
pub fn read_link(text: &str) -> Result<Offer, LinkError> {
    let offer = text
        .trim()
        .parse::<PairingLink>()
        .and_then(|link| link.decode())
        .map_err(|error| LinkError::PairingLink(error.to_string()))?;
    let routes: Vec<String> = offer
        .direct
        .iter()
        .chain(&offer.relays)
        .map(ToString::to_string)
        .collect();
    Ok(Offer {
        summary: format!(
            "node {} · via {} · expires {}",
            offer.node,
            routes.join(", "),
            offer.expires.format("%Y-%m-%d %H:%M UTC"),
        ),
    })
}
