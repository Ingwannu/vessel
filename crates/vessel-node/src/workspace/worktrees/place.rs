//! Where a session runs: the folder its `place` resolves to and the
//! notices its log starts with, the worktree it claims, and the project,
//! worktree and owner its spec names. Whether an agent may use a place is
//! the caller's check, not this module's.
//!
//! [Decision Log]
//! - A worktree is made before its session (the three steps of ADR 0027),
//!   so the first session started in one made for no session claims it
//!   (`claim`), and tidying then removes it with that session.

use std::path::PathBuf;

use vessel_proto::workspace::{
    ProjectState, SessionPlace, SessionWorkspace, SetupFailure, SetupState, WorktreeInfo,
};
use vessel_proto::{Creator, SessionId, SessionSpec, WorktreeId};
use vessel_workspace::paths::Root;

use crate::workspace::{WorkspaceError, Workspaces};

/// Where a session's place resolved to.
#[derive(Debug)]
pub(crate) struct Placed {
    /// Its working folder.
    pub(crate) cwd: PathBuf,
    /// The lines its log starts with.
    pub(crate) notices: Vec<String>,
}

impl Workspaces {
    /// The folder a session's `place` resolves to, and the notices its log
    /// starts with. Ownership (for an agent) is the caller's check.
    ///
    /// # Errors
    /// `not_found`; `conflict` for a project not ready, a worktree whose
    /// setup runs, or a folder gone.
    pub(crate) async fn place(&self, place: &SessionPlace) -> Result<Placed, WorkspaceError> {
        match place {
            SessionPlace::Project { project, dir } => {
                let (root, name) = {
                    let state = self.state();
                    let entry = state
                        .projects
                        .get(project)
                        .ok_or(WorkspaceError::UnknownProject(*project))?;
                    if !matches!(entry.info.state, ProjectState::Ready) {
                        return Err(WorkspaceError::Conflict(
                            "the project is not ready (cloning, failed, or its folder is gone)"
                                .to_owned(),
                        ));
                    }
                    (entry.info.root.clone(), entry.info.name.clone())
                };
                let dir = dir.clone();
                let (cwd, folder) = tokio::task::spawn_blocking(move || {
                    let opened = Root::open(&root)?;
                    let cwd = match &dir {
                        Some(dir) if !dir.is_root() => opened.existing(dir)?,
                        _ => opened.path().to_path_buf(),
                    };
                    let folder = cwd.is_dir();
                    Ok::<_, WorkspaceError>((cwd, folder))
                })
                .await
                .map_err(|_panicked| {
                    WorkspaceError::Io("resolving the folder stopped".to_owned())
                })??;
                if !folder {
                    return Err(WorkspaceError::Invalid(
                        "the place is not a folder".to_owned(),
                    ));
                }
                let notices = vec![format!("vessel: in project {name} ({})", cwd.display())];
                Ok(Placed { cwd, notices })
            }
            SessionPlace::Worktree { worktree } => {
                let info = {
                    let state = self.state();
                    let entry = state
                        .worktrees
                        .get(worktree)
                        .ok_or(WorkspaceError::UnknownWorktree(*worktree))?;
                    if entry.running.is_some() {
                        return Err(WorkspaceError::Conflict(
                            "setup or teardown runs in this worktree; follow `worktree.output` \
                             until it ends"
                                .to_owned(),
                        ));
                    }
                    entry.info.clone()
                };
                if !tokio::fs::try_exists(&info.path).await.unwrap_or(false) {
                    return Err(WorkspaceError::Conflict(
                        "the worktree's folder is gone from this machine".to_owned(),
                    ));
                }
                let notices = self.worktree_notices(&info).await;
                Ok(Placed {
                    cwd: info.path,
                    notices,
                })
            }
        }
    }

    /// Whether setup, teardown or a removal holds `worktree` now: a
    /// session does not start in it then (`session.resume` asks, as
    /// `place` does).
    pub(crate) fn busy(&self, worktree: WorktreeId) -> bool {
        self.state()
            .worktrees
            .get(&worktree)
            .is_some_and(|entry| entry.running.is_some())
    }

    async fn worktree_notices(&self, info: &WorktreeInfo) -> Vec<String> {
        let mut notices = vec![format!("vessel: in worktree {}", info.path.display())];
        if let Some(branch) = &info.branch {
            let from = info.base.as_ref().map(|start| match &start.name {
                Some(name) => format!(" from {name} ({})", short_commit(start.commit.as_str())),
                None => format!(" from {}", short_commit(start.commit.as_str())),
            });
            notices.push(format!(
                "vessel: on branch {branch}{}",
                from.unwrap_or_default()
            ));
        }
        let setup = match &info.setup {
            SetupState::Skipped => Some("vessel: setup was skipped".to_owned()),
            SetupState::Succeeded { .. } => Some("vessel: setup succeeded".to_owned()),
            SetupState::Failed { failure, .. } => Some(format!(
                "vessel: setup failed ({}); the session starts anyway",
                describe_failure(*failure)
            )),
            SetupState::None | SetupState::Running { .. } | SetupState::Unknown => None,
        };
        if let Some(setup) = setup {
            notices.push(setup);
            if matches!(info.setup, SetupState::Failed { .. }) {
                let tail = self.log_tail(info.id).await;
                if !tail.is_empty() {
                    notices.push(format!("vessel: setup's last lines:\n{tail}"));
                }
            }
        }
        notices
    }

    /// The first session started in a worktree made for no session claims
    /// it: tidying (`retire`) then removes it with that session, by the
    /// rule. A worktree is made before its session (the three steps of
    /// ADR 0027), so this is how it becomes the session's own.
    pub(crate) async fn claim(&self, worktree: WorktreeId, session: SessionId) {
        let claimed = {
            let mut state = self.state();
            state.worktrees.get_mut(&worktree).and_then(|entry| {
                entry.info.created_for.is_none().then(|| {
                    entry.info.created_for = Some(session);
                    entry.info.clone()
                })
            })
        };
        if let Some(info) = claimed
            && let Err(error) = self.save_worktree(info).await
        {
            tracing::warn!(%error, "could not record a worktree's session");
        }
    }

    /// The project, worktree and branch a session's spec names, as the
    /// registry knows them now.
    pub(crate) fn workspace_of(&self, spec: &SessionSpec) -> Option<SessionWorkspace> {
        match spec.place.as_deref()? {
            SessionPlace::Project { project, .. } => Some(SessionWorkspace {
                project: *project,
                worktree: None,
                branch: None,
            }),
            SessionPlace::Worktree { worktree } => {
                let state = self.state();
                let entry = state.worktrees.get(worktree)?;
                Some(SessionWorkspace {
                    project: entry.info.project,
                    worktree: Some(*worktree),
                    branch: entry.info.branch.clone(),
                })
            }
        }
    }

    /// A worktree's creator and the session it was made for, for an
    /// agent's ownership check.
    pub(crate) fn owner_of(
        &self,
        worktree: WorktreeId,
    ) -> Result<(Creator, Option<SessionId>), WorkspaceError> {
        let info = self.worktree(worktree)?;
        Ok((info.created_by, info.created_for))
    }
}

fn short_commit(commit: &str) -> &str {
    commit.get(..12).unwrap_or(commit)
}

fn describe_failure(failure: SetupFailure) -> String {
    match failure {
        SetupFailure::Exit {
            command,
            code: Some(code),
        } => format!("command {} exited with {code}", command.saturating_add(1)),
        SetupFailure::Exit {
            command,
            code: None,
        } => {
            format!("command {} was stopped", command.saturating_add(1))
        }
        SetupFailure::TimedOut => "it reached its time limit".to_owned(),
        SetupFailure::Spawn => "the shell did not start".to_owned(),
        SetupFailure::Unknown => "for a reason this node does not know".to_owned(),
    }
}
