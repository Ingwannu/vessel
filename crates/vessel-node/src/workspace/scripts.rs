//! What the workspace gives a project script's run (protocol 11, ADR
//! 0031): the checkout it runs in, the project's file there as it is now,
//! whether that set is approved, and the command through the platform's
//! shell. Running it is `crate::terminals`'.

use std::ffi::OsString;
use std::path::PathBuf;

use vessel_proc::CommandSpec;
use vessel_proto::scripts::ScriptEntry;
use vessel_proto::workspace::{BranchName, CommandsDigest, ProjectCommands};
use vessel_proto::{ProjectId, WorktreeId};
use vessel_workspace::paths::{PathError, Root};
use vessel_workspace::project_file::{ProjectFile, ProjectFileError};

use crate::workspace::{WorkspaceError, Workspaces};

/// The checkout a script runs in: a worktree, or a project's folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScriptRoot {
    /// Whose approval it runs under.
    pub(crate) project: ProjectId,
    /// The folder `vessel.toml` is read in.
    pub(crate) root: PathBuf,
    /// The project's own folder.
    pub(crate) project_root: PathBuf,
    /// The worktree, when it is one.
    pub(crate) worktree: Option<WorktreeId>,
    /// The branch checked out there, when the node knows it.
    pub(crate) branch: Option<BranchName>,
}

/// Why a script's folder is not usable.
#[derive(Debug)]
pub(crate) enum DirError {
    /// Not a folder inside the root.
    Missing,
    /// The node could not look.
    Workspace(WorkspaceError),
}

impl Workspaces {
    /// The checkout of `project`, or of its `worktree`.
    ///
    /// # Errors
    /// `not_found` for a project or a worktree the node does not have.
    pub(crate) fn script_root(
        &self,
        project: ProjectId,
        worktree: Option<WorktreeId>,
    ) -> Result<ScriptRoot, WorkspaceError> {
        let project_root = self.project_root(project)?;
        let state = self.state();
        let (root, branch) = match worktree {
            Some(id) => {
                let entry = state
                    .worktrees
                    .get(&id)
                    .ok_or(WorkspaceError::UnknownWorktree(id))?;
                (entry.info.path.clone(), entry.info.branch.clone())
            }
            None => (project_root.clone(), None),
        };
        Ok(ScriptRoot {
            project,
            root,
            project_root,
            worktree,
            branch,
        })
    }

    /// The root's `vessel.toml` as it is now, on the wire's terms with its
    /// approval, and parsed.
    pub(crate) async fn scripts(
        &self,
        root: &ScriptRoot,
    ) -> (
        ProjectCommands,
        Result<Option<ProjectFile>, ProjectFileError>,
    ) {
        let file = self.file_in(&root.root).await;
        let shown = match &file {
            Ok(Some(file)) => self.describe(root.project, Ok(Some(file.clone()))),
            Ok(None) => ProjectCommands::None,
            Err(error) => ProjectCommands::Invalid {
                message: error.to_string(),
            },
        };
        (shown, file)
    }

    /// Whether a person approved `digest` for `project` (asked again
    /// before a service's every restart).
    pub(crate) fn is_approved(&self, project: ProjectId, digest: &CommandsDigest) -> bool {
        self.approved(project, digest)
    }

    /// `entry`'s command in `root`, through the platform's shell, with
    /// setup's variables and the script's name; and a file to remove
    /// once it has ended (Windows).
    ///
    /// # Errors
    /// [`DirError::Missing`] when its `dir` is not a folder inside the
    /// root.
    pub(crate) async fn script_command(
        &self,
        root: &ScriptRoot,
        entry: &ScriptEntry,
    ) -> Result<(CommandSpec, Option<PathBuf>), DirError> {
        let folder = {
            let base = root.root.clone();
            let dir = entry.dir.clone();
            tokio::task::spawn_blocking(move || {
                let opened = Root::open(&base)?;
                let folder = match &dir {
                    Some(dir) if !dir.is_root() => opened.existing(dir)?,
                    _ => opened.path().to_path_buf(),
                };
                if folder.is_dir() {
                    Ok(folder)
                } else {
                    Err(PathError::Root)
                }
            })
            .await
            .map_err(|_stopped| {
                DirError::Workspace(WorkspaceError::Io(
                    "resolving the folder stopped".to_owned(),
                ))
            })?
            .map_err(|_missing| DirError::Missing)?
        };
        let mut env: Vec<(String, OsString)> = vec![
            (
                "VESSEL_PROJECT_ROOT".to_owned(),
                root.project_root.clone().into_os_string(),
            ),
            ("VESSEL_SCRIPT".to_owned(), entry.name.as_str().into()),
        ];
        if root.worktree.is_some() {
            env.push((
                "VESSEL_WORKTREE".to_owned(),
                root.root.clone().into_os_string(),
            ));
        }
        if let Some(branch) = &root.branch {
            env.push(("VESSEL_BRANCH".to_owned(), branch.as_str().into()));
        }
        vessel_workspace::commands::shell_command(&entry.command, &folder, &env, &self.dirs.scratch)
            .await
            .map_err(|error| DirError::Workspace(WorkspaceError::Io(error.to_string())))
    }
}
