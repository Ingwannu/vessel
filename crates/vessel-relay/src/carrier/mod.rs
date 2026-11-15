//! Websockets as [`Carrier`]s for the secure channel: [`WsCarrier`] for
//! the websockets nodes and clients open (to a relay, or to a node's
//! `/secure`), [`AxumCarrier`] for the ones a node accepts on `/secure`.
//!
//! [Decision Log]
//! - Purpose: the channel needs whole binary messages; nodes and clients
//!   also need to know why a connection was closed: a relay's code (`4404`
//!   means the node may be restarting, `4429` to slow down) or a node's
//!   (`4401`, the handshake failed: pair again).
//! - Choice: a close frame with one of those codes is an error carrying
//!   [`RelayClosed`] or [`NodeClosed`], found again through
//!   `SecureError::Carrier` with [`relay_close`] and [`node_close`]. Any
//!   other close ends the stream. A text message is an error: these
//!   connections carry records only.
//! - Keepalive ([`Keepalive`], one set of numbers): one end pings every
//!   10 s and both ends fail a connection after 30 s of silence
//!   (`TimedOut`), pings and pongs included, so a half-open connection (a
//!   laptop that slept, a NAT mapping that expired) cannot hang a reader.
//!   Through a relay, the relay pings both sides; on a direct route the
//!   node pings its `/secure` connections ([`AxumCarrier`] with a
//!   keepalive). Either way the client's carrier has the same silence
//!   limit, so a client treats a direct route and a relayed one alike, and
//!   the reconnecting stream takes over when it fires.
//! - Pings from inside a carrier: the channel owns the carrier, and the RPC
//!   layer splits it into a reading and a writing task, so a ping is sent
//!   from whichever side is polled: the reading side when nothing is being
//!   written (a quiet connection, the case that needs pings), the writing
//!   side before its next record otherwise. The reading side never touches
//!   the socket's write half while a write is in flight: the websocket
//!   keeps one waker per direction, and taking it would leave the writing
//!   task asleep.
//! - The node's handshake: with NK only the node notices a client that
//!   pinned the wrong key, so [`respond`] closes with `4401` when the
//!   handshake fails (`vessel_proto::NodeClose`), and relays forward it.
//! - One module for both websocket libraries: the node's `/secure` (axum)
//!   and its relayed connections (tungstenite) take the same path into
//!   [`respond`].

mod accepted;
mod liveness;
mod opened;

use std::future::Future;
use std::io;
use std::time::Duration;

use vessel_proto::{NodeClose, RelayClose};
use vessel_secure::{
    Carrier, Config, NodeKeypair, RespondError, SecureChannel, SecureError, try_respond,
};

pub use self::accepted::AxumCarrier;
use self::liveness::Idle;
pub use self::liveness::Liveness;
pub use self::opened::WsCarrier;

/// A relay closed the connection with one of its close codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the relay closed the connection ({})", describe_relay(*.0))]
pub struct RelayClosed(pub RelayClose);

/// A node closed the connection with one of its close codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the node closed the connection ({})", describe_node(*.0))]
pub struct NodeClosed(pub NodeClose);

fn describe_relay(close: RelayClose) -> &'static str {
    match close {
        RelayClose::NoSuchNode => "the node is not connected to it",
        RelayClose::NotAccepted => "the node did not accept in time",
        RelayClose::TooLarge => "a message was too large",
        RelayClose::RateLimited => "rate limited",
        RelayClose::Full => "the relay is at its limit of connections",
    }
}

fn describe_node(close: NodeClose) -> &'static str {
    match close {
        NodeClose::HandshakeFailed => {
            "the secure handshake failed: the node's key is not the one pinned; pair it again"
        }
    }
}

/// The relay close code inside a carrier error, if that is why it failed.
#[must_use]
pub fn relay_close(error: &io::Error) -> Option<RelayClose> {
    error
        .get_ref()?
        .downcast_ref::<RelayClosed>()
        .map(|closed| closed.0)
}

/// The node close code inside a carrier error, if that is why it failed.
/// `HandshakeFailed` is final: the client pinned another key.
#[must_use]
pub fn node_close(error: &io::Error) -> Option<NodeClose> {
    error
        .get_ref()?
        .downcast_ref::<NodeClosed>()
        .map(|closed| closed.0)
}

/// What a close frame with `code` means to a reader: an error for the
/// relay's and the node's codes, a plain end otherwise.
fn closed(code: Option<u16>) -> Option<io::Error> {
    let code = code?;
    if let Some(close) = RelayClose::from_code(code) {
        return Some(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            RelayClosed(close),
        ));
    }
    NodeClose::from_code(code)
        .map(|close| io::Error::new(io::ErrorKind::ConnectionRefused, NodeClosed(close)))
}

fn text_message() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "a text message on a secure-channel connection",
    )
}

/// How a connection is kept alive: one end pings every `ping`, and either
/// end gives up after `silence` without a message. The same numbers for a
/// relay's connections, a node's `/secure` and every client carrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keepalive {
    /// How often the pinging end pings.
    pub ping: Duration,
    /// Silence after which either end drops the connection: three pings
    /// missed.
    pub silence: Duration,
}

impl Keepalive {
    /// Ping every 10 s, give up after 30 s of silence.
    pub const DEFAULT: Self = Self {
        ping: Duration::from_secs(10),
        silence: Duration::from_secs(30),
    };
}

impl Default for Keepalive {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A carrier the node can close with a reason the client reads.
pub trait NodeCarrier: Carrier {
    /// Close with `close`'s code, then read and discard what the peer
    /// still sends until its own close or the end (best effort, all within
    /// [`CLOSE_TIMEOUT`]).
    ///
    /// Why the reading: a socket dropped with bytes it has not read is
    /// reset, not closed, and on Windows the reset throws away what is
    /// still in flight to the peer, this close frame among it. A client
    /// whose handshake failed, or a relay forwarding a node's `4401`, would
    /// then never learn why. The relay does the same for its own sockets
    /// (`server::accept`'s `linger`).
    fn close_with(self, close: NodeClose) -> impl Future<Output = ()> + Send;
}

/// How long closing with a code may take, reading until the peer's close
/// included.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(1);

/// The node's side of a secure channel on a websocket:
/// [`vessel_secure::respond_with`], closing the websocket with
/// `NodeClose::HandshakeFailed` (`4401`) when the handshake itself fails,
/// so a client that pinned another key stops retrying.
///
/// # Errors
/// As [`vessel_secure::respond`].
pub async fn respond<C: NodeCarrier>(
    carrier: C,
    keypair: &NodeKeypair,
    config: &Config,
) -> Result<SecureChannel<C>, SecureError> {
    match try_respond(carrier, keypair, config).await {
        Ok(channel) => Ok(channel),
        Err(failed) => {
            let RespondError { error, carrier, .. } = failed;
            if matches!(
                error,
                SecureError::Handshake(_) | SecureError::HandshakeMessage { .. }
            ) {
                carrier.close_with(NodeClose::HandshakeFailed).await;
            }
            Err(error)
        }
    }
}
