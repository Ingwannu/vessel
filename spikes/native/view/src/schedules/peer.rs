//! Which node a schedule lives on, and how this device reaches it.
//!
//! The list spans every node the app shows. A node the app can call only
//! through `fleet.forward` is not the paired device: create, update and run
//! are refused there (ADR 0072).

use std::sync::Arc;

use vessel_proto::automation::TimeZoneName;

/// A node-book name the book already parsed. The link's connection map is
/// keyed by that name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BookName(Arc<str>);

impl BookName {
    /// Carry a name the book has already accepted.
    #[must_use]
    pub fn from_book(name: &str) -> Self {
        Self(Arc::from(name))
    }

    /// The connection-map key.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Correlates one schedule call with its answer, so a late reply cannot be
/// applied to a different ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ticket(u64);

impl Ticket {
    /// `value` is the screen's own counter, not a protocol id.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// The counter the screen minted.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Whether `schedule.preview` can be asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewLane {
    /// The handshake is older than protocol 13.
    Older,
    /// Protocol 13 or newer.
    Offered,
}

/// How the app reaches the node that holds the schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleReach {
    /// This device's own session on that node.
    Direct,
    /// Only through `fleet.forward`.
    Forwarded,
}

/// What a forwarded caller must not send. Pause, resume and delete are not
/// in this set: a delegated person may those.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockedAct {
    /// `schedule.create`.
    Create,
    /// `schedule.update`.
    Update,
    /// `schedule.run`.
    Run,
}

/// Whether the node's session directory has loaded. The badge does not
/// read this: a missing session is the node's `TargetGone`, not a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionDirectory {
    /// The directory has never loaded. A missing agent is not "gone".
    Pending,
    /// The directory has loaded at least once.
    Loaded,
}

/// Schedules on this node, as the handshake and the path allow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulePeer {
    /// The handshake is older than protocol 11.
    Older,
    /// Schedules are on this machine.
    Ready(ReadyPeer),
}

/// A node whose handshake offers schedules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadyPeer {
    /// Whether a preview can be asked.
    pub preview: PreviewLane,
    /// The machine's zone, when the node could name one. Absence is not UTC.
    pub zone: Option<TimeZoneName>,
    /// Direct session, or only a forwarded call.
    pub reach: ScheduleReach,
}
