//! Real registry admission and invalidation in the label writer's savepoint.
//! Canonical helpers run inside the label mutation savepoint.

use super::{WorkspaceLabelCommit, plans::AssignmentChange, sql};
use crate::{StoreError, registry::RegistryChange};
use rusqlite::Connection;
use vessel_proto::{
    WorkspaceId,
    workspace_labels::{
        AssignWorkspaceLabel, DeleteWorkspaceLabel, InspectWorkspaceLabelDelete,
        UpdateWorkspaceLabel, WorkspaceLabelAssigned, WorkspaceLabelDeleted, WorkspaceLabelFailure,
        WorkspaceLabelUpdated,
    },
};

struct CanonicalRegistry;
impl sql::Registry for CanonicalRegistry {
    fn require_active(
        &self,
        connection: &Connection,
        workspace: WorkspaceId,
    ) -> Result<(), StoreError> {
        crate::registry::write::require_active(connection, workspace).map_err(|error| match error {
            StoreError::RegistryMissing(_) => {
                StoreError::WorkspaceLabelRefused(WorkspaceLabelFailure::WorkspaceNotFound)
            }
            StoreError::RegistryInactive(_) => {
                StoreError::WorkspaceLabelRefused(WorkspaceLabelFailure::WorkspaceInactive)
            }
            error => error,
        })
    }
    fn changed(&self, connection: &Connection) -> Result<(), StoreError> {
        crate::registry::write::bump_revision(connection)
    }
}

pub(crate) struct Applied<T> {
    pub(crate) committed: WorkspaceLabelCommit<T>,
    pub(crate) registry: RegistryChange,
}

fn applied<T>(mutation: sql::Mutation<T>) -> Applied<T> {
    Applied {
        committed: WorkspaceLabelCommit {
            result: mutation.result,
            delta: mutation.catalog,
        },
        registry: match mutation.assignments {
            AssignmentChange::Unchanged => RegistryChange::Unchanged,
            AssignmentChange::Changed => RegistryChange::Changed,
        },
    }
}

pub(crate) fn assign(
    connection: &Connection,
    request: &AssignWorkspaceLabel,
) -> Result<Applied<WorkspaceLabelAssigned>, StoreError> {
    sql::assign(connection, &CanonicalRegistry, request).map(applied)
}
pub(crate) fn update(
    connection: &Connection,
    request: &UpdateWorkspaceLabel,
) -> Result<Applied<WorkspaceLabelUpdated>, StoreError> {
    sql::update(connection, &CanonicalRegistry, request).map(applied)
}
pub(crate) fn delete(
    connection: &Connection,
    request: &DeleteWorkspaceLabel,
) -> Result<Applied<WorkspaceLabelDeleted>, StoreError> {
    sql::delete(connection, &CanonicalRegistry, request).map(applied)
}
pub(crate) fn inspect(
    connection: &Connection,
    request: &InspectWorkspaceLabelDelete,
) -> Result<WorkspaceLabelDeleted, StoreError> {
    sql::inspect(connection, &request.name)
}
