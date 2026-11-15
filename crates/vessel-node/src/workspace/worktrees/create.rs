//! Making a worktree (`worktree.create`): the bounds, the folder and
//! branch git makes from what `start` resolved, the record, and the setup
//! it gets; and running setup again (`worktree.setup`). The commands
//! themselves run in the workspace's `setup`, never here.
//!
//! [Decision Log]
//! - `worktree.create` answers at once: the folder and branch are made in
//!   the call, setup runs afterwards as a task of the node's. The commands
//!   are read from the blob at the start commit, the one the folder is
//!   made from, never from the checkout (which setup could change).
//! - Making a worktree runs as a task of the node's, awaited by the call:
//!   a caller that goes away (a cancelled call, a closed page) leaves no
//!   folder made and unrecorded.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use vessel_proto::workspace::{
    BranchName, CreateWorktree, CreateWorktreeOutcome, ProjectCommands, ProjectState, SetupChoice,
    SetupState, WorktreeBase, WorktreeInfo, WorktreeRefusal,
};
use vessel_proto::{Creator, SessionId, WorktreeId};
use vessel_workspace::git::{AddRefusal, NewBranch};

use super::start::{Chosen, choose};
use crate::workspace::projects::{admission::ProjectAdmission, short};
use crate::workspace::{
    MAX_PER_AGENT, MAX_WORKTREES, State, WorkspaceError, Workspaces, WorktreeEntry,
};

impl Workspaces {
    /// Make a worktree for `creator` (and the session `created_for`, when
    /// one is named), and start its setup.
    ///
    /// # Errors
    /// `not_found`; `unsupported` without git; git's failures. The rules'
    /// refusals are `refused` outcomes.
    pub(crate) async fn create_worktree(
        self: &Arc<Self>,
        creator: Creator,
        request: CreateWorktree,
        created_for: Option<SessionId>,
    ) -> Result<CreateWorktreeOutcome, WorkspaceError> {
        self.create_worktree_with_id(creator, request, created_for, WorktreeId::random())
            .await
    }

    /// The legacy entry mints once before its tracked task starts.
    async fn create_worktree_with_id(
        self: &Arc<Self>,
        creator: Creator,
        request: CreateWorktree,
        created_for: Option<SessionId>,
        id: WorktreeId,
    ) -> Result<CreateWorktreeOutcome, WorkspaceError> {
        // A task of the node's: a caller that goes away does not leave a
        // folder made and unrecorded.
        let workspaces = Arc::clone(self);
        self.tasks
            .spawn(async move {
                // Preserve the existing typed legacy refusals before ready-root admission.
                if let Some(reason) = workspaces.creation_refusal(request.project)? {
                    return Ok(CreateWorktreeOutcome::Refused { reason });
                }
                let admission = workspaces.admit_project(request.project).await?;
                match request.setup {
                    SetupChoice::Skip => {
                        workspaces
                            .create_worktree_admitted(admission, creator, request, created_for, id)
                            .await
                    }
                    SetupChoice::Run => {
                        workspaces
                            .create_admitted_now(&admission, creator, request, created_for, id)
                            .await
                    }
                }
            })
            .await
            .map_err(WorkspaceError::project_task)?
    }

    fn creation_refusal(
        &self,
        project: vessel_proto::ProjectId,
    ) -> Result<Option<WorktreeRefusal>, WorkspaceError> {
        let state = self.state();
        let entry = state
            .projects
            .get(&project)
            .ok_or(WorkspaceError::UnknownProject(project))?;
        Ok(if !entry.info.git {
            Some(WorktreeRefusal::NotGit)
        } else if !matches!(entry.info.state, ProjectState::Ready) {
            Some(WorktreeRefusal::NotReady)
        } else {
            None
        })
    }

    /// Provision backing without bootstrap; the driver retains its own Arc through
    /// descriptor finalization. Workspace-aware setup is a separate later admission.
    pub(crate) async fn create_worktree_admitted(
        self: &Arc<Self>,
        admission: Arc<ProjectAdmission>,
        creator: Creator,
        request: CreateWorktree,
        created_for: Option<SessionId>,
        id: WorktreeId,
    ) -> Result<CreateWorktreeOutcome, WorkspaceError> {
        admission.require(self, request.project)?;
        if request.setup != SetupChoice::Skip {
            return Err(WorkspaceError::Invalid("workspace backing creation requires setup to be skipped until its descriptor is finalized".to_owned()));
        }
        let workspaces = Arc::clone(self);
        self.tasks
            .spawn(async move {
                workspaces
                    .create_admitted_now(&admission, creator, request, created_for, id)
                    .await
            })
            .await
            .map_err(WorkspaceError::project_task)?
    }

    async fn create_admitted_now(
        self: &Arc<Self>,
        admission: &ProjectAdmission,
        creator: Creator,
        request: CreateWorktree,
        created_for: Option<SessionId>,
        id: WorktreeId,
    ) -> Result<CreateWorktreeOutcome, WorkspaceError> {
        admission.require(self, request.project)?;
        let refused = |reason| Ok(CreateWorktreeOutcome::Refused { reason });
        let project = request.project;
        let (root, default_branch) = {
            let mut state = self.state();
            require_new_worktree_id(&state, id)?;
            let entry = state
                .projects
                .get(&project)
                .ok_or(WorkspaceError::UnknownProject(project))?;
            if !entry.info.git {
                return refused(WorktreeRefusal::NotGit);
            }
            if !matches!(entry.info.state, ProjectState::Ready) {
                return refused(WorktreeRefusal::NotReady);
            }
            let found = (
                admission.root().path().to_path_buf(),
                entry.info.default_branch.clone(),
            );
            if over_limit(&state, &creator) {
                return refused(WorktreeRefusal::Limit);
            }
            // Counted under the lock that checked the limit, so creations
            // at once see each other; given back when this one ends,
            // however it ends (`Making`).
            state.making.push(creator.clone());
            let _new_id = state.making_ids.insert(id);
            found
        };
        let _making = Making {
            workspaces: self,
            creator: creator.clone(),
            id,
        };
        let git = self.git()?.clone();
        let Chosen {
            branch,
            start,
            created,
            restore,
        } = match choose(&git, &root, default_branch, request.branch, id).await? {
            Ok(chosen) => chosen,
            Err(reason) => return refused(reason),
        };
        let commands = self
            .commands(
                project,
                Some(&WorktreeBase::Commit {
                    id: start.commit.clone(),
                }),
            )
            .await?;
        let Some(setup) = plan(request.setup, &commands) else {
            return Ok(CreateWorktreeOutcome::NeedsApproval { commands });
        };
        let path = self
            .dirs
            .worktrees
            .join(project.to_string())
            .join(short(id));
        if let Err(reason) =
            make_folder(&git, &root, &path, &branch, created.as_ref(), restore).await?
        {
            return refused(reason);
        }
        let branch = match &branch {
            NewBranch::Create { name, .. } | NewBranch::Existing { name } => name.clone(),
        };
        let info = WorktreeInfo {
            id,
            project,
            path: canonical(path).await,
            branch: Some(branch),
            created_branch: created,
            base: Some(start),
            created_at: Utc::now(),
            created_by: creator,
            created_for,
            sessions: Vec::new(),
            setup: match setup {
                Planned::Skipped => SetupState::Skipped,
                Planned::Nothing | Planned::Run(_) => SetupState::None,
            },
            kept: None,
        };
        self.remember_created_worktree(&info);
        self.save_worktree(info.clone()).await?;
        let info = match setup {
            Planned::Run(commands) => self.start_setup(id, commands)?,
            Planned::Skipped | Planned::Nothing => info,
        };
        Ok(CreateWorktreeOutcome::Created { worktree: info })
    }

    /// Preserve the existing publication order; durability and setup stay with the tracked creator.
    fn remember_created_worktree(&self, info: &WorktreeInfo) {
        let (written, _) = tokio::sync::watch::channel(0);
        drop(self.state().worktrees.insert(
            info.id,
            WorktreeEntry {
                info: info.clone(),
                written: Arc::new(written),
                running: None,
                starting: 0,
            },
        ));
    }

    /// Run setup again with the commands of the worktree's file as it is.
    ///
    /// # Errors
    /// `conflict` while a run holds it, or when that set is not approved
    /// (or the file cannot be used); `not_found`.
    pub(crate) async fn run_setup(
        self: &Arc<Self>,
        worktree: WorktreeId,
    ) -> Result<WorktreeInfo, WorkspaceError> {
        let info = self.worktree(worktree)?;
        let file = self.file_in(&info.path).await;
        match self.describe(info.project, file) {
            ProjectCommands::Listed {
                approved: true,
                setup,
                ..
            } => self.start_setup(worktree, setup),
            ProjectCommands::Listed { .. } => Err(WorkspaceError::Conflict(
                "the worktree's commands are not approved; approve them with `project.approve`"
                    .to_owned(),
            )),
            ProjectCommands::Invalid { message } => Err(WorkspaceError::Conflict(format!(
                "the worktree's `vessel.toml` cannot be used: {message}"
            ))),
            ProjectCommands::None => self.start_setup(worktree, Vec::new()),
        }
    }
}

/// `git worktree add` into `path`, and the snapshot put back in it; a
/// refusal of git's as the wire says it.
async fn make_folder(
    git: &vessel_workspace::Git,
    root: &Path,
    path: &Path,
    branch: &NewBranch,
    created: Option<&BranchName>,
    restore: Option<vessel_proto::workspace::CommitId>,
) -> Result<Result<(), WorktreeRefusal>, WorkspaceError> {
    if let Some(folder) = path.parent() {
        prepare(folder, path).await?;
    }
    if let Err(refusal) = git.add_worktree(root, path, branch).await? {
        return Ok(Err(match refusal {
            AddRefusal::BranchExists => WorktreeRefusal::BranchExists,
            AddRefusal::CheckedOutElsewhere(path) => WorktreeRefusal::CheckedOutElsewhere { path },
            AddRefusal::NoSuchBranch => WorktreeRefusal::NoSuchBranch,
        }));
    }
    if let Some(snapshot) = restore
        && let Err(error) = git.restore_snapshot(path, &snapshot).await
    {
        if let Err(cleanup) = git.remove_worktree(root, path, created, true).await {
            tracing::warn!(error = %cleanup, "a worktree whose snapshot failed stays");
        }
        return Err(error.into());
    }
    Ok(Ok(()))
}

/// What setup a new worktree gets from the commands at its start; `None`
/// when they need a person's approval first (or cannot be used).
fn plan(choice: SetupChoice, commands: &ProjectCommands) -> Option<Planned> {
    Some(match (choice, commands) {
        (
            SetupChoice::Skip,
            ProjectCommands::Listed {
                setup, terminals, ..
            },
        ) if !setup.is_empty() || !terminals.is_empty() => Planned::Skipped,
        (SetupChoice::Skip, _) | (SetupChoice::Run, ProjectCommands::None) => Planned::Nothing,
        // Terminal-only plans still contain approved executable shell input.
        // Empty setup must not skip the command-set approval boundary.
        (
            SetupChoice::Run,
            ProjectCommands::Listed {
                setup, terminals, ..
            },
        ) if setup.is_empty() && terminals.is_empty() => Planned::Nothing,
        (
            SetupChoice::Run,
            ProjectCommands::Listed {
                approved: true,
                setup,
                ..
            },
        ) => {
            if setup.is_empty() {
                Planned::Nothing
            } else {
                Planned::Run(setup.clone())
            }
        }
        (SetupChoice::Run, ProjectCommands::Listed { .. } | ProjectCommands::Invalid { .. }) => {
            return None;
        }
    })
}

/// Whether one more worktree for `creator` would pass the node's bound,
/// or an agent's, counting the ones being made.
fn over_limit(state: &State, creator: &Creator) -> bool {
    if state.worktrees.len().saturating_add(state.making.len()) >= MAX_WORKTREES {
        return true;
    }
    let own = state
        .worktrees
        .values()
        .filter(|entry| entry.info.created_by == *creator)
        .count()
        .saturating_add(state.making.iter().filter(|made| *made == creator).count());
    matches!(creator, Creator::Agent { .. }) && own >= MAX_PER_AGENT
}

/// One worktree being made, counted (in `State::making`, by the check
/// that let it through) against the limits until this is dropped.
struct Making<'a> {
    workspaces: &'a Workspaces,
    creator: Creator,
    id: WorktreeId,
}

impl Drop for Making<'_> {
    fn drop(&mut self) {
        let mut state = self.workspaces.state();
        let _released = state.making_ids.remove(&self.id);
        if let Some(at) = state.making.iter().position(|made| *made == self.creator) {
            let _given_back = state.making.swap_remove(at);
        }
    }
}

/// What setup a new worktree gets.
enum Planned {
    Nothing,
    Skipped,
    Run(Vec<vessel_proto::workspace::ProjectCommand>),
}

/// Make the project's folder of worktrees; the worktree's own must not
/// exist yet.
async fn prepare(folder: &Path, path: &Path) -> Result<(), WorkspaceError> {
    tokio::fs::create_dir_all(folder)
        .await
        .map_err(|error| WorkspaceError::Io(error.to_string()))?;
    if tokio::fs::symlink_metadata(path).await.is_ok() {
        return Err(WorkspaceError::Conflict(format!(
            "`{}` exists",
            path.display()
        )));
    }
    Ok(())
}

async fn canonical(path: PathBuf) -> PathBuf {
    tokio::task::spawn_blocking(move || std::fs::canonicalize(&path).unwrap_or(path))
        .await
        .unwrap_or_default()
}

/// A durably reserved identity must never replay an already-registered backing effect.
fn require_new_worktree_id(state: &State, id: WorktreeId) -> Result<(), WorkspaceError> {
    if state.worktrees.contains_key(&id) || state.making_ids.contains(&id) {
        return Err(WorkspaceError::Conflict(
            "the reserved worktree identity already exists".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "create/terminal_plan_tests.rs"]
mod terminal_plan_tests;
