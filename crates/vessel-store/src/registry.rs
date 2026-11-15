//! Durable independent workspaces; SQL runs only on store writer/read threads.
//! No filesystem deletion, agent shutdown, or provider launch belongs here.

mod read;
#[cfg(test)]
mod tests;
pub(crate) mod write;

use crate::StoreError;
use serde::{Deserialize, Serialize};
use vessel_proto::registry::{
    RegistryRevision, WorkspaceDescriptor, WorkspaceIsolation, WorkspaceMembership,
    WorkspaceResource, WorkspaceVisibility,
};
use vessel_proto::workspace::WorktreeInfo;
use vessel_proto::{ProjectId, SessionTitle};
use vessel_proto::{WorkspaceId, WorktreeId};

pub(crate) use read::{load, reservation, workspace};
pub(crate) use write::apply;

/// One snapshot read transaction returns placement and ownership together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRegistry {
    /// Global ordering token changes in the transaction that mutates any registry row.
    pub revision: RegistryRevision,
    /// Empty workspaces are retained independently of session membership.
    pub workspaces: Vec<WorkspaceDescriptor>,
    /// Explicit ownership never follows from a shared cwd.
    pub memberships: Vec<WorkspaceMembership>,
}

/// Recovery source retained even after managed backing is safely removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryRecord {
    /// The public placement/lifecycle snapshot.
    pub descriptor: WorkspaceDescriptor,
    /// Original worktree metadata; restoration never guesses from a path pattern.
    pub recovery: Option<WorktreeInfo>,
}

/// Exact provision intent is retained before any filesystem side effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceProvision {
    /// Existing registered project.
    pub project: ProjectId,
    /// Local or the existing typed worktree creation mode.
    pub isolation: WorkspaceIsolation,
    /// Explicit user name, if supplied.
    pub title: Option<SessionTitle>,
    /// Resolved caller inheritance; the store never guesses a caller's workspace.
    pub visibility: WorkspaceVisibility,
}

/// Lifecycle for backing reservations, separate from user operation receipts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReservationStage {
    /// Intent/ids are durable; no filesystem mutation has begun.
    Reserved,
    /// Side effects may have occurred; replay must inspect instead of re-create.
    Creating,
    /// Descriptor has been persisted in the same transaction as this stage.
    Ready,
    /// Interrupted work cannot be retried until reconciled against the actual backing.
    Uncertain,
}
impl ReservationStage {
    const fn sql(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::Creating => "creating",
            Self::Ready => "ready",
            Self::Uncertain => "uncertain",
        }
    }
}

/// Workspace-specific id allocation is durable underneath the create-flow receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryReservation {
    /// Caller/create-flow reserved identity.
    pub workspace: WorkspaceId,
    /// Exact immutable provision input; reusing an id with different input is refused.
    pub intent: WorkspaceProvision,
    /// Fixed backing id chosen before a worktree directory can be created.
    pub worktree: Option<WorktreeId>,
    /// Side-effect admission state is never inferred from existence of a directory.
    pub stage: ReservationStage,
}

/// One writer transaction applies a mutation and bumps the snapshot version.
#[derive(Debug)]
pub(crate) enum RegistryWrite {
    Completed(Box<RegistryRecord>),
    Workspace {
        record: Box<RegistryRecord>,
        expected: Option<RegistryRevision>,
    },
    Membership {
        workspace: WorkspaceId,
        resource: WorkspaceResource,
        expected: RegistryRevision,
    },
    Reserve(RegistryReservation),
    Stage {
        workspace: WorkspaceId,
        expected: ReservationStage,
        stage: ReservationStage,
    },
}

fn decode<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, StoreError> {
    serde_json::from_str(text).map_err(StoreError::RegistryDecode)
}
