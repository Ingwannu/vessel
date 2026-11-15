//! Registered project authority around the existing project-file edit substrate.

use crate::workspace::{WorkspaceError, Workspaces};
use std::sync::Arc;
use vessel_proto::{
    ErrorCode, ProjectId, WireError,
    registry::{ConfigDocument, WriteProjectConfig},
};
use vessel_workspace::{
    files::FileError,
    project_config::{self, ConfigEditError},
};

impl Workspaces {
    pub(crate) async fn read_project_config(
        self: &Arc<Self>,
        project: ProjectId,
    ) -> Result<ConfigDocument, WireError> {
        let admission = self
            .admit_project(project)
            .await
            .map_err(|error| error.wire())?;
        let root = admission.root().path().to_owned();
        let document = project_config::read(root)
            .await
            .map_err(|error| edit_error(&error))?;
        if !self.has_project(project) {
            return Err(WorkspaceError::UnknownProject(project).wire());
        }
        Ok(document)
    }

    pub(crate) async fn write_project_config(
        self: &Arc<Self>,
        request: WriteProjectConfig,
    ) -> Result<ConfigDocument, WireError> {
        let owner = Arc::clone(self);
        self.tasks
            .spawn(async move {
                let admission = owner
                    .admit_project(request.project)
                    .await
                    .map_err(|error| error.wire())?;
                let root = admission.root().path().to_owned();
                project_config::write(root, request.expected, request.text)
                    .await
                    .map_err(|error| edit_error(&error))
            })
            .await
            .map_err(|error| WorkspaceError::project_task(error).wire())?
    }
}

fn edit_error(error: &ConfigEditError) -> WireError {
    let code = match error {
        ConfigEditError::Invalid(_)
        | ConfigEditError::NotText(_)
        | ConfigEditError::Text(_)
        | ConfigEditError::Read(_)
        | ConfigEditError::Write(
            FileError::Kind(..) | FileError::NotEmpty(_) | FileError::TooLarge,
        ) => ErrorCode::InvalidRequest,
        ConfigEditError::Root(_) | ConfigEditError::Write(FileError::Path(_)) => {
            ErrorCode::NotFound
        }
        ConfigEditError::Write(FileError::Changed(_) | FileError::Exists(_)) => ErrorCode::Conflict,
        ConfigEditError::Write(FileError::Io(_) | FileError::Task)
        | ConfigEditError::Task(_)
        | ConfigEditError::Name(_) => ErrorCode::Internal,
    };
    WireError::new(code, error.to_string())
}
