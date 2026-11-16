//! Label SQL uses the existing read pool and the sole transactional writer.
use super::Store;
use crate::{
    StoreError, WorkspaceLabelCommit,
    workspace_labels::{self, Write},
    writer,
};
use tokio::sync::oneshot;
use vessel_proto::workspace_labels::{
    AssignWorkspaceLabel, DeleteWorkspaceLabel, InspectWorkspaceLabelDelete, Label,
    UpdateWorkspaceLabel, WorkspaceLabelAssigned, WorkspaceLabelDeleted, WorkspaceLabelUpdated,
};

impl Store {
    /// Read the complete node-local catalog using the existing blocking pool.
    /// # Errors
    /// Database or stored-value validation failure.
    pub async fn workspace_label_catalog(&self) -> Result<Vec<Label>, StoreError> {
        self.read(|reader| reader.with(workspace_labels::catalog))
            .await
    }
    /// Set assignment and optional new definition in one durable savepoint.
    /// # Errors
    /// Missing/inactive workspace, corrupt state or failed persistence.
    pub async fn assign_workspace_label(
        &self,
        request: AssignWorkspaceLabel,
    ) -> Result<WorkspaceLabelCommit<WorkspaceLabelAssigned>, StoreError> {
        let (done, committed) = oneshot::channel();
        self.send(writer::Write::WorkspaceLabels(Box::new(Write::Assign {
            request,
            done,
        })))
        .await?;
        committed.await.map_err(|_closed| StoreError::Closed)?
    }
    /// Rename/recolor atomically; the committed delta belongs to the label actor.
    /// # Errors
    /// Missing/colliding definition, corrupt state or persistence failure.
    pub async fn update_workspace_label(
        &self,
        request: UpdateWorkspaceLabel,
    ) -> Result<WorkspaceLabelCommit<WorkspaceLabelUpdated>, StoreError> {
        let (done, committed) = oneshot::channel();
        self.send(writer::Write::WorkspaceLabels(Box::new(Write::Update {
            request,
            done,
        })))
        .await?;
        committed.await.map_err(|_closed| StoreError::Closed)?
    }
    /// Advisory affected count; it grants no lock on a later deletion.
    /// # Errors
    /// Database or typed count validation failure.
    pub async fn inspect_workspace_label(
        &self,
        request: InspectWorkspaceLabelDelete,
    ) -> Result<WorkspaceLabelDeleted, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| workspace_labels::inspect(connection, &request))
        })
        .await
    }
    /// Remove the definition and every assignment in one durable savepoint.
    /// # Errors
    /// Corrupt state or failed persistence; a missing definition is idempotent.
    pub async fn delete_workspace_label(
        &self,
        request: DeleteWorkspaceLabel,
    ) -> Result<WorkspaceLabelCommit<WorkspaceLabelDeleted>, StoreError> {
        let (done, committed) = oneshot::channel();
        self.send(writer::Write::WorkspaceLabels(Box::new(Write::Delete {
            request,
            done,
        })))
        .await?;
        committed.await.map_err(|_closed| StoreError::Closed)?
    }
}
