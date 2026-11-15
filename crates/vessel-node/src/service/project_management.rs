//! Exact project selection and settings routes; workspace creation remains separate.

use super::{Connection, NodeService};
use vessel_proto::registry::{ConfigDocument, ReadProjectConfig, WriteProjectConfig};
use vessel_proto::{
    WireError,
    project_management::{
        CreateProjectDirectory, ProjectDirectoryOutcome, RegisterProjectDirectory,
    },
    workspace::ProjectInfo,
};

impl NodeService {
    pub(crate) async fn read_project_config(
        &self,
        _: &Connection,
        request: ReadProjectConfig,
    ) -> Result<ConfigDocument, WireError> {
        self.workspaces.read_project_config(request.project).await
    }

    pub(crate) async fn write_project_config(
        &self,
        _: &Connection,
        request: WriteProjectConfig,
    ) -> Result<ConfigDocument, WireError> {
        self.workspaces.write_project_config(request).await
    }

    pub(crate) async fn create_project_directory(
        &self,
        _: &Connection,
        request: CreateProjectDirectory,
    ) -> Result<ProjectDirectoryOutcome, WireError> {
        self.workspaces.create_project_directory(request).await
    }

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
