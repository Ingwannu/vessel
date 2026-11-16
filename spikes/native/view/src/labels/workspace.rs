//! Workspace label intent is bound to the shared layout and connection contracts.
//!
//! [Decision Log]
//! - Purpose: a session/cwd or another alias must not become a workspace write.
//! - Constraint: layout owns workspace identity; the link owns admissions.
//! - Alternatives: local workspace keys or resolve a connection by node id.
//! - Choice: validate their agreement once and retain the immutable admission.
//! - Tradeoff: legacy session layouts cannot offer workspace assignment actions.

use crate::{connection::ConnectionContext, layout::LayoutKey};
use vessel_proto::WorkspaceId;

use super::Failure;

/// The exact workspace and authenticated admission captured by a row/header action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceTarget {
    connection: ConnectionContext,
    workspace: WorkspaceId,
}

impl WorkspaceTarget {
    /// Admit only a canonical workspace on this same authenticated node.
    ///
    /// # Errors
    /// Legacy session layouts refuse workspace operations; a node mismatch is
    /// a changed selection, never a reason to find a different connection.
    pub fn new(connection: ConnectionContext, key: &LayoutKey) -> Result<Self, Failure> {
        match key {
            LayoutKey::Workspace { node, workspace } => {
                if *node != connection.node() {
                    return Err(Failure::Changed);
                }
                Ok(Self {
                    connection,
                    workspace: *workspace,
                })
            }
            LayoutKey::Session { .. } => Err(Failure::Unsupported),
        }
    }

    /// The original admission; callers must not re-resolve by node identity.
    #[must_use]
    pub const fn connection(&self) -> &ConnectionContext {
        &self.connection
    }

    /// Registry-owned identity, never a session or a directory inference.
    #[must_use]
    pub const fn workspace(&self) -> WorkspaceId {
        self.workspace
    }

    /// Old callbacks/results cannot claim a new admission or workspace.
    #[must_use]
    pub fn matches(&self, connection: &ConnectionContext, key: &LayoutKey) -> bool {
        &self.connection == connection
            && matches!(key, LayoutKey::Workspace {node,workspace}
                if *node == connection.node() && *workspace == self.workspace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{ConnectionEpochs, MachineKey};
    use vessel_proto::NodeId;

    #[test]
    fn exact_workspace_target_refuses_other_node_workspace_or_admission() {
        let node = NodeId::random();
        let workspace = WorkspaceId::random();
        let key = LayoutKey::Workspace { node, workspace };
        let mut epochs = ConnectionEpochs::default();
        let connection = epochs
            .admit(MachineKey::from_book("fixture"), node)
            .unwrap();
        let target = WorkspaceTarget::new(connection.clone(), &key).unwrap();
        assert!(target.matches(&connection, &key));
        let reconnect = epochs
            .admit(MachineKey::from_book("fixture"), node)
            .unwrap();
        assert!(!target.matches(&reconnect, &key));
        let other_workspace = LayoutKey::Workspace {
            node,
            workspace: WorkspaceId::random(),
        };
        assert!(!target.matches(&connection, &other_workspace));
        let legacy = LayoutKey::Session {
            machine: crate::layout::HostAlias::parse("fixture").unwrap(),
            session: vessel_proto::SessionId::random(),
        };
        assert_eq!(
            WorkspaceTarget::new(connection.clone(), &legacy),
            Err(Failure::Unsupported)
        );
        let other_node = LayoutKey::Workspace {
            node: NodeId::random(),
            workspace,
        };
        assert_eq!(
            WorkspaceTarget::new(connection, &other_node),
            Err(Failure::Changed)
        );
    }
}
