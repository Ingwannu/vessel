//! Projects: the node's record of them, read back when the node starts,
//! listed, waited on and forgotten, and the lookups the other workspace
//! modules make in it (protocol 10, ADR 0027). Adding a project is
//! `adding`'s; what a project's file would run, and its approval, is
//! `commands`'.

mod adding;
pub(super) mod admission;
mod commands;

use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::sync::watch;
use vessel_proto::workspace::{CloneFailure, ProjectInfo, ProjectList, ProjectState};
use vessel_proto::{ProjectId, SessionId, TerminalId};

use super::{MAX_PROJECTS, MAX_WAIT_SECS, ProjectEntry, WorkspaceError, Workspaces};

/// What runs in a folder: a live session, or (protocol 11, ADR 0031) a
/// live terminal. Either keeps a worktree or a project from being removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Occupant {
    /// A live session.
    Session(SessionId),
    /// A live terminal (a shell, a script's run, a service).
    Terminal(TerminalId),
}

/// What runs where, as the node knows it now.
pub(crate) type Live = [(Occupant, PathBuf)];

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
            for (project, set) in stored.approved_sets {
                state.approved_sets.entry(project).or_default().push(set);
            }
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
        mark_missing(vec![info])
            .await
            .pop()
            .ok_or(WorkspaceError::UnknownProject(project))
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

    /// Forget a project; its files stay. A clone still running is stopped.
    ///
    /// # Errors
    /// `not_found`; `conflict` while it has worktrees, or live sessions or
    /// terminals (protocol 11) run in it.
    pub(crate) async fn remove_project(
        &self,
        project: ProjectId,
        live: &Live,
    ) -> Result<(), WorkspaceError> {
        let _capacity = self.project_operation_capacity()?;
        let _serial = self.project_registrations.lock().await;
        self.require_no_registry_reference(project).await?;
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
            if let Some((occupant, _)) = live.iter().find(|(_, cwd)| cwd.starts_with(root)) {
                return Err(WorkspaceError::Conflict(
                    match occupant {
                        Occupant::Session(_) => "sessions run in the project",
                        Occupant::Terminal(_) => "terminals run in the project; close them first",
                    }
                    .to_owned(),
                ));
            }
            let removed = state.projects.remove(&project);
            state.approvals.retain(|(owner, _)| *owner != project);
            let _sets = state.approved_sets.remove(&project);
            removed.and_then(|entry| entry.clone)
        };
        if let Some(cancel) = cancel {
            cancel.cancel();
        }
        self.store.forget_project(project).await.map_err(Into::into)
    }

    /// Whether the node has `project` (protocol 11: a terminal placed in
    /// one ends if it was removed while the terminal started).
    pub(crate) fn has_project(&self, project: ProjectId) -> bool {
        self.state().projects.contains_key(&project)
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

/// Projects whose folder is gone, told as `missing`.
async fn mark_missing(mut projects: Vec<ProjectInfo>) -> Vec<ProjectInfo> {
    // Looked at on the blocking pool, the list kept here: a look that
    // stops (the runtime shutting down) leaves them as recorded rather
    // than dropping them.
    let roots: Vec<PathBuf> = projects.iter().map(|info| info.root.clone()).collect();
    let present = tokio::task::spawn_blocking(move || {
        roots
            .iter()
            .map(|root| root.is_dir())
            .collect::<Vec<bool>>()
    })
    .await;
    match present {
        Ok(present) => {
            for (info, present) in projects.iter_mut().zip(present) {
                if matches!(info.state, ProjectState::Ready) && !present {
                    info.state = ProjectState::Missing;
                }
            }
        }
        Err(error) => tracing::debug!(%error, "looking for projects' folders stopped"),
    }
    projects
}

/// The first eight hex digits of an id, for folder names.
pub(super) fn short(id: impl std::fmt::Display) -> String {
    id.to_string()
        .chars()
        .filter(char::is_ascii_hexdigit)
        .take(8)
        .collect()
}
