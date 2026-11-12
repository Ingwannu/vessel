//! Projects: add a folder, clone, start a new one, forget one, and what a
//! project's file would run (protocol 10, ADR 0027).

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use tokio::sync::watch;
use vessel_proto::workspace::{
    CloneFailure, CloneStage, CloneUrl, CommandsDigest, FolderName, ProjectCommands, ProjectInfo,
    ProjectList, ProjectName, ProjectState, WorktreeBase,
};
use vessel_proto::{ProjectId, SessionId};
use vessel_workspace::project_file::{self, ProjectFile};

use super::{MAX_PROJECTS, MAX_WAIT_SECS, ProjectEntry, WorkspaceError, Workspaces, policy};

/// The live sessions and the folders they run in, as the node knows them.
pub(crate) type Live = [(SessionId, PathBuf)];

impl Workspaces {
    /// Read the stored record back: a clone or a setup the node's stop
    /// interrupted is told as failed.
    pub(super) async fn restore(&self, stored: vessel_store::StoredWorkspace) {
        let mut rewrite = Vec::new();
        {
            let mut state = self.state();
            for mut info in stored.projects {
                if matches!(info.state, ProjectState::Cloning { .. }) {
                    info.state = ProjectState::CloneFailed {
                        reason: CloneFailure::Other,
                        message: "the node stopped during the clone".to_owned(),
                    };
                    rewrite.push(info.clone());
                }
                let (revision, _) = watch::channel(info.revision);
                drop(state.projects.insert(
                    info.id,
                    ProjectEntry {
                        info,
                        revision,
                        clone: None,
                    },
                ));
            }
            state.approvals.extend(stored.approvals);
        }
        for info in rewrite {
            if let Err(error) = self.save_project(info).await {
                tracing::warn!(%error, "could not record an interrupted clone");
            }
        }
        self.restore_worktrees(stored.worktrees).await;
    }

    /// Every project (oldest first), and the git the node found.
    pub(crate) async fn list_projects(&self) -> ProjectList {
        let mut projects: Vec<ProjectInfo> = self
            .state()
            .projects
            .values()
            .map(|entry| entry.info.clone())
            .collect();
        projects.sort_by(|a, b| (a.created_at, a.id).cmp(&(b.created_at, b.id)));
        let more = projects.len() > MAX_PROJECTS;
        projects.truncate(MAX_PROJECTS);
        let projects = mark_missing(projects).await;
        ProjectList {
            git: self.support,
            projects,
            more,
        }
    }

    /// One project; with `changed_from` and `wait`, after its revision
    /// differs from it (at most [`MAX_WAIT_SECS`]).
    ///
    /// # Errors
    /// `not_found`.
    pub(crate) async fn get_project(
        &self,
        project: ProjectId,
        changed_from: Option<u64>,
        wait: bool,
    ) -> Result<ProjectInfo, WorkspaceError> {
        let mut revision = {
            let state = self.state();
            let entry = state
                .projects
                .get(&project)
                .ok_or(WorkspaceError::UnknownProject(project))?;
            entry.revision.subscribe()
        };
        if let (Some(from), true) = (changed_from, wait) {
            let limit = Duration::from_secs(MAX_WAIT_SECS.into());
            let _changed = tokio::time::timeout(limit, revision.wait_for(|now| *now != from)).await;
        }
        let info = self
            .state()
            .projects
            .get(&project)
            .map(|entry| entry.info.clone())
            .ok_or(WorkspaceError::UnknownProject(project))?;
        Ok(mark_missing(vec![info])
            .await
            .into_iter()
            .next()
            .unwrap_or_else(|| unreachable_project(project)))
    }

    /// Add the folder `path` (absolute, existing) as a project: the
    /// repository's top level when it is inside one. A root already known
    /// answers that project.
    ///
    /// # Errors
    /// `invalid_request` for a path that is not an absolute folder or a
    /// refused root; `conflict` past the bound.
    pub(crate) async fn add_project(
        &self,
        path: PathBuf,
        name: Option<ProjectName>,
    ) -> Result<ProjectInfo, WorkspaceError> {
        let folder = existing_folder(path).await?;
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
        &self,
        path: PathBuf,
        name: Option<ProjectName>,
    ) -> Result<ProjectInfo, WorkspaceError> {
        let git = self.git()?.clone();
        if !path.is_absolute() {
            return Err(WorkspaceError::Invalid(
                "the folder must be an absolute path".to_owned(),
            ));
        }
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

    /// Change a project in memory and bump its revision; the project after.
    fn update(
        &self,
        project: ProjectId,
        change: impl FnOnce(&mut ProjectInfo),
    ) -> Option<ProjectInfo> {
        let mut state = self.state();
        let entry = state.projects.get_mut(&project)?;
        change(&mut entry.info);
        entry.info.revision = entry.info.revision.wrapping_add(1);
        let _receivers = entry.revision.send_replace(entry.info.revision);
        Some(entry.info.clone())
    }

    /// The project whose root is `root`, if one is.
    fn project_at(&self, root: &Path) -> Option<ProjectInfo> {
        self.state()
            .projects
            .values()
            .find(|entry| entry.info.root == root)
            .map(|entry| entry.info.clone())
    }

    async fn insert_project(
        &self,
        root: PathBuf,
        git: bool,
        name: Option<ProjectName>,
        state: ProjectState,
    ) -> Result<ProjectInfo, WorkspaceError> {
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
        {
            let mut registry = self.state();
            if registry.projects.len() >= MAX_PROJECTS {
                return Err(WorkspaceError::Conflict(format!(
                    "this machine keeps at most {MAX_PROJECTS} projects"
                )));
            }
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
        self.save_project(info.clone()).await?;
        Ok(info)
    }

    /// Forget a project; its files stay. A clone still running is stopped.
    ///
    /// # Errors
    /// `not_found`; `conflict` while it has worktrees or live sessions.
    pub(crate) async fn remove_project(
        &self,
        project: ProjectId,
        live: &Live,
    ) -> Result<(), WorkspaceError> {
        let cancel = {
            let mut state = self.state();
            let entry = state
                .projects
                .get(&project)
                .ok_or(WorkspaceError::UnknownProject(project))?;
            if state
                .worktrees
                .values()
                .any(|worktree| worktree.info.project == project)
            {
                return Err(WorkspaceError::Conflict(
                    "the project has worktrees; remove them first".to_owned(),
                ));
            }
            let root = &entry.info.root;
            if live.iter().any(|(_, cwd)| cwd.starts_with(root)) {
                return Err(WorkspaceError::Conflict(
                    "sessions run in the project".to_owned(),
                ));
            }
            let removed = state.projects.remove(&project);
            state.approvals.retain(|(owner, _)| *owner != project);
            removed.and_then(|entry| entry.clone)
        };
        if let Some(cancel) = cancel {
            cancel.cancel();
        }
        self.store.forget_project(project).await.map_err(Into::into)
    }

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
    pub(super) async fn file_at(
        &self,
        root: &Path,
        base: &WorktreeBase,
    ) -> Result<Result<Option<ProjectFile>, String>, WorkspaceError> {
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
            Ok(Some(bytes)) => ProjectFile::parse(&bytes)
                .map(Some)
                .map_err(|error| error.to_string()),
            Ok(None) => Ok(None),
            Err(vessel_workspace::GitError::OutputLimit) => {
                Err(project_file::ProjectFileError::TooLarge.to_string())
            }
            Err(error) => return Err(error.into()),
        })
    }

    /// The project's file in a folder as it is.
    pub(super) async fn file_in(&self, folder: &Path) -> Result<Option<ProjectFile>, String> {
        let path = folder.join(project_file::FILE_NAME);
        tokio::task::spawn_blocking(move || {
            let metadata = match std::fs::metadata(&path) {
                Ok(metadata) => metadata,
                Err(_missing) => return Ok(None),
            };
            if metadata.len() > u64::try_from(project_file::MAX_BYTES).unwrap_or(u64::MAX) {
                return Err(project_file::ProjectFileError::TooLarge.to_string());
            }
            let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
            ProjectFile::parse(&bytes)
                .map(Some)
                .map_err(|error| error.to_string())
        })
        .await
        .unwrap_or_else(|_panicked| Err("reading the project's file stopped".to_owned()))
    }

    /// The wire's form of a parsed file, with its approval.
    pub(super) fn describe(
        &self,
        project: ProjectId,
        file: Result<Option<ProjectFile>, String>,
    ) -> ProjectCommands {
        match file {
            Err(message) => ProjectCommands::Invalid { message },
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

    /// A project's root.
    pub(super) fn project_root(&self, project: ProjectId) -> Result<PathBuf, WorkspaceError> {
        self.state()
            .projects
            .get(&project)
            .map(|entry| entry.info.root.clone())
            .ok_or(WorkspaceError::UnknownProject(project))
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
async fn canonical(path: PathBuf) -> PathBuf {
    tokio::task::spawn_blocking(move || std::fs::canonicalize(&path).unwrap_or(path))
        .await
        .unwrap_or_default()
}

/// Projects whose folder is gone, told as `missing`.
async fn mark_missing(projects: Vec<ProjectInfo>) -> Vec<ProjectInfo> {
    tokio::task::spawn_blocking(move || {
        projects
            .into_iter()
            .map(|mut info| {
                if matches!(info.state, ProjectState::Ready) && !info.root.is_dir() {
                    info.state = ProjectState::Missing;
                }
                info
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

/// The first eight hex digits of an id, for folder names.
pub(super) fn short(id: impl std::fmt::Display) -> String {
    id.to_string()
        .chars()
        .filter(char::is_ascii_hexdigit)
        .take(8)
        .collect()
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

#[expect(
    clippy::unreachable,
    reason = "the project was read under the same call a moment before"
)]
fn unreachable_project(project: ProjectId) -> ProjectInfo {
    unreachable!("project {project} vanished between two reads")
}
