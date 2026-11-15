//! Scoped read state, with no toolkit or I/O.
use super::Snapshot;
use crate::devices::{Target, Ticket, Tickets};
use vessel_proto::ErrorCode;

/// Safe failures at the log/screen boundary; raw peer text is discarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// The connection or command queue has closed.
    #[error("the selected machine is not connected")]
    Offline,
    /// A refusal by structured code only.
    #[error("the sync read was refused ({0:?})")]
    Refused(ErrorCode),
    /// Invalid data, without retaining the untrusted reply.
    #[error("the sync reply was invalid")]
    InvalidReply,
    /// Request identities cannot be reused safely.
    #[error("request identities exhausted")]
    Exhausted,
}
/// Which independent read a failure belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// Loading the page as a whole failed before independent reads.
    Overview,
    /// Shared names and file metadata.
    Publication,
    /// Member reports.
    Members,
    /// Fleet connectivity and home information.
    Connections,
    /// Stored secret names.
    Secrets,
}
/// One correlated read of a selected machine.
#[derive(Debug, Default)]
pub enum Reading {
    /// No read issued in this scope.
    #[default]
    Empty,
    /// Awaiting this request only.
    Loading(Ticket),
    /// A safe snapshot, including partial failures.
    Ready(Box<Snapshot>),
    /// Dispatch failed before a snapshot could be read.
    Failed(Problem),
}
impl Reading {
    /// Accept only the still-outstanding request.
    pub fn answer(&mut self, ticket: Ticket, result: Result<Box<Snapshot>, Problem>) {
        if matches!(self, Self::Loading(expected) if *expected == ticket) {
            *self = match result {
                Ok(snapshot) => Self::Ready(snapshot),
                Err(why) => Self::Failed(why),
            };
        }
    }
}
/// A per-machine page; the non-reused ticket source survives navigation.
#[derive(Debug, Default)]
pub struct Page {
    /// Selected machine availability, shared with Devices.
    pub target: Target,
    /// Its most recent read.
    pub reading: Reading,
    /// Request identities never reset when the selected machine changes.
    pub tickets: Tickets,
}
impl Page {
    /// Drop replies belonging to a previous scope.
    pub fn target(&mut self, target: Target) {
        if self.target != target {
            self.reading = Reading::Empty;
            self.target = target;
        }
    }
}
/// Local CLI guidance only; the app never executes these commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalCommand {
    /// Make a home from the node's own data directory.
    Init,
    /// Create a fleet invitation on the home machine.
    Invite,
    /// Join on the receiving machine, pasting a link at its prompt.
    Join,
    /// Leave on the member machine; shared files remain.
    Leave,
    /// Inspect local sync state; there is no sync-now command or RPC.
    Status,
}
impl LocalCommand {
    /// All available local guidance, no invented remote action.
    pub const ALL: [Self; 5] = [
        Self::Init,
        Self::Invite,
        Self::Join,
        Self::Leave,
        Self::Status,
    ];
    /// Exact commands supported by the existing CLI.
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            Self::Init => "vessel fleet init",
            Self::Invite => "vessel fleet invite",
            Self::Join => "vessel fleet join",
            Self::Leave => "vessel fleet leave",
            Self::Status => "vessel fleet status",
        }
    }
}

impl Problem {
    /// Read failures are error notices; progress is never a warning.
    #[must_use]
    pub const fn notice(self) -> crate::alert::Variant {
        match self {
            Self::Offline | Self::Refused(_) | Self::InvalidReply | Self::Exhausted => {
                crate::alert::Variant::Error
            }
        }
    }
}
