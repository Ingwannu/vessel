//! The frames that carry calls, replies and subscription events, and the
//! handshake.

use serde::{Deserialize, Serialize};

use super::{Call, Reply};
use crate::error::WireError;
use crate::event::EventRecord;
use crate::id::{EventSeq, RequestId, SubscriptionId};
use crate::node::{NodeInfo, ProtocolRange, ProtocolVersion};

/// Who is connecting; for logs and future per-client capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ClientInfo {
    /// Client program name (`vessel-cli`, `vessel-web`, ...).
    pub name: String,
    /// Client program version.
    pub version: String,
}

/// First frame a client sends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Hello {
    /// Protocols the client speaks.
    pub protocol: ProtocolRange,
    /// The client.
    pub client: ClientInfo,
}

/// The node's answer to a successful `Hello`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Welcome {
    /// The negotiated protocol version for this connection.
    pub protocol: ProtocolVersion,
    /// The node.
    pub node: NodeInfo,
}

/// Why a subscription stopped delivering events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum SubscriptionEnd {
    /// The session ended and its final event was delivered.
    SessionEnded,
    /// The client unsubscribed.
    Unsubscribed,
    /// The client fell too far behind the live stream. Nothing was skipped:
    /// read or resubscribe from `resume_from`.
    Lagged {
        /// First sequence number not delivered.
        resume_from: EventSeq,
    },
}

/// A frame from client to node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientFrame {
    /// Handshake; must be the first frame.
    Hello(Hello),
    /// A call.
    Request {
        /// Correlation id, unique per connection.
        id: RequestId,
        /// The call.
        call: Call,
    },
    /// Abandon an in-flight call; the node may still answer it.
    Cancel {
        /// The call's id.
        id: RequestId,
    },
}

/// A frame from node to client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
#[expect(
    clippy::large_enum_variant,
    reason = "a frame is built, encoded and dropped, as `Outcome` is (protocol 9's \
              titles and options made `SessionInfo`, and so a response, tip the lint); \
              boxing the outcome would add an allocation to every call to save a copy"
)]
pub enum ServerFrame {
    /// Handshake accepted.
    Welcome(Welcome),
    /// Handshake refused (for example `ProtocolMismatch`); the node closes the
    /// connection after sending this.
    Rejected {
        /// Why.
        error: WireError,
    },
    /// The answer to a call.
    Response {
        /// The call's id.
        id: RequestId,
        /// What happened.
        outcome: Outcome,
    },
    /// A subscribed session event.
    Event {
        /// Which subscription.
        subscription: SubscriptionId,
        /// The record.
        record: EventRecord,
    },
    /// A subscription stopped; no more events carry its id.
    SubscriptionEnded {
        /// Which subscription.
        subscription: SubscriptionId,
        /// Why.
        end: SubscriptionEnd,
    },
}

impl ServerFrame {
    /// This frame as a client speaking `protocol` reads it, or `None` when
    /// it needs no change: events newer than the connection are replaced by
    /// their stand-ins (`SessionEvent::for_protocol`), in subscription
    /// events and in replies (`Reply::for_protocol`).
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        match self {
            Self::Event {
                subscription,
                record,
            } => record.for_protocol(protocol).map(|record| Self::Event {
                subscription: *subscription,
                record,
            }),
            Self::Response {
                id,
                outcome: Outcome::Ok(reply),
            } => match reply.withheld(protocol) {
                // About one agent the client could not decode (protocol
                // 11, ADR 0032): it gets the reason instead.
                Some(error) => Some(Self::Response {
                    id: *id,
                    outcome: Outcome::Err(error),
                }),
                None => reply.for_protocol(protocol).map(|reply| Self::Response {
                    id: *id,
                    outcome: Outcome::Ok(reply),
                }),
            },
            Self::Welcome(welcome) => welcome.node.for_protocol(protocol).map(|node| {
                Self::Welcome(Welcome {
                    protocol: welcome.protocol,
                    node,
                })
            }),
            Self::Rejected { .. }
            | Self::Response {
                outcome: Outcome::Err(_),
                ..
            }
            | Self::SubscriptionEnded { .. } => None,
        }
    }
}

/// Result of a call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
#[expect(
    clippy::large_enum_variant,
    reason = "one Outcome per call, moved once to the writer and never stored in bulk; \
              boxing the reply would add an allocation to every call to save a copy"
)]
pub enum Outcome {
    /// Success.
    Ok(Reply),
    /// Failure.
    Err(WireError),
}
