//! Typed protocol 26 requests; shared admission refuses them below 26.

use serde::{Deserialize, Deserializer, Serialize};

use super::{Assignment, CatalogCursor, Label, LabelColor, LabelCount, LabelName};
use crate::WorkspaceId;

/// Assignment targets the reviewed registry identity, never a session or cwd.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AssignWorkspaceLabel {
    /// Stable independent workspace identity.
    pub workspace: WorkspaceId,
    /// Existing host spelling/color wins for a known normalized name.
    pub label: Label,
    /// Creating a definition is allowed only for Assigned.
    pub assignment: Assignment,
}

/// A committed assignment reply carries names; color remains in the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WorkspaceLabelAssigned {
    /// Authoritative host definition, or requested definition on unknown removal.
    pub label: Label,
    /// Ordered committed assignments after this operation.
    pub labels: Vec<LabelName>,
}

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
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "provided"
    )]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub new_name: Option<LabelName>,
    /// Omission preserves the current host color.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "provided"
    )]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<LabelColor>,
}

// Omitted patch fields preserve state; an explicitly null name/color is invalid,
// rather than a silently accepted edit that the reference would reject.
fn provided<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    decoder: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(decoder).map(Some)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceLabelFailure {
    /// A rename collides with another normalized catalog name.
    #[error("a label with that name already exists")]
    NameTaken,
    /// The edited catalog target no longer exists.
    #[error("workspace label was not found")]
    LabelNotFound,
    /// The assignment target has no durable registry identity.
    #[error("workspace was not found")]
    WorkspaceNotFound,
    /// Archived or archiving targets refuse new assignments.
    #[error("workspace is archived or being archived")]
    WorkspaceInactive,
    /// A newer node introduced another reason; never guess an action.
    #[serde(other)]
    #[error("workspace label operation was refused")]
    Unknown,
}

impl WorkspaceLabelFailure {
    /// The same semantic reason maps once to the transport's broad category.
    #[must_use]
    pub const fn code(self) -> crate::ErrorCode {
        match self {
            Self::NameTaken | Self::WorkspaceInactive => crate::ErrorCode::Conflict,
            Self::LabelNotFound | Self::WorkspaceNotFound => crate::ErrorCode::NotFound,
            Self::Unknown => crate::ErrorCode::Internal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_patch_fields_preserve_state_but_null_empty_and_unknown_values_refuse() {
        let request: UpdateWorkspaceLabel = serde_json::from_str(r#"{"name":"Ready"}"#).unwrap();
        assert_eq!(request.new_name, None);
        assert_eq!(request.color, None);
        for input in [
            r#"{"name":"Ready","new_name":null}"#,
            r#"{"name":"Ready","new_name":"  "}"#,
            r#"{"name":"Ready","color":null}"#,
            r#"{"name":"Ready","color":"future-color"}"#,
        ] {
            assert!(serde_json::from_str::<UpdateWorkspaceLabel>(input).is_err());
        }
    }

    #[test]
    fn structured_refusals_round_trip_and_future_reasons_stay_unknown() {
        use crate::{ErrorDetail, WireError};
        for reason in [
            WorkspaceLabelFailure::NameTaken,
            WorkspaceLabelFailure::LabelNotFound,
            WorkspaceLabelFailure::WorkspaceNotFound,
            WorkspaceLabelFailure::WorkspaceInactive,
        ] {
            let mut error = WireError::new(reason.code(), reason.to_string());
            error.detail = Some(ErrorDetail::WorkspaceLabel { reason });
            assert_eq!(
                serde_json::from_str::<WireError>(&serde_json::to_string(&error).unwrap()).unwrap(),
                error
            );
        }
        let future: WireError = serde_json::from_str(r#"{"code":"conflict","message":"refused","detail":{"kind":"workspace_label","reason":"future_reason"}}"#).unwrap();
        assert_eq!(
            future.detail,
            Some(ErrorDetail::WorkspaceLabel {
                reason: WorkspaceLabelFailure::Unknown
            })
        );
    }
}
