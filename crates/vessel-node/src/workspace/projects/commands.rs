//! What a project's file (`vessel.toml`) would run, read at a commit or
//! in a folder as it is, and a person's approval of that set. Nothing is
//! run here: setup and teardown are the workspace's `setup`.

use std::path::Path;

use vessel_proto::ProjectId;
use vessel_proto::workspace::{CommandsDigest, ProjectCommands, WorktreeBase};
use vessel_workspace::project_file::{self, ProjectFile, ProjectFileError};

use crate::workspace::{WorkspaceError, Workspaces};

impl Workspaces {
    /// What the project's file names, at `base` or in its folder as it is,
    /// and whether that set is approved.
    ///
    /// # Errors
    /// `not_found`; `invalid_request` for a base that does not resolve.
    pub(crate) async fn commands(
        &self,
        project: ProjectId,
        base: Option<&WorktreeBase>,
    ) -> Result<ProjectCommands, WorkspaceError> {
        let root = self.project_root(project)?;
        let file = match base {
            Some(base) => self.file_at(&root, base).await?,
            None => self.file_in(&root).await,
        };
        Ok(self.describe(project, file))
    }

    /// The project's file at a commit or ref, read as a blob.
    pub(crate) async fn file_at(
        &self,
        root: &Path,
        base: &WorktreeBase,
    ) -> Result<Result<Option<ProjectFile>, ProjectFileError>, WorkspaceError> {
        let git = self.git()?;
        let commit = git
            .resolve(root, base)
            .await?
            .ok_or_else(|| WorkspaceError::Invalid("the base does not name a commit".to_owned()))?;
        let bytes = git
            .read_blob(
                root,
                &commit,
                project_file::FILE_NAME,
                project_file::MAX_BYTES,
            )
            .await;
        Ok(match bytes {
            Ok(Some(bytes)) => ProjectFile::parse(&bytes).map(Some),
            Ok(None) => Ok(None),
            Err(vessel_workspace::GitError::OutputLimit) => Err(ProjectFileError::TooLarge),
            Err(error) => return Err(error.into()),
        })
    }

    /// The project's file in a folder as it is.
    /// Only a regular file inside the folder is read, and only up to the
    /// bound (`ProjectFile::read_in`): an agent can write in a project's
    /// folder and in its own worktree.
    pub(crate) async fn file_in(
        &self,
        folder: &Path,
    ) -> Result<Option<ProjectFile>, ProjectFileError> {
        let folder = folder.to_path_buf();
        tokio::task::spawn_blocking(move || ProjectFile::read_in(&folder))
            .await
            .unwrap_or_else(|stopped| {
                Err(ProjectFileError::Unreadable(std::io::Error::other(stopped)))
            })
    }

    /// The wire's form of a parsed file, with its approval.
    pub(crate) fn describe(
        &self,
        project: ProjectId,
        file: Result<Option<ProjectFile>, ProjectFileError>,
    ) -> ProjectCommands {
        match file {
            // Said on the wire as text, here only.
            Err(error) => ProjectCommands::Invalid {
                message: error.to_string(),
            },
            Ok(None) => ProjectCommands::None,
            Ok(Some(file)) if file.is_empty() => ProjectCommands::None,
            Ok(Some(file)) => {
                let digest = file.digest();
                ProjectCommands::Listed {
                    approved: self.approved(project, &digest),
                    setup: file.setup,
                    teardown: file.teardown,
                    digest,
                }
            }
        }
    }

    /// Whether a person approved `digest` for `project`.
    pub(super) fn approved(&self, project: ProjectId, digest: &CommandsDigest) -> bool {
        self.state().approvals.contains(&(project, digest.clone()))
    }

    /// Approve or withdraw a set; the folder's commands after.
    ///
    /// # Errors
    /// `not_found`.
    pub(crate) async fn approve(
        &self,
        project: ProjectId,
        digest: CommandsDigest,
        approved: bool,
    ) -> Result<ProjectCommands, WorkspaceError> {
        let root = self.project_root(project)?;
        self.store
            .set_approval(project, digest.clone(), approved)
            .await?;
        {
            let mut state = self.state();
            if approved {
                let _new = state.approvals.insert((project, digest));
            } else {
                let _was = state.approvals.remove(&(project, digest));
            }
        }
        let file = self.file_in(&root).await;
        Ok(self.describe(project, file))
    }
}
