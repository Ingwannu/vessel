//! Desktop-local intent and authenticated provenance; no process or file access.

use crate::machine_book::Name;
use serde::{Deserialize, Serialize};
use vessel_proto::NodeId;

pub mod names;

/// Deliberate desktop launch policy, persisted independently from node settings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Management {
    /// Start the packaged daemon when this desktop starts.
    #[default]
    Enabled,
    /// Explicit opt out; existing remote destinations remain usable.
    Paused,
}

/// Closing the desktop does not implicitly authorize interrupting live work.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AfterQuit {
    /// Graceful stop refuses active sessions until a person chooses otherwise.
    #[default]
    Stop,
    /// Explicitly consume ownership and leave the local node running.
    KeepRunning,
}

/// These choices are client-local, never replicated to a remote node.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    /// Automatic management at the next launch.
    pub management: Management,
    /// Finalization when the app closes.
    pub after_quit: AfterQuit,
}

/// Evidence of locality comes from the controller's explicit private home.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalProvenance {
    /// This app holds the actual spawned child handle.
    OwnedChild,
    /// Authenticated reuse at the managed home; no stop authority.
    AttachedManagedHome,
}

/// Published only after the link registers this exact authenticated connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalTarget {
    /// Authenticated greeting identity, never inferred from an address or alias.
    pub node: NodeId,
    /// Collision-checked ephemeral fleet destination, not a saved remote entry.
    pub machine: Name,
    /// Ownership evidence remains explicit to every consumer.
    pub provenance: LocalProvenance,
}

/// Safe admission refusals expose no saved credential, remote text or PID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionProblem {
    /// The bounded link queue did not accept the operation.
    Unsent,
    /// Actual authenticated identity differs from the fixed controller source.
    IdentityChanged,
    /// An explicit saved destination already owns the generated name.
    BookCollision,
    /// A distinct source cannot displace the one active local admission.
    LocalOccupied,
    /// The canonical allocator cannot mint another non-reused admission.
    AdmissionExhausted,
    /// The saved book could not be checked authoritatively.
    BookRead,
    /// The controller withdrew or lost the original transport.
    SourceRetired,
    /// This client cannot represent the requested local destination.
    Unavailable,
}

/// Local-node status displayed independently of the remote fleet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// This distribution does not yet carry the companion daemon.
    NotInstalled,
    /// Automatic management is explicitly paused.
    Paused,
    /// A controller operation has not finished.
    Working,
    /// The app retains its launched child handle; this alone does not promise liveness.
    Owned(NodeId),
    /// The same local home already had a node; the app only attached.
    Attached(NodeId),
    /// Explicit keep-running choice surrendered cleanup authority.
    Kept(NodeId),
    /// Actual direct-child exit was observed.
    Stopped,
    /// The reused connection closed, without claiming OS process exit.
    Disconnected,
    /// The operation failed; a typed cause remains in the link snapshot.
    Failed,
}
