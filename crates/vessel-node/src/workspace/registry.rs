//! Staged registry provisioning (ADR 0082); remove cfg(test) when pending RPCs wire.
//! Uses existing project root checks/worktree approval and the store writer actor.

mod provision;

use super::WorkspaceError;
use vessel_proto::{WorkspaceId, registry::WorkspaceDescriptor};
use vessel_store::StoreError;

/// Do not collapse uncertain backing effects into a retryable connection failure.
#[derive(Debug, thiserror::Error)]
pub(crate) enum RegistryError {
    #[error("workspace registry persistence failed")]
    Store(#[from] StoreError),
    #[error("workspace placement failed")]
    Placement(#[from] WorkspaceError),
    #[error("workspace identity is already reserved for a different creation intent")]
    IntentConflict,
    #[error("workspace registry task stopped")]
    Task(#[source] tokio::task::JoinError),
    #[error("workspace reservation has no managed backing identity")]
    MissingBacking,
}

/// Internal creation service result; the create-flow owner adds user operation receipts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Provisioned {
    Ready(WorkspaceDescriptor),
    NeedsApproval(vessel_proto::workspace::ProjectCommands),
    Refused(vessel_proto::workspace::WorktreeRefusal),
    InProgress(WorkspaceId),
    Uncertain(WorkspaceId),
}
