//! Workspace ownership is supplied by the registry, never inferred from a path.
use super::{HostAlias, LayoutKey};
use vessel_proto::workspace::WorkspaceRoot;
use vessel_proto::{NodeId, SessionId, WorkspaceId};

/// Explicit hydration/failure prevents an empty startup list from redefining identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LookupProblem {
    /// A complete ownership snapshot has not arrived.
    #[error("workspace ownership is not hydrated")]
    NotHydrated,
    /// The node cannot currently supply authoritative ownership.
    #[error("workspace ownership is unavailable")]
    Unavailable,
}
/// Root resolution distinguishes an unregistered external folder from loading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceLocation {
    /// Registry-owned project or worktree root.
    Root(WorkspaceRoot),
    /// Local directory not registered as a file API root.
    External,
}
/// Integration seam for the registry owner; no daemon calls or fake implementation
/// exist in this module's product build. The registry controller owns freshness.
pub trait WorkspaceLookup {
    /// `None` means authoritatively unowned, never not-yet-hydrated.
    ///
    /// # Errors
    /// Refuses a snapshot that is incomplete or unavailable.
    fn session_workspace(
        &self,
        node: NodeId,
        session: SessionId,
    ) -> Result<Option<WorkspaceId>, LookupProblem>;
    /// File/Git panels resolve their own workspace, not the globally selected agent.
    ///
    /// # Errors
    /// Refuses a root lookup without an authoritative registry entry.
    fn workspace_location(
        &self,
        node: NodeId,
        workspace: WorkspaceId,
    ) -> Result<WorkspaceLocation, LookupProblem>;
}
/// Resolve a registry key or an explicitly unowned older-session bridge.
///
/// # Errors
/// Propagates not-hydrated or unavailable ownership; neither becomes a guessed workspace.
pub fn session_layout_key(
    lookup: &impl WorkspaceLookup,
    node: NodeId,
    session: SessionId,
    alias: HostAlias,
) -> Result<LayoutKey, LookupProblem> {
    lookup
        .session_workspace(node, session)
        .map(|owner| match owner {
            Some(workspace) => LayoutKey::Workspace { node, workspace },
            None => LayoutKey::Session {
                machine: alias,
                session,
            },
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    struct RegistryFake {
        node: NodeId,
        session: SessionId,
        workspace: WorkspaceId,
        root: WorkspaceRoot,
    }
    impl WorkspaceLookup for RegistryFake {
        fn session_workspace(
            &self,
            node: NodeId,
            session: SessionId,
        ) -> Result<Option<WorkspaceId>, LookupProblem> {
            if node != self.node {
                return Err(LookupProblem::Unavailable);
            }
            Ok((session == self.session).then_some(self.workspace))
        }
        fn workspace_location(
            &self,
            node: NodeId,
            workspace: WorkspaceId,
        ) -> Result<WorkspaceLocation, LookupProblem> {
            if node == self.node && workspace == self.workspace {
                Ok(WorkspaceLocation::Root(self.root))
            } else {
                Err(LookupProblem::Unavailable)
            }
        }
    }
    #[test]
    fn ownership_stays_host_scoped_and_two_workspaces_can_share_the_same_root() {
        let registry = RegistryFake {
            node: NodeId::random(),
            session: SessionId::random(),
            workspace: WorkspaceId::random(),
            root: WorkspaceRoot::Project {
                project: vessel_proto::ProjectId::random(),
            },
        };
        let alias = HostAlias::parse("local").unwrap();
        let key =
            session_layout_key(&registry, registry.node, registry.session, alias.clone()).unwrap();
        assert_eq!(
            key,
            LayoutKey::Workspace {
                node: registry.node,
                workspace: registry.workspace
            }
        );
        assert!(
            session_layout_key(&registry, NodeId::random(), registry.session, alias.clone())
                .is_err()
        );
        let other = LayoutKey::Workspace {
            node: registry.node,
            workspace: WorkspaceId::random(),
        };
        assert_ne!(key, other);
        assert!(matches!(
            session_layout_key(&registry, registry.node, SessionId::random(), alias).unwrap(),
            LayoutKey::Session { .. }
        ));
        assert_eq!(
            registry.workspace_location(registry.node, registry.workspace),
            Ok(WorkspaceLocation::Root(registry.root))
        );
    }
}
