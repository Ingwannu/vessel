//! Workspace recovery values are staged before registry-backed call wiring.
//! No request is callable until its pending method row and handler are connected.
use crate::{ProtocolVersion, WireError, WorkspaceId};
use serde::{Deserialize, Serialize};

/// One merge-assigned feature threshold covers workspace lifecycle calls.
pub const WORKSPACE_LIFECYCLE: Option<ProtocolVersion> = None;

/// Inspect an exact archived workspace; callers never infer recovery from a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InspectWorkspaceRecovery {
    /// Canonical registry identity, retained across backing recreation.
    pub workspace: WorkspaceId,
}

/// What an authoritative inspection permits the person to request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRecoveryAction {
    /// The original directory exists; make its workspace active again.
    Unarchive,
    /// Recreate the recorded worktree and validate its original subdirectory.
    Restore,
    /// A newer node knows this action; an older client must not execute it.
    #[serde(other)]
    Unknown,
}

/// Stable unavailable conditions remain separate from inspection I/O failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRecoveryUnavailable {
    /// The canonical registry contains no such identity.
    WorkspaceNotFound,
    /// An active workspace does not need restoration.
    WorkspaceNotArchived,
    /// The owning project no longer exists.
    ProjectNotFound,
    /// A local workspace's original directory is missing.
    WorkspaceDirectoryMissing,
    /// The source project directory needed for recreation is missing.
    ProjectDirectoryMissing,
    /// Recorded placement cannot safely be reconstructed by this node.
    UnsupportedBacking,
    /// Earlier actor launches still need authoritative reconciliation.
    UncertainStarts,
    /// A newer node reports another unavailable condition.
    #[serde(other)]
    Unknown,
}

/// A missing/unavailable result is data, rather than an invented successful plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum WorkspaceRecoveryState {
    /// Offer only the exact action the node inspected.
    Recoverable {
        /// A future action must produce an upgrade state on older clients.
        action: WorkspaceRecoveryAction,
    },
    /// Explain why retrying this action cannot recreate the missing source.
    Unavailable {
        /// Typed reason suitable for localized presentation.
        reason: WorkspaceRecoveryUnavailable,
    },
    /// A future state is not permission to restore.
    #[serde(other)]
    Unknown,
}

/// An inspection remains tied to the exact requested workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[must_use]
pub struct WorkspaceRecoveryInspection {
    /// The inspected durable identity.
    pub workspace: WorkspaceId,
    /// Recoverable, unavailable, or newer-node state.
    pub state: WorkspaceRecoveryState,
}

/// A recovery failure identifies the safety boundary that did not settle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRecoveryStage {
    /// Reinspection failed before an exclusive claim.
    Inspect,
    /// Another operation or registry revision won the claim.
    Claim,
    /// Directory/backing validation or recreation did not settle.
    Backing,
    /// The final fresh registry commit did not settle.
    Activate,
    /// A future stage remains a failure, never an implicit success.
    #[serde(other)]
    Unknown,
}

/// Restore never starts archived agents or reruns workspace setup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum WorkspaceRestoreOutcome {
    /// The backing was validated and the exclusive claim committed Active.
    Restored {
        /// Which known recovery operation actually settled.
        action: WorkspaceRecoveryAction,
    },
    /// Reinspection found that the requested recovery cannot be performed.
    Unavailable {
        /// No path-derived fallback action is authorized.
        reason: WorkspaceRecoveryUnavailable,
    },
    /// A typed failure retains the node's mapped source once, at the RPC edge.
    Failed {
        /// Which safety boundary failed.
        stage: WorkspaceRecoveryStage,
        /// Existing wire error taxonomy and human explanation.
        error: WireError,
    },
    /// The deadline elapsed; the durable claim remains non-active for reconciliation.
    Uncertain,
    /// An older client must refresh/upgrade rather than treating a future result as success.
    #[serde(other)]
    Unknown,
}

/// A result for one exact workspace, never a replacement workspace identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[must_use]
pub struct WorkspaceRestoreReport {
    /// The original identity retained by the operation.
    pub workspace: WorkspaceId,
    /// Success, authoritative refusal, typed failure or uncertainty.
    pub outcome: WorkspaceRestoreOutcome,
}

#[cfg(test)]
mod tests;
