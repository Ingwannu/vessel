//! File scope retains one original connection and physical root. These are
//! request facts, not capabilities: the shared actor admits every operation.

use crate::connection::ConnectionContext;
use vessel_proto::{NodeId, workspace::WorkspaceRoot};

/// Renderer/tab identity never substitutes for the source's actual node/root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileScope {
    connection: ConnectionContext,
    root: WorkspaceRoot,
}

impl FileScope {
    /// Carry a root already resolved for this exact connection. Constructing
    /// this value grants no permission and performs no alias/cwd fallback.
    #[must_use]
    pub const fn new(connection: ConnectionContext, root: WorkspaceRoot) -> Self {
        Self { connection, root }
    }

    /// Original admission, checked before polling and after every node call.
    #[must_use]
    pub const fn connection(&self) -> &ConnectionContext {
        &self.connection
    }

    /// Canonical resource identity, independent from the mutable book alias.
    #[must_use]
    pub const fn node(&self) -> NodeId {
        self.connection.node()
    }

    /// Node-authorized physical file root, distinct from logical `WorkspaceId`.
    #[must_use]
    pub const fn root(&self) -> WorkspaceRoot {
        self.root
    }
}
