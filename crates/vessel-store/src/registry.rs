//! Durable independent workspaces; SQL runs only on store writer/read threads.
//! No filesystem deletion, agent shutdown, or provider launch belongs here.

mod automation;
mod capacity;
mod lease_read;
mod leases;
mod placement;
mod project_reference;
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

pub use capacity::{
    MAX_DESCRIPTOR_BYTES, MAX_INTENT_BYTES, MAX_RECOVERY_BYTES, MAX_RESOURCES, MAX_RETAINED_BYTES,
    MAX_WORKSPACES, RESERVED_BYTES_PER_RESOURCE, RESERVED_BYTES_PER_WORKSPACE, WORKSPACE_KEY_BYTES,
};

pub(crate) use lease_read::{current as resource_run, pending as pending_starts};
pub use placement::{MAX_ARCHIVED_WORKSPACES, Placement};
pub(crate) use placement::{Register, archive_stranded, register, unregistered_placed};
pub(crate) use project_reference::read as project_reference;
pub(crate) use read::{
    load, memberships as workspace_memberships, reservation, resource_owner, workspace,
};
pub(crate) use write::apply;

/// Whether a transaction changed the durable snapshot version.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegistryChange {
    #[default]
    Unchanged,
    Changed,
}

/// One snapshot read transaction returns placement and ownership together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRegistry {
    /// Ordered label names hydrated in the same read transaction; never persisted here.
    pub labels: Vec<vessel_proto::workspace_labels::WorkspaceLabelAssignments>,
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
    /// Node-resolved source gate; never derived from a client cwd or a command approval.
    pub automation: vessel_proto::registry::WorkspaceAutomation,
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
    Restart {
        previous: WorkspaceStartLease,
        expected: RegistryRevision,
    },
    Release(vessel_proto::registry::ReleaseWorkspaceAutomation),
    Claim {
        workspace: WorkspaceId,
        resource: WorkspaceResource,
        expected: RegistryRevision,
    },
    Settle {
        lease: WorkspaceStartLease,
        settled: RegistryStartSettlement,
    },
    RecoverCreating,
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

fn resource_key(resource: WorkspaceResource) -> (&'static str, String) {
    match resource {
        WorkspaceResource::Session { session } => ("session", session.to_string()),
        WorkspaceResource::Terminal { terminal } => ("terminal", terminal.to_string()),
    }
}

/// A pre-spawn admission token, allocated by the registry writer rather than from cwd.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceStartLease {
    workspace: WorkspaceId,
    resource: WorkspaceResource,
    revision: RegistryRevision,
}
impl WorkspaceStartLease {
    pub(crate) const fn new(
        workspace: WorkspaceId,
        resource: WorkspaceResource,
        revision: RegistryRevision,
    ) -> Self {
        Self {
            workspace,
            resource,
            revision,
        }
    }
    /// Exact owner whose lifecycle admitted this start.
    #[must_use]
    pub const fn workspace(&self) -> WorkspaceId {
        self.workspace
    }
    /// Preallocated actor identity held by this lease.
    #[must_use]
    pub const fn resource(&self) -> WorkspaceResource {
        self.resource
    }
    /// Stale completions cannot settle a newer lease for the same resource.
    #[must_use]
    pub const fn revision(&self) -> RegistryRevision {
        self.revision
    }
}

/// Unsettled starts must finish or be reconciled before archiving can remove backing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryStartState {
    /// The node admitted a start but actor registration has not settled it.
    Pending,
    /// A previous node process stopped after admission; do not replay blindly.
    Uncertain,
}

/// Lifecycle drain receives exact lease tokens, not just an inferred resource count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRegistryStart {
    /// Used only after authoritative process/session reconciliation.
    pub lease: WorkspaceStartLease,
    /// Recovery distinguishes a live admitted operation from an interrupted one.
    pub state: RegistryStartState,
}

/// Completion is allowed during Archiving for starts admitted while Active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegistryStartSettlement {
    Ready,
    Failed,
}

/// Durable admission state, not proof that a registered process is still alive or has stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryResourceReadiness {
    /// Admitted but process registration has not settled.
    Pending,
    /// Last admitted run was registered; restart still requires an actual wait/join proof.
    Ready,
    /// The caller proved its last start did not spawn a resource.
    Failed,
    /// Restart interrupted admission; reconcile the exact actor before any replay.
    Uncertain,
}

/// Exact retained run token for supervisor recovery; reading it grants no process-liveness proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryResourceRun {
    /// Opaque owner/resource/admission identity accepted by lifecycle CAS.
    pub lease: WorkspaceStartLease,
    /// Only Ready permits a joined old run to seek a new restart admission.
    pub readiness: RegistryResourceReadiness,
}
