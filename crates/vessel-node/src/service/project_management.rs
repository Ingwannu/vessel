//! Exact project selection and settings routes; workspace creation remains separate.

use super::{Connection, NodeService};
use vessel_proto::{
    WireError, project_management::RegisterProjectDirectory, workspace::ProjectInfo,
};

impl NodeService {
    pub(crate) async fn register_project_directory(
        &self,
        _: &Connection,
        request: RegisterProjectDirectory,
    ) -> Result<ProjectInfo, WireError> {
        self.workspaces
            .register_project_directory(request.path, request.name)
            .await
            .map_err(|error| error.wire())
    }
}
