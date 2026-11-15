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
