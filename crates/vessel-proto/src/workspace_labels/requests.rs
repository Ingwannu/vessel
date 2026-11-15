//! Request shapes remain staged until Quality B registers their pending rows.

use serde::{Deserialize, Serialize};

use super::{CatalogCursor, Label, LabelColor, LabelCount, LabelName};

/// One-shot list or bounded observation; no second subscription protocol.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CatalogRead {
    /// Return immediately, including an empty retained change batch.
    #[default]
    Now,
    /// With a current cursor, wait for the next committed catalog change.
    Observe,
}

/// Load a full catalog or resume a generation-scoped contiguous sequence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListWorkspaceLabels {
    /// No cursor means authoritative snapshot, even in Observe mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<CatalogCursor>,
    /// Cancellation releases a bounded long poll; it leaves no server lease.
    #[serde(default)]
    pub read: CatalogRead,
}

/// Rename and recolor share one atomic writer operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UpdateWorkspaceLabel {
    /// Immutable normalized target identity.
    pub name: LabelName,
    /// Omission preserves current host spelling; case-only changes are legal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_name: Option<LabelName>,
    /// Omission preserves the current host color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<LabelColor>,
}

/// Reply follows the durable commit; no-op updates have affected count zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WorkspaceLabelUpdated {
    /// The authoritative definition after update.
    pub label: Label,
    /// Workspaces whose assignment spelling changed.
    pub affected: LabelCount,
}

/// Deletion inspection is separate from mutation and grants no lock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InspectWorkspaceLabelDelete {
    /// Catalog target.
    pub name: LabelName,
}

/// A user confirmed deletion; recompute affected assignments in the commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DeleteWorkspaceLabel {
    /// Catalog target; a missing target is an idempotent zero-count result.
    pub name: LabelName,
}

/// Inspection or committed deletion result; callers retain operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WorkspaceLabelDeleted {
    /// Advisory during inspection; authoritative when deletion commits.
    pub affected: LabelCount,
}

/// Stable semantic reasons, never parsed from a peer's message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceLabelFailure {
    /// A rename collides with another normalized catalog name.
    NameTaken,
    /// The edited catalog target no longer exists.
    LabelNotFound,
    /// The assignment target has no durable registry identity.
    WorkspaceNotFound,
    /// Archived or archiving targets refuse new assignments.
    WorkspaceInactive,
    /// A newer node introduced another reason; never guess an action.
    #[serde(other)]
    Unknown,
}
