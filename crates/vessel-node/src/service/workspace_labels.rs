//! Real unary handlers; the generated Guarded scope/agent checks precede them.
use super::{Connection, NodeService};
use crate::{error::ServiceError, principal::Principal, waits::Waiter, workspace::MAX_WAIT_SECS};
use vessel_proto::{
    WireError,
    workspace_labels::{
        AssignWorkspaceLabel, CatalogList, CatalogRead, DeleteWorkspaceLabel,
        InspectWorkspaceLabelDelete, ListWorkspaceLabels, UpdateWorkspaceLabel,
        WorkspaceLabelAssigned, WorkspaceLabelDeleted, WorkspaceLabelUpdated,
    },
};
use vessel_store::StoreError;

fn wire(error: StoreError) -> WireError {
    ServiceError::from(error).to_wire()
}
impl NodeService {
    pub(crate) async fn list_workspace_labels(
        &self,
        connection: &Connection,
        mut request: ListWorkspaceLabels,
    ) -> Result<CatalogList, WireError> {
        let mut wait = match request.read {
            CatalogRead::Now => None,
            CatalogRead::Observe => Some(MAX_WAIT_SECS),
        };
        if matches!(connection.principal(), Principal::Delegated { .. }) {
            wait = None;
        }
        let here = self.links.node();
        let shares = Waiter::of(connection.principal(), here).shares(here);
        let _slot = self.wait_slot(connection, shares, &mut wait);
        if wait.is_none() {
            request.read = CatalogRead::Now;
        }
        self.workspace_labels.list(request).await.map_err(wire)
    }
    pub(crate) async fn assign_workspace_label(
        &self,
        _: &Connection,
        request: AssignWorkspaceLabel,
    ) -> Result<WorkspaceLabelAssigned, WireError> {
        self.workspace_labels.assign(request).await.map_err(wire)
    }
    pub(crate) async fn update_workspace_label(
        &self,
        _: &Connection,
        request: UpdateWorkspaceLabel,
    ) -> Result<WorkspaceLabelUpdated, WireError> {
        self.workspace_labels.update(request).await.map_err(wire)
    }
    pub(crate) async fn inspect_workspace_label_delete(
        &self,
        _: &Connection,
        request: InspectWorkspaceLabelDelete,
    ) -> Result<WorkspaceLabelDeleted, WireError> {
        self.workspace_labels.inspect(request).await.map_err(wire)
    }
    pub(crate) async fn delete_workspace_label(
        &self,
        _: &Connection,
        request: DeleteWorkspaceLabel,
    ) -> Result<WorkspaceLabelDeleted, WireError> {
        self.workspace_labels.delete(request).await.map_err(wire)
    }
}
