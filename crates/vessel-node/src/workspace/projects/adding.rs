//! Adding a project: a folder that exists (the repository's top level
//! when it is inside one), a new repository, or a clone, which runs as a
//! task of the node's and shows its progress on the project. Every root
//! is checked against the refused ones (`policy`) before it is recorded.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::watch;
use vessel_proto::ProjectId;
use vessel_proto::workspace::{
    CloneFailure, CloneStage, CloneUrl, FolderName, ProjectInfo, ProjectName, ProjectState,
};

use super::short;
use crate::workspace::{MAX_PROJECTS, ProjectEntry, WorkspaceError, Workspaces, policy};

impl Workspaces {
    /// Add the folder `path` (absolute, existing) as a project: the
    /// repository's top level when it is inside one. A root already known
    /// answers that project.
    ///
    /// # Errors
    /// `invalid_request` for a path that is not an absolute folder or a
    /// refused root; `conflict` past the bound.
    pub(crate) async fn add_project(
        self: &Arc<Self>,
        path: PathBuf,
        name: Option<ProjectName>,
    ) -> Result<ProjectInfo, WorkspaceError> {
        let folder = existing_folder(path).await?;
        self.refuse_git_dir(&folder).await?;
        let (root, git) = match &self.git {
            Some(git) => match git.toplevel(&folder).await? {
                Some(top) => (canonical(top).await, true),
                None => (folder, false),
            },
            None => (folder, false),
        };
        policy::check_root(&self.places, &root)?;
        if let Some(known) = self.project_at(&root) {
            return Ok(known);
        }
        self.insert_project(root, git, name, ProjectState::Ready)
            .await
    }

    /// Start a new repository in `path` (absent, or an empty folder).
    ///
    /// # Errors
    /// `unsupported` without git; `conflict` for a folder that is not
    /// empty; `invalid_request` for a relative path or a refused root.
    pub(crate) async fn init_project(
        self: &Arc<Self>,
        path: PathBuf,
        name: Option<ProjectName>,
    ) -> Result<ProjectInfo, WorkspaceError> {
        let git = self.git()?.clone();
        if !path.is_absolute() {
            return Err(WorkspaceError::Invalid(
                "the folder must be an absolute path".to_owned(),
            ));
        }
        // Checked before anything is made, so a refused root is not left
        // behind as an empty folder (inside the data folder, say); checked
        // again once made, as the folder that is there then.
        let planned = path.clone();
        let planned = tokio::task::spawn_blocking(move || would_be(&planned))
            .await
            .map_err(|_panicked| WorkspaceError::Io("resolving the folder stopped".to_owned()))??;
        policy::check_root(&self.places, &planned)?;
        let made = path.clone();
        tokio::task::spawn_blocking(move || {
            if made.exists() {
                let empty = std::fs::read_dir(&made)
                    .map_err(|error| {
                        WorkspaceError::Conflict(format!("`{}`: {error}", made.display()))
                    })?
                    .next()
                    .is_none();
                if !empty {
                    return Err(WorkspaceError::Conflict(format!(
                        "`{}` is not an empty folder",
                        made.display()
                    )));
                }
                Ok(())
            } else {
                std::fs::create_dir_all(&made)
                    .map_err(|error| WorkspaceError::Io(error.to_string()))
            }
        })
        .await
        .map_err(|_panicked| WorkspaceError::Io("making the folder stopped".to_owned()))??;
        let root = canonical(path).await;
        policy::check_root(&self.places, &root)?;
        self.refuse_git_dir(&root).await?;
        git.init(&root).await?;
        self.insert_project(root, true, name, ProjectState::Ready)
            .await
    }

    /// Clone `url` into `parent/folder`: answers at once with the project
    /// `cloning`; the clone runs as a task of the node's.
    ///
    /// # Errors
    /// `unsupported` without git; `invalid_request` for a parent that is not
    /// an absolute folder or no folder name; `conflict` when the target
    /// exists.
    pub(crate) async fn clone_project(
        self: &std::sync::Arc<Self>,
        url: CloneUrl,
        parent: PathBuf,
        folder: Option<FolderName>,
        name: Option<ProjectName>,
    ) -> Result<ProjectInfo, WorkspaceError> {
        let git = self.git()?.clone();
        let parent = existing_folder(parent).await?;
        let folder: FolderName = match folder {
            Some(folder) => folder,
            None => url
                .repository_name()
                .and_then(|name| name.parse().ok())
                .ok_or_else(|| {
                    WorkspaceError::Invalid("the URL gives no folder name; name one".to_owned())
                })?,
        };
        let target = parent.join(folder.as_str());
        policy::check_root(&self.places, &target)?;
        if tokio::fs::symlink_metadata(&target).await.is_ok() {
            return Err(WorkspaceError::Conflict(format!(
                "`{}` exists",
                target.display()
            )));
        }
        let name = name.or_else(|| folder.as_str().parse().ok());
        let cloning = ProjectState::Cloning {
            stage: CloneStage::Connecting,
            percent: None,
        };
        let info = self
            .insert_project(target.clone(), true, name, cloning)
            .await?;
        let cancel = self.stop.child_token();
        if let Some(entry) = self.state().projects.get_mut(&info.id) {
            entry.clone = Some(cancel.clone());
        }
        let temporary = parent.join(format!(".{folder}.vessel-clone-{}", short(info.id)));
        let workspaces = std::sync::Arc::clone(self);
        let project = info.id;
        drop(self.tasks.spawn(async move {
            let progress_to = std::sync::Arc::clone(&workspaces);
            let result = git
                .clone_into(&url, &target, &temporary, cancel, move |progress| {
                    let _now = progress_to.update(project, |info| {
                        info.state = ProjectState::Cloning {
                            stage: progress.stage,
                            percent: progress.percent,
                        };
                    });
                })
                .await;
            let state = match result {
                Ok(Ok(())) => {
                    let default = git.default_branch(&target).await.ok().flatten();
                    let _now = workspaces.update(project, |info| info.default_branch = default);
                    ProjectState::Ready
                }
                Ok(Err(failed)) => ProjectState::CloneFailed {
                    reason: failed.reason,
                    message: failed.message,
                },
                // Cancelled by `project.remove`, which forgot it already;
                // or the node is stopping, which `restore` tells as failed.
                Err(vessel_workspace::GitError::Cancelled) => return,
                Err(error) => ProjectState::CloneFailed {
                    reason: CloneFailure::Other,
                    message: error.to_string(),
                },
            };
            if let Some(info) = workspaces.update(project, |info| info.state = state)
                && let Err(error) = workspaces.save_project(info).await
            {
                tracing::warn!(%error, "could not record a clone's end");
            }
        }));
        Ok(info)
    }

    /// `invalid_request` for a folder that is git's own (a `.git` folder,
    /// inside one, or a bare repository): ADR 0030, wave 1's review
    /// finding 17. Asked of git when the node has one, and by its files
    /// either way.
    async fn refuse_git_dir(&self, folder: &Path) -> Result<(), WorkspaceError> {
        let refused = || {
            Err(WorkspaceError::Invalid(
                "this folder cannot be a project: it is git's own folder (a `.git` folder or a \
                 bare repository), not a working tree"
                    .to_owned(),
            ))
        };
        let probe = folder.to_path_buf();
        let looks = tokio::task::spawn_blocking(move || policy::looks_like_git_dir(&probe))
            .await
            .map_err(|_panicked| WorkspaceError::Io("looking at the folder stopped".to_owned()))?;
        if looks {
            return refused();
        }
        if let Some(git) = &self.git
            && git.is_git_dir(folder).await?
        {
            return refused();
        }
        Ok(())
    }

    async fn insert_project(
        self: &Arc<Self>,
        root: PathBuf,
        git: bool,
        name: Option<ProjectName>,
        state: ProjectState,
    ) -> Result<ProjectInfo, WorkspaceError> {
        let owner = Arc::clone(self);
        self.tasks
            .spawn(async move { owner.insert_project_now(root, git, name, state).await })
            .await
            .map_err(WorkspaceError::ProjectTask)?
    }

    async fn insert_project_now(
        &self,
        root: PathBuf,
        git: bool,
        name: Option<ProjectName>,
        state: ProjectState,
    ) -> Result<ProjectInfo, WorkspaceError> {
        // Serialize admission through durability/publication; cancelled callers cannot leave ghosts.
        let _serial = self.project_registrations.lock().await;
        if let Some(known) = self.project_at(&root) {
            return Ok(known);
        }
        if self.state().projects.len() >= MAX_PROJECTS {
            return Err(WorkspaceError::Conflict(format!(
                "this machine keeps at most {MAX_PROJECTS} projects"
            )));
        }
        let name = match name {
            Some(name) => name,
            None => root
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.parse().ok())
                .unwrap_or_else(default_name),
        };
        let default_branch = match (&self.git, git && matches!(state, ProjectState::Ready)) {
            (Some(git), true) => git.default_branch(&root).await.ok().flatten(),
            _ => None,
        };
        let info = ProjectInfo {
            id: ProjectId::random(),
            name,
            root,
            git,
            default_branch,
            state,
            created_at: Utc::now(),
            revision: 0,
        };
        self.save_project(info.clone()).await?;
        {
            let mut registry = self.state();
            let (revision, _) = watch::channel(0);
            drop(registry.projects.insert(
                info.id,
                ProjectEntry {
                    info: info.clone(),
                    revision,
                    clone: None,
                },
            ));
        }
        Ok(info)
    }
}

/// `path`, resolved, when it is an absolute folder.
async fn existing_folder(path: PathBuf) -> Result<PathBuf, WorkspaceError> {
    if !path.is_absolute() {
        return Err(WorkspaceError::Invalid(
            "the folder must be an absolute path".to_owned(),
        ));
    }
    tokio::task::spawn_blocking(move || {
        let resolved = std::fs::canonicalize(&path).map_err(|_missing| {
            WorkspaceError::Invalid(format!("`{}` does not exist", path.display()))
        })?;
        if !resolved.is_dir() {
            return Err(WorkspaceError::Invalid(format!(
                "`{}` is not a folder",
                path.display()
            )));
        }
        Ok(resolved)
    })
    .await
    .map_err(|_panicked| WorkspaceError::Io("resolving the folder stopped".to_owned()))?
}

/// `path` resolved, or as given when it cannot be.
/// The folder `path` (absolute) would be once made: its nearest existing
/// ancestor resolved, the rest joined as named. `.` and `..` are refused:
/// past an ancestor that exists only resolving could read them rightly,
/// and a new folder is named plainly. Blocking.
fn would_be(path: &Path) -> Result<PathBuf, WorkspaceError> {
    if path
        .components()
        .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
    {
        return Err(WorkspaceError::Invalid(format!(
            "`{}` names `.` or `..`; name the folder plainly",
            path.display()
        )));
    }
    let mut missing = Vec::new();
    let mut at = path;
    loop {
        if let Ok(found) = std::fs::canonicalize(at) {
            return Ok(missing
                .iter()
                .rev()
                .fold(found, |made, name| made.join(name)));
        }
        let (Some(parent), Some(name)) = (at.parent(), at.file_name()) else {
            return Err(WorkspaceError::Invalid(format!(
                "`{}` cannot be made: none of its folders exists",
                path.display()
            )));
        };
        missing.push(name);
        at = parent;
    }
}

async fn canonical(path: PathBuf) -> PathBuf {
    tokio::task::spawn_blocking(move || std::fs::canonicalize(&path).unwrap_or(path))
        .await
        .unwrap_or_default()
}

fn default_name() -> ProjectName {
    match "project".parse() {
        Ok(name) => name,
        Err(_never) => default_name_fallback(),
    }
}

#[expect(
    clippy::unreachable,
    reason = "a constant that parses; the type has no other constructor"
)]
fn default_name_fallback() -> ProjectName {
    unreachable!("`project` is a valid project name")
}
