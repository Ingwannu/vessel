//! Atomic registry projection: clients select by node/workspace, never cwd or alias.

use super::{RegistryRevision, WorkspaceDescriptor};
use crate::{NodeId, SessionId, TerminalId, WorkspaceId};
use serde::{Deserialize, Serialize};

/// Ownership is explicit even when several workspaces share the same directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceResource {
    /// A registered agent/session; archived resources retain historical ownership.
    Session {
        /// Stable session identity, independent of provider-specific session ids.
        session: SessionId,
    },
    /// A supervised shell/script/service terminal, including an ended terminal.
    Terminal {
        /// Terminal identity does not imply ownership by its cwd.
        terminal: TerminalId,
    },
}

/// Membership travels with the same snapshot as placement to avoid split hydration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WorkspaceMembership {
    /// The independent lifecycle container.
    pub workspace: WorkspaceId,
    /// A resource belongs to at most one workspace on this node.
    pub resource: WorkspaceResource,
}

/// Complete person-visible registry state, replaced atomically by clients.
/// No field is added to existing session replies while the feature is pending.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RegistrySnapshot {
    /// Optional ordered assignments, omitted when empty or unavailable to this peer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub labels: Vec<crate::workspace_labels::WorkspaceLabelAssignments>,
    /// Cryptographic node identity survives a node-book alias rename.
    pub node: NodeId,
    /// Global monotonic version for ordering complete snapshots.
    pub revision: RegistryRevision,
    /// Includes empty workspaces; separate from the session list.
    pub workspaces: Vec<WorkspaceDescriptor>,
    /// Explicit session/terminal ownership in the same authoritative snapshot.
    pub memberships: Vec<WorkspaceMembership>,
}

impl RegistrySnapshot {
    /// Omit label assignments until their own contract is assigned and readable.
    /// This applies at every envelope hop as well as the direct handler.
    #[must_use]
    pub fn for_protocol(&self, protocol: crate::ProtocolVersion) -> Option<Self> {
        if self.labels.is_empty()
            || crate::workspace_labels::WORKSPACE_LABELS.is_some_and(|since| protocol >= since)
        {
            return None;
        }
        let mut snapshot = self.clone();
        snapshot.labels.clear();
        Some(snapshot)
    }

    /// Resolve a resource by stable identity; same cwd never merges workspaces.
    #[must_use]
    pub fn owner(&self, resource: WorkspaceResource) -> Option<WorkspaceId> {
        self.memberships
            .iter()
            .find(|member| member.resource == resource)
            .map(|member| member.workspace)
    }
}

/// Long-poll one complete registry revision, with a finite server wait.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetRegistry {
    /// None reads immediately; an unchanged revision may wait within caller shares.
    pub changed_from: Option<RegistryRevision>,
}
