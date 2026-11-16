//! Legacy cleanup never bypasses retained independent workspace/backing ownership.
//! This is metadata serialization, not exit, generation or physical-root proof.
use super::{WorkspaceError, Workspaces};
use vessel_proto::ProjectId;

impl Workspaces {
    /// Must run under the same retained project metadata admission as cleanup.
    pub(super) async fn require_no_registry_reference(
        &self,
        project: ProjectId,
    ) -> Result<(), WorkspaceError> {
        if self
            .store
            .registry_project_reference(project)
            .await?
            .is_some()
        {
            return Err(WorkspaceError::RegistryProjectOwned(project));
        }
        Ok(())
    }
}
