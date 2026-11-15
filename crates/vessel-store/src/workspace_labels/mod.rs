//! Catalog and assignment SQL behind the existing registry and writer contracts.
//! Admission, revision and publication reuse the reviewed registry writer.
mod adapter;
mod commit;
mod plans;
mod sql;

pub(crate) use adapter::{Applied, assign, delete, inspect, update};
pub use commit::WorkspaceLabelCommit;
pub(crate) use commit::Write;

use crate::StoreError;
use rusqlite::Connection;
use vessel_proto::{
    registry::WorkspaceDescriptor,
    workspace_labels::{Label, WorkspaceLabelAssignments},
};

pub(crate) const MIGRATION: &str = sql::SQL;

pub(crate) fn catalog(connection: &Connection) -> Result<Vec<Label>, StoreError> {
    sql::catalog(connection)
}

/// Called inside `registry::load`'s held read transaction. Descriptor JSON is
/// deliberately untouched; ordered names join the same snapshot, colors do not.
pub(crate) fn registry_labels(
    connection: &Connection,
    workspaces: &[WorkspaceDescriptor],
) -> Result<Vec<WorkspaceLabelAssignments>, StoreError> {
    let mut assignments = Vec::new();
    for workspace in workspaces {
        let labels = sql::names(connection, workspace.id)?;
        if !labels.is_empty() {
            assignments.push(WorkspaceLabelAssignments {
                workspace: workspace.id,
                labels,
            });
        }
    }
    Ok(assignments)
}
