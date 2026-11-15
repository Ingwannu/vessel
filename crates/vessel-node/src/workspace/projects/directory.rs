//! One-component mkdir with nonrecursive compensation, separate from legacy Git init.
//!
//! The tracked task spans creation, durable registration and cleanup so dropping an RPC
//! waiter cannot abandon a directory between those steps. A node crash can still leave
//! an unregistered directory; no retry deletes or adopts an existing target implicitly.

use std::{io, path::PathBuf, sync::Arc};
use vessel_proto::{
    ErrorCode, WireError,
    project_management::{
        CreateProjectDirectory, ProjectDirectoryCleanup, ProjectDirectoryOutcome,
    },
};

use super::adding::existing_folder;
use crate::workspace::{WorkspaceError, Workspaces, policy};

impl Workspaces {
    /// Create one empty directory under an existing absolute parent, then register it.
    /// No repository, workspace, setup or approval is created as a side effect.
    pub(crate) async fn create_project_directory(
        self: &Arc<Self>,
        request: CreateProjectDirectory,
    ) -> Result<ProjectDirectoryOutcome, WireError> {
        let owner = Arc::clone(self);
        self.tasks
            .spawn(async move { owner.create_project_directory_now(request).await })
            .await
            .map_err(|error| WorkspaceError::project_task(error).wire())?
    }

    async fn create_project_directory_now(
        self: &Arc<Self>,
        request: CreateProjectDirectory,
    ) -> Result<ProjectDirectoryOutcome, WireError> {
        let parent = existing_folder(request.parent)
            .await
            .map_err(|error| error.wire())?;
        self.refuse_git_dir(&parent)
            .await
            .map_err(|error| error.wire())?;
        let target = parent.join(request.name.as_str());
        policy::check_root(&self.places, &target).map_err(|error| error.wire())?;
        tokio::fs::create_dir(&target)
            .await
            .map_err(|error| directory_error(&error))?;
        match self.register_project_directory(target.clone(), None).await {
            Ok(project) => Ok(ProjectDirectoryOutcome::Created { project }),
            Err(error) => Ok(ProjectDirectoryOutcome::RegistrationFailed {
                error: error.wire(),
                cleanup: rollback_directory(target).await,
            }),
        }
    }
}

pub(super) async fn rollback_directory(path: PathBuf) -> ProjectDirectoryCleanup {
    match tokio::fs::remove_dir(&path).await {
        Ok(()) => ProjectDirectoryCleanup::Removed,
        Err(error) => ProjectDirectoryCleanup::Retained {
            path,
            error: directory_error(&error),
        },
    }
}

fn directory_error(error: &io::Error) -> WireError {
    let kind = error.kind();
    let code = if matches!(
        kind,
        io::ErrorKind::AlreadyExists | io::ErrorKind::DirectoryNotEmpty
    ) {
        ErrorCode::Conflict
    } else if matches!(kind, io::ErrorKind::NotFound | io::ErrorKind::NotADirectory) {
        ErrorCode::NotFound
    } else if kind == io::ErrorKind::PermissionDenied {
        ErrorCode::Forbidden
    } else {
        ErrorCode::Internal
    };
    WireError::new(code, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn compensation_retains_a_populated_directory_and_never_deletes_its_contents() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("retained");
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("keep"), "concurrent contents").unwrap();
        let result = rollback_directory(path.clone()).await;
        assert!(
            matches!(result, ProjectDirectoryCleanup::Retained { path: kept, .. } if kept == path)
        );
        assert_eq!(
            std::fs::read_to_string(path.join("keep")).unwrap(),
            "concurrent contents"
        );
    }
}
