//! Cross-machine attention state. Nodes own persistence and resolution; this
//! model owns stale-result rejection, selection and notification eligibility.
//! It never reconstructs an inbox entry from a session's status.

pub mod admission;
pub mod delivery;
mod detail;
pub mod notices;
mod pages;
pub mod presence;
pub mod reading;
pub mod settings;
mod state;
pub use pages::{MAX_PAGES, Pages};
pub mod words;
pub use detail::{Detail, Operation};
#[cfg(test)]
mod tests;
pub use state::{Extent, Inbox, Listing, Node, Request, Snapshot};

use chrono::{DateTime, Utc};
use vessel_proto::{
    EventSeq, NodeId, PermissionRequest, Question, SessionId, SessionTitle, ToolCall,
};

use crate::machine_book::Name;

/// An authenticated transport incarnation, generated at connection setup.
/// A reconnect to the same node must still invalidate an older queued read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionId([u8; 16]);
impl ConnectionId {
    /// Opaque random bytes supplied by the link's connection boundary.
    #[must_use]
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

/// A node identity and its already validated book route. The identity, rather
/// than a reused name, separates one machine's attention from another's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// This authenticated connection, distinct after every reconnect.
    pub connection: ConnectionId,
    /// The authenticated reporting node.
    pub node: NodeId,
    /// Its route in the local node book.
    pub name: Name,
}

/// An exact attention source, safe across machines and repeated requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    /// Reporting node; sessions on two nodes never share read state.
    pub node: NodeId,
    /// Session owning the source record.
    pub session: SessionId,
    /// Event that first requested attention, not the latest session event.
    pub seq: EventSeq,
}

/// Whether this connection can answer, separately from reading/marking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authority {
    /// The node explicitly reports operate scope.
    Operate,
    /// No operate scope; reading and marking remain available.
    ReadOnly,
}
impl Authority {
    /// Effective scopes are reported by device.self, never inferred from
    /// authentication success or from a peer's display label.
    #[must_use]
    pub fn of(scopes: &[vessel_proto::Scope]) -> Self {
        if scopes.contains(&vessel_proto::Scope::Operate) {
            Self::Operate
        } else {
            Self::ReadOnly
        }
    }
}

/// Reading attention is independent of answering a live request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Read {
    /// Still unseen.
    Unread,
    /// Explicitly seen on a paired client.
    Read,
}

/// What the person can do with an entry; compact/future payloads remain
/// openable in the source session without guessed answer buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    /// Reuse the existing permission card and its own options.
    Permission {
        /// The actor's request.
        request: PermissionRequest,
        /// Optional tool detail, projected without a synthesized log event.
        tool: Option<ToolCall>,
        /// Whether complete tool details are available here or only in history.
        detail: vessel_proto::inbox::InboxDetail,
    },
    /// Reuse the existing question form and checker.
    Question(Question),
    /// Oversized form stays in the source session.
    InSession,
    /// A normal completed turn.
    Finished,
    /// A failed turn or process.
    Failed,
    /// A usage limit.
    Limit,
    /// A future node item; readable, never actionable.
    Unknown,
}
impl Content {
    /// Whether this entry waits for a response even after it is read.
    #[must_use]
    pub const fn waiting(&self) -> bool {
        match self {
            Self::Permission { .. } | Self::Question(_) | Self::InSession => true,
            Self::Finished | Self::Failed | Self::Limit | Self::Unknown => false,
        }
    }

    /// Notification eligibility mirrors Paseo: errors stay in the inbox.
    #[must_use]
    pub const fn announces(&self) -> bool {
        match self {
            Self::Permission { .. }
            | Self::Question(_)
            | Self::InSession
            | Self::Finished
            | Self::Limit => true,
            Self::Failed | Self::Unknown => false,
        }
    }
}

/// Monotonic position assigned by the reporting node, independent of time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position(u64);
impl Position {
    /// Keep a node-issued cursor at the transport edge.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    /// Its wire representation for continuing an existing page.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Presentation data decoded by link from one existing node inbox entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Stable monotonic inbox position on this node.
    pub position: Position,
    /// Exact source identity.
    pub key: Key,
    /// The current session's optional title.
    pub title: Option<SessionTitle>,
    /// When the source first requested attention.
    pub at: DateTime<Utc>,
    /// Shared read state, authoritative on the node.
    pub read: Read,
    /// The request or outcome.
    pub content: Content,
}

/// Whole-node counts. Never derive these from a partial page of entries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    total: u64,
    waiting: u64,
    unread: u64,
}
impl Counts {
    /// Keep the node's exact independent counts at the decoding edge.
    #[must_use]
    pub const fn new(total: u64, waiting: u64, unread: u64) -> Self {
        Self {
            total,
            waiting,
            unread,
        }
    }
    /// Number of retained entries on the node.
    #[must_use]
    pub const fn total(self) -> u64 {
        self.total
    }
    /// Pending requests including read ones.
    #[must_use]
    pub const fn waiting(self) -> u64 {
        self.waiting
    }
    /// Unseen attention, independent of pending requests.
    #[must_use]
    pub const fn unread(self) -> u64 {
        self.unread
    }
    /// Aggregate nodes without wrapping if a faulty peer reports huge counts.
    #[must_use]
    pub const fn plus(self, other: Self) -> Self {
        Self::new(
            self.total.saturating_add(other.total),
            self.waiting.saturating_add(other.waiting),
            self.unread.saturating_add(other.unread),
        )
    }
}

/// Safe failures; raw transport or node text never reaches an inbox alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// No current connection.
    #[error("machine offline")]
    Offline,
    /// RPC error code, without the peer's potentially sensitive message.
    #[error("inbox request refused: {0:?}")]
    Refused(vessel_proto::ErrorCode),
    /// A decoded reply violated the presentation contract.
    #[error("invalid inbox reply")]
    InvalidReply,
    /// The non-repeating local request counter is exhausted.
    #[error("inbox request identities exhausted")]
    Exhausted,
}
