//! Writer answers carry publication data only after the outer batch commits.
use crate::writer::Done;
use vessel_proto::workspace_labels::{
    AssignWorkspaceLabel, CatalogDelta, DeleteWorkspaceLabel, UpdateWorkspaceLabel,
    WorkspaceLabelAssigned, WorkspaceLabelDeleted, WorkspaceLabelUpdated,
};

/// Committed label mutation; the node's serial label actor owns publication.
#[derive(Debug)]
pub struct WorkspaceLabelCommit<T> {
    /// Durable authoritative result; callers must not publish before receiving it.
    pub result: T,
    /// Catalog-only journal update, absent for idempotent and assignment-only changes.
    pub delta: Option<CatalogDelta>,
}

#[derive(Debug)]
pub(crate) enum Write {
    Assign {
        request: AssignWorkspaceLabel,
        done: Done<WorkspaceLabelCommit<WorkspaceLabelAssigned>>,
    },
    Update {
        request: UpdateWorkspaceLabel,
        done: Done<WorkspaceLabelCommit<WorkspaceLabelUpdated>>,
    },
    Delete {
        request: DeleteWorkspaceLabel,
        done: Done<WorkspaceLabelCommit<WorkspaceLabelDeleted>>,
    },
}
