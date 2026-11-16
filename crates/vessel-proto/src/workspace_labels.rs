//! Host-local colored workspace labels (ADR 0086), not session metadata.
//! The five `workspace.label.*` calls are protocol 26.

#[cfg(test)]
mod protocol_tests;
mod registry;
mod requests;
mod sync;
mod values;

pub use registry::WorkspaceLabelAssignments;
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

/// Protocol 26 (2026-11-16): `workspace.label.list` and
/// `.inspect_delete` (Read), `.assign`, `.update` and `.delete` (Operate),
/// all unary and fleet-forwardable, and the registry snapshot's `labels`
/// projection. Their effects are complete: the node-local catalog and ordered
/// assignments in one store savepoint, the commit journal and its bounded
/// observation. Reviewed with None (ADR 0086); the integrator assigned the
/// number in merge order at the node line.
pub const WORKSPACE_LABELS: Option<ProtocolVersion> = Some(ProtocolVersion::new(26));
