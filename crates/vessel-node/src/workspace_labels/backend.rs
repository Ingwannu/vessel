//! Adapter over the canonical Store; never a second writer or inferred registry.
use super::service::{Backend, Committed, Labels};
use vessel_proto::workspace_labels::{
    AssignWorkspaceLabel, DeleteWorkspaceLabel, InspectWorkspaceLabelDelete, Label,
    UpdateWorkspaceLabel, WorkspaceLabelAssigned, WorkspaceLabelDeleted, WorkspaceLabelUpdated,
};
use vessel_store::{Store, StoreError};

struct StoreBackend(Store);
impl Backend for StoreBackend {
    async fn catalog(&self) -> Result<Vec<Label>, StoreError> {
        self.0.workspace_label_catalog().await
    }
    async fn assign(
        &self,
        request: AssignWorkspaceLabel,
    ) -> Result<Committed<WorkspaceLabelAssigned>, StoreError> {
        self.0.assign_workspace_label(request).await
    }
    async fn update(
        &self,
        request: UpdateWorkspaceLabel,
    ) -> Result<Committed<WorkspaceLabelUpdated>, StoreError> {
        self.0.update_workspace_label(request).await
    }
    async fn inspect(
        &self,
        request: InspectWorkspaceLabelDelete,
    ) -> Result<WorkspaceLabelDeleted, StoreError> {
        self.0.inspect_workspace_label(request).await
    }
    async fn delete(
        &self,
        request: DeleteWorkspaceLabel,
    ) -> Result<Committed<WorkspaceLabelDeleted>, StoreError> {
        self.0.delete_workspace_label(request).await
    }
}
impl Labels {
    pub(crate) async fn open(store: Store) -> Result<Self, StoreError> {
        Self::start(StoreBackend(store)).await
    }
}
