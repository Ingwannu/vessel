//! Host-local colored workspace labels (ADR 0086), not session metadata.
//! Calls remain unregistered until the shared registry and method table land.

mod requests;
mod sync;
mod values;

pub use requests::{
    AssignWorkspaceLabel, CatalogRead, DeleteWorkspaceLabel, InspectWorkspaceLabelDelete,
    ListWorkspaceLabels, UpdateWorkspaceLabel, WorkspaceLabelAssigned, WorkspaceLabelDeleted,
    WorkspaceLabelFailure, WorkspaceLabelUpdated,
};
pub use sync::{CatalogCursor, CatalogDelta, CatalogList, CatalogMode, CatalogUpdate};
pub use values::{
    Assignment, Label, LabelColor, LabelCount, LabelKey, LabelName, LabelNameError, trim_whitespace,
};

use crate::ProtocolVersion;

/// Assign only when registry ownership, durable mutation, observation, and
/// all client/forwarded descriptor gates have reviewed consumers.
pub const WORKSPACE_LABELS: Option<ProtocolVersion> = None;
