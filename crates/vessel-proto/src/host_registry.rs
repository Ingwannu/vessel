//! Host directory references existing trust/transport authorities; no credentials live here.
//! New RPC registration waits for Quality B and the integrator's feature version.

mod shared;
mod text;
pub use crate::id::HostRegistrationId;
use crate::{DeviceId, NodeId, ProtocolVersion};
use serde::{Deserialize, Serialize};
pub use shared::{
    ListSharedHostRegistrations, SharedHostDirectory, SharedHostRegistration,
    SharedHostRegistrations, TooManySharedHosts,
};
pub use text::{
    HostLabel, HostMetadata, InvalidHostText, LABEL_CHARS, METADATA_BYTES, METADATA_ENTRIES,
    METADATA_KEY_CHARS, METADATA_VALUE_CHARS, MetadataKey, MetadataValue,
};

/// One package version covers CLI host registration and terminal capture additions.
pub const CLI_PARITY_PROTOCOL: Option<ProtocolVersion> = None;
/// Admission budget is separate from connectivity's own existing node/device limits.
pub const MAX_HOST_REGISTRATIONS: usize = 256;

/// A registration points at established destinations, never grants access itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HostDestination {
    /// Existing fleet node and its current routing/policy authority.
    Fleet {
        /// Stable node identity, not its display name.
        node: NodeId,
    },
    /// Existing paired-node link; the registry contains no device secret.
    Paired {
        /// Already authenticated peer identity.
        node: NodeId,
    },
}
impl HostDestination {
    /// Compare destination identity independently of its transport path.
    #[must_use]
    pub const fn node(self) -> NodeId {
        match self {
            Self::Fleet { node } | Self::Paired { node } => node,
        }
    }
}

/// The node derives ownership from authentication, never a client-provided source id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HostSource {
    /// Owner token or local authenticated socket.
    Owner,
    /// Paired device controlling its own registration source.
    Device {
        /// Authenticated device, not a target destination.
        device: DeviceId,
    },
}
/// Persisted enablement never revokes a destination's underlying credentials.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum HostEnabled {
    /// Eligible for connecting through its existing authority.
    #[default]
    Enabled,
    /// Retained in the directory but not selected for automatic connections.
    Disabled,
}
/// Rebinding to another node identity requires this explicit user choice.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum IdentityChange {
    /// Different identity is a conflict.
    #[default]
    KeepPinned,
    /// The owner explicitly accepted the new identity.
    Replace,
}
/// Label patch keeps omission distinct from clearing a custom label.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", content = "label", rename_all = "snake_case")]
pub enum HostLabelChange {
    /// Preserve the label.
    #[default]
    Keep,
    /// Revert to the destination label.
    Clear,
    /// Set validated display text.
    Set(HostLabel),
}
/// Revision prevents two CLI metadata merges from silently losing changes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct HostRevision(u64);
impl HostRevision {
    /// Optimistic updates refuse exhaustion instead of wrapping to an old revision.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}
/// The durable registry record carries only refs and display annotations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct HostRegistration {
    /// Opaque registry identity survives connection replacement.
    pub registration_id: HostRegistrationId,
    /// Authentication-derived source ownership.
    pub source: HostSource,
    /// Destination's existing trust authority.
    pub destination: HostDestination,
    /// Custom label; omitted follows destination display name.
    pub label: Option<HostLabel>,
    /// Public bounded display annotations; local agents may read them across sources.
    pub metadata: HostMetadata,
    /// Directory state, independent of device revocation/fleet membership.
    pub enabled: HostEnabled,
    /// Optimistic metadata/identity updates compare this value.
    pub revision: HostRevision,
}
/// List includes disabled and unreachable registrations; connectivity is a projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListHostRegistrations;
/// Source is deliberately absent from an unauthenticated request payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct AddHostRegistration {
    /// Resolve an existing trusted node before adding display metadata.
    pub destination: HostDestination,
    /// An absent custom label follows the trusted destination label.
    pub label: Option<HostLabel>,
    /// Public display metadata readable by local agents; never credentials or private config.
    pub metadata: HostMetadata,
}
/// An edit compares the whole prior record's revision before changing anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct UpdateHostRegistration {
    /// Directory entry, never a node id.
    pub registration_id: HostRegistrationId,
    /// Reject a stale CLI metadata merge.
    pub expected_revision: HostRevision,
    /// Keep, clear or replace the display label.
    pub label: HostLabelChange,
    /// None preserves annotations; replacements are public to local agents, never secrets.
    pub metadata: Option<HostMetadata>,
    /// None preserves enabled state.
    pub enabled: Option<HostEnabled>,
    /// None preserves the current destination ref.
    pub destination: Option<HostDestination>,
    /// Explicit consent if the destination's `NodeId` changes.
    pub identity_change: IdentityChange,
}
/// Removing metadata never forgets a fleet node or revokes a paired device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RemoveHostRegistration {
    /// Directory entry to remove.
    pub registration_id: HostRegistrationId,
    /// Concurrent updates must not be silently removed.
    pub expected_revision: HostRevision,
}

/// Connection facts are projected from existing authorities, never guessed from registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "RegisteredHostStatus"))]
#[serde(rename_all = "snake_case")]
pub enum HostStatus {
    /// Existing destination authority currently has an established route.
    Online,
    /// Existing trusted destination is currently unreachable.
    Offline,
    /// Registration is retained but not enabled for directory selection.
    Disabled,
    /// Destination authority no longer exists or could not be observed.
    Error,
}
/// Display metadata and independently observed connectivity stay distinguishable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HostSummary {
    /// Durable identity/annotations, owned by this caller's source.
    pub registration: HostRegistration,
    /// This read's actual transport observation.
    pub status: HostStatus,
    /// Destination display name when the existing authority supplies one.
    pub destination_name: Option<HostLabel>,
    /// Explicit uncertainty/failure instead of fabricated online state.
    pub error: Option<crate::WireError>,
}
/// A directory includes disabled and failed records as well as the node's own identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HostDirectory {
    /// The selected node, not a registration pretending to be self.
    pub node: crate::NodeInfo,
    /// Caller-source entries, including unavailable targets.
    pub hosts: Vec<HostSummary>,
}
