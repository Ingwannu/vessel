//! Worktrees: making one (with its setup), removing one without losing
//! work, snapshots, tidying a session's worktree, and where a session runs
//! (protocol 10, ADR 0024).
//!
//! [Decision Log]
//! - `worktree.create` answers at once: the folder and branch are made in
//!   the call, setup runs afterwards as a task of the node's. The commands
//!   are read from the blob at the start commit, the one the folder is
//!   made from, never from the checkout (which setup could change).
//! - Removal follows one rule for a person's `worktree.remove` and for
//!   tidying (`retire`): inspect, teardown (when approved), inspect again,
//!   then git's own non-forced removal. Only `discard`, or a snapshot kept
//!   first, forces it. A worktree a live session is in, or whose setup or
//!   teardown runs, is kept (`in_use`).
//! - Making and removing a worktree run as tasks of the node's, awaited by
//!   the call: a caller that goes away (a cancelled call, a closed page)
//!   leaves neither a folder made and unrecorded nor a worktree claimed
//!   by a removal that stopped halfway.
//! - Tidying is offered to track A's archive and delete as `retire`; this
//!   track calls it from nowhere yet (the coordinator wires it when both
//!   tracks are merged). A worktree it keeps says why in `kept`. A
//!   worktree is made before its session (the three steps), so the first
//!   session started in one made for no session claims it (`claim`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use tokio_util::sync::CancellationToken;
use vessel_proto::workspace::{
    BranchName, CreateWorktree, CreateWorktreeOutcome, IfLost, KeptReason, ProjectCommands,
    ProjectState, RemoveWorktreeOutcome, SessionPlace, SessionWorkspace, SetupChoice, SetupFailure,
    SetupState, Snapshot, SnapshotList, WorktreeBase, WorktreeBranch, WorktreeInfo, WorktreeList,
    WorktreeRefusal, WorktreeStart,
};
use vessel_proto::{Creator, ProjectId, SessionId, SessionSpec, WorktreeId};
use vessel_workspace::git::{AddRefusal, NewBranch};
use vessel_workspace::paths::Root;

use super::projects::{Live, short};
use super::setup::{Kind, note};
use super::{MAX_PER_AGENT, MAX_WORKTREES, WorkspaceError, Workspaces, WorktreeEntry};

/// Where a session's place resolved to.
#[derive(Debug)]
pub(crate) struct Placed {
    /// Its working folder.
    pub(crate) cwd: PathBuf,
    /// The lines its log starts with.
    pub(crate) notices: Vec<String>,
}

/// How a removal ended, inside this module.
enum Removal {
    Done(Option<Snapshot>),
    Kept(Vec<KeptReason>),
}

impl Workspaces {
    /// Read the stored worktrees back: a setup or teardown the node's stop
    /// interrupted is told as failed.
    pub(super) async fn restore_worktrees(&self, stored: Vec<WorktreeInfo>) {
        let mut rewrite = Vec::new();
        {
            let mut state = self.state();
            for mut info in stored {
                if matches!(info.setup, SetupState::Running { .. }) {
                    info.setup = SetupState::Failed {
                        failure: SetupFailure::Exit {
                            command: 0,
                            code: None,
                        },
                        finished_at: Utc::now(),
                    };
                    rewrite.push(info.clone());
                }
                let (written, _) = tokio::sync::watch::channel(0);
                drop(state.worktrees.insert(
                    info.id,
                    WorktreeEntry {
                        info,
                        written: Arc::new(written),
                        running: None,
                    },
                ));
            }
        }
        for info in rewrite {
            note(
                &self.log_path(info.id),
                "vessel: the node stopped during setup",
            )
            .await;
            if let Err(error) = self.save_worktree(info).await {
                tracing::warn!(%error, "could not record an interrupted setup");
            }
        }
    }

    /// Every worktree, or a project's, oldest first, with the live
    /// sessions in each.
    pub(crate) fn list_worktrees(&self, project: Option<ProjectId>, live: &Live) -> WorktreeList {
        let mut worktrees: Vec<WorktreeInfo> = self
            .state()
            .worktrees
            .values()
            .filter(|entry| project.is_none_or(|project| entry.info.project == project))
            .map(|entry| with_sessions(entry.info.clone(), live))
            .collect();
        worktrees.sort_by(|a, b| (a.created_at, a.id).cmp(&(b.created_at, b.id)));
        let more = worktrees.len() > MAX_WORKTREES;
        worktrees.truncate(MAX_WORKTREES);
        WorktreeList { worktrees, more }
    }

    /// One worktree as recorded.
    ///
    /// # Errors
    /// `not_found`.
    pub(crate) fn worktree(&self, worktree: WorktreeId) -> Result<WorktreeInfo, WorkspaceError> {
        self.state()
            .worktrees
            .get(&worktree)
            .map(|entry| entry.info.clone())
            .ok_or(WorkspaceError::UnknownWorktree(worktree))
    }

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
        // A task of the node's: a caller that goes away does not leave a
        // folder made and unrecorded.
        let workspaces = Arc::clone(self);
        self.tasks
            .spawn(async move { workspaces.create_now(creator, request, created_for).await })
            .await
            .map_err(|_stopped| WorkspaceError::Io("making the worktree stopped".to_owned()))?
    }

    async fn create_now(
        self: &Arc<Self>,
        creator: Creator,
        request: CreateWorktree,
        created_for: Option<SessionId>,
    ) -> Result<CreateWorktreeOutcome, WorkspaceError> {
        let refused = |reason| Ok(CreateWorktreeOutcome::Refused { reason });
        let project = request.project;
        let (root, default_branch) = {
            let state = self.state();
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
            if over_limit(&state.worktrees, &creator) {
                return refused(WorktreeRefusal::Limit);
            }
            (entry.info.root.clone(), entry.info.default_branch.clone())
        };
        let git = self.git()?.clone();
        let id = WorktreeId::random();
        let Chosen {
            branch,
            start,
            created,
            restore,
        } = match choose(&git, &root, default_branch, request.branch, id).await? {
            Ok(chosen) => chosen,
            Err(reason) => return refused(reason),
        };
        let commands = {
            let file = self
                .file_at(
                    &root,
                    &WorktreeBase::Commit {
                        id: start.commit.clone(),
                    },
                )
                .await?;
            self.describe(project, file)
        };
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
        {
            let (written, _) = tokio::sync::watch::channel(0);
            drop(self.state().worktrees.insert(
                id,
                WorktreeEntry {
                    info: info.clone(),
                    written: Arc::new(written),
                    running: None,
                },
            ));
        }
        self.save_worktree(info.clone()).await?;
        let info = match setup {
            Planned::Run(commands) => self.start_setup(id, commands)?,
            Planned::Skipped | Planned::Nothing => info,
        };
        Ok(CreateWorktreeOutcome::Created { worktree: info })
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

    /// `worktree.remove`: by the rule, `if_lost` saying what to do with work
    /// that would be lost.
    ///
    /// # Errors
    /// `not_found`; `unsupported` without git; git's failures.
    pub(crate) async fn remove_worktree(
        self: &Arc<Self>,
        worktree: WorktreeId,
        if_lost: IfLost,
        live: &Live,
    ) -> Result<RemoveWorktreeOutcome, WorkspaceError> {
        Ok(match self.remove(worktree, if_lost, live).await? {
            Removal::Done(snapshot) => RemoveWorktreeOutcome::Removed { snapshot },
            Removal::Kept(reasons) => RemoveWorktreeOutcome::Kept { reasons },
        })
    }

    /// Tidy the worktree made for `session` (now archived, deleted or
    /// dropped by retention), unless another live session is in it or
    /// something would be lost; a kept one records why. `None` when the
    /// session had no worktree of its own.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "track A's archive and delete call it; the coordinator wires it when both \
                      tracks are merged"
        )
    )]
    pub(crate) async fn retire(
        self: &Arc<Self>,
        session: SessionId,
        live: &Live,
    ) -> Option<RemoveWorktreeOutcome> {
        let worktree = self
            .state()
            .worktrees
            .values()
            .find(|entry| entry.info.created_for == Some(session))
            .map(|entry| entry.info.id)?;
        let others: Vec<(SessionId, PathBuf)> = live
            .iter()
            .filter(|(id, _)| *id != session)
            .cloned()
            .collect();
        let outcome = match self.remove(worktree, IfLost::Keep, &others).await {
            Ok(Removal::Done(snapshot)) => RemoveWorktreeOutcome::Removed { snapshot },
            Ok(Removal::Kept(reasons)) => RemoveWorktreeOutcome::Kept { reasons },
            Err(error) => {
                tracing::warn!(%error, "a session's worktree could not be tidied");
                RemoveWorktreeOutcome::Kept {
                    reasons: vec![KeptReason::InspectionFailed],
                }
            }
        };
        if let RemoveWorktreeOutcome::Kept { reasons } = &outcome {
            let kept = {
                let mut state = self.state();
                state.worktrees.get_mut(&worktree).map(|entry| {
                    entry.info.kept = reasons.first().copied();
                    entry.info.clone()
                })
            };
            if let Some(info) = kept
                && let Err(error) = self.save_worktree(info).await
            {
                tracing::warn!(%error, "could not record why a worktree was kept");
            }
        }
        Some(outcome)
    }

    /// Claim the worktree, remove it by the rule, release it when kept.
    /// The removal runs as a task of the node's: a caller that goes away
    /// mid-teardown neither stops it halfway nor leaves the worktree
    /// claimed.
    async fn remove(
        self: &Arc<Self>,
        worktree: WorktreeId,
        if_lost: IfLost,
        live: &Live,
    ) -> Result<Removal, WorkspaceError> {
        let git = self.git()?.clone();
        let claim = self.stop.child_token();
        let (info, root, written) = {
            let mut state = self.state();
            let root = {
                let entry = state
                    .worktrees
                    .get(&worktree)
                    .ok_or(WorkspaceError::UnknownWorktree(worktree))?;
                state
                    .projects
                    .get(&entry.info.project)
                    .map(|project| project.info.root.clone())
                    .ok_or(WorkspaceError::UnknownProject(entry.info.project))?
            };
            let entry = state
                .worktrees
                .get_mut(&worktree)
                .ok_or(WorkspaceError::UnknownWorktree(worktree))?;
            let in_use = live
                .iter()
                .any(|(_, cwd)| cwd.starts_with(&entry.info.path));
            if in_use || entry.running.is_some() {
                return Ok(Removal::Kept(vec![KeptReason::InUse]));
            }
            entry.running = Some(claim.clone());
            (entry.info.clone(), root, Arc::clone(&entry.written))
        };
        let workspaces = Arc::clone(self);
        self.tasks
            .spawn(async move {
                let removal = workspaces
                    .remove_claimed(&git, &info, &root, if_lost, claim, &written)
                    .await;
                workspaces.settle(worktree, removal).await
            })
            .await
            .map_err(|_stopped| WorkspaceError::Io("removing the worktree stopped".to_owned()))?
    }

    /// After a removal: a kept worktree is released, a removed one
    /// forgotten with its log.
    async fn settle(
        &self,
        worktree: WorktreeId,
        removal: Result<Removal, WorkspaceError>,
    ) -> Result<Removal, WorkspaceError> {
        if !matches!(removal, Ok(Removal::Done(_))) {
            if let Some(entry) = self.state().worktrees.get_mut(&worktree) {
                entry.running = None;
            }
            return removal;
        }
        drop(self.state().worktrees.remove(&worktree));
        self.store.forget_worktree(worktree).await?;
        if let Err(error) = tokio::fs::remove_file(self.log_path(worktree)).await
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::debug!(%error, "a removed worktree's log stays");
        }
        removal
    }

    async fn remove_claimed(
        &self,
        git: &vessel_workspace::Git,
        info: &WorktreeInfo,
        root: &Path,
        if_lost: IfLost,
        claim: CancellationToken,
        written: &vessel_workspace::commands::Written,
    ) -> Result<Removal, WorkspaceError> {
        let path = &info.path;
        let created = info.created_branch.as_ref();
        if !tokio::fs::try_exists(path).await.unwrap_or(false) {
            // The folder went another way; git's record of it goes at its
            // next prune. Nothing is left to lose here.
            return Ok(Removal::Done(None));
        }
        let mut snapshot = None;
        let mut force = false;
        match if_lost {
            IfLost::Discard => force = true,
            IfLost::Keep | IfLost::Snapshot => {
                let lost = git.would_lose(path, created).await?;
                if !lost.is_empty() {
                    if if_lost == IfLost::Keep {
                        return Ok(Removal::Kept(lost));
                    }
                    match git.snapshot(path, info.id, &self.dirs.scratch).await? {
                        Ok(kept) => {
                            snapshot = Some(kept);
                            force = true;
                        }
                        Err(refusal) => {
                            tracing::info!(?refusal, "no snapshot: the worktree stays");
                            return Ok(Removal::Kept(lost));
                        }
                    }
                }
            }
        }
        let file = self.file_in(path).await;
        if let ProjectCommands::Listed {
            approved: true,
            teardown,
            ..
        } = self.describe(info.project, file)
            && !teardown.is_empty()
            && let Some(failure) = self
                .run_commands(info, &teardown, Kind::Teardown, claim, written)
                .await
        {
            tracing::info!(
                ?failure,
                "a worktree's teardown failed; removing it goes on"
            );
        }
        if !force {
            let lost = git.would_lose(path, created).await?;
            if !lost.is_empty() {
                return Ok(Removal::Kept(lost));
            }
        }
        match git.remove_worktree(root, path, created, force).await {
            Ok(()) => Ok(Removal::Done(snapshot)),
            Err(vessel_workspace::GitError::Failed { message, .. }) if !force => {
                tracing::info!(%message, "git kept the worktree");
                Ok(Removal::Kept(vec![KeptReason::InspectionFailed]))
            }
            Err(error) => Err(error.into()),
        }
    }

    /// A project's snapshots.
    ///
    /// # Errors
    /// `not_found`; `unsupported`.
    pub(crate) async fn list_snapshots(
        &self,
        project: ProjectId,
    ) -> Result<SnapshotList, WorkspaceError> {
        let root = self.project_root(project)?;
        Ok(self.git()?.snapshots(&root).await?)
    }

    /// Delete a snapshot's ref.
    ///
    /// # Errors
    /// `not_found` for a snapshot the project does not have.
    pub(crate) async fn drop_snapshot(
        &self,
        project: ProjectId,
        worktree: WorktreeId,
    ) -> Result<(), WorkspaceError> {
        let root = self.project_root(project)?;
        if self.git()?.drop_snapshot(&root, worktree).await? {
            Ok(())
        } else {
            Err(WorkspaceError::UnknownSnapshot(worktree))
        }
    }

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
                let cwd = tokio::task::spawn_blocking(move || {
                    let opened = Root::open(&root)?;
                    match &dir {
                        Some(dir) if !dir.is_root() => opened.existing(dir),
                        _ => Ok(opened.path().to_path_buf()),
                    }
                })
                .await
                .map_err(|_panicked| {
                    WorkspaceError::Io("resolving the folder stopped".to_owned())
                })??;
                if !cwd.is_dir() {
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
    /// ADR 0024), so this is how it becomes the session's own.
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

    /// The project's worktrees follow `git.rename_branch`: a branch is the
    /// repository's, so every worktree that has it checked out, or created
    /// it, records the new name.
    pub(crate) async fn branch_renamed(
        &self,
        project: ProjectId,
        from: &BranchName,
        to: &BranchName,
    ) {
        let changed: Vec<WorktreeInfo> = {
            let mut state = self.state();
            state
                .worktrees
                .values_mut()
                .filter(|entry| entry.info.project == project)
                .filter_map(|entry| {
                    let mut changed = false;
                    if entry.info.branch.as_ref() == Some(from) {
                        entry.info.branch = Some(to.clone());
                        changed = true;
                    }
                    if entry.info.created_branch.as_ref() == Some(from) {
                        entry.info.created_branch = Some(to.clone());
                        changed = true;
                    }
                    changed.then(|| entry.info.clone())
                })
                .collect()
        };
        for info in changed {
            if let Err(error) = self.save_worktree(info).await {
                tracing::warn!(%error, "could not record a renamed branch");
            }
        }
    }

    /// A worktree's branch after `git.switch` in it.
    pub(crate) async fn branch_switched(&self, worktree: WorktreeId, now: Option<BranchName>) {
        let changed = {
            let mut state = self.state();
            state.worktrees.get_mut(&worktree).and_then(|entry| {
                (entry.info.branch != now).then(|| {
                    entry.info.branch = now;
                    entry.info.clone()
                })
            })
        };
        if let Some(info) = changed
            && let Err(error) = self.save_worktree(info).await
        {
            tracing::warn!(%error, "could not record a switched branch");
        }
    }
}

/// The branch a new worktree checks out, where it starts, the branch the
/// node creates for it, and the snapshot to put back in it.
struct Chosen {
    branch: NewBranch,
    start: WorktreeStart,
    created: Option<BranchName>,
    restore: Option<vessel_proto::workspace::CommitId>,
}

/// Resolve what `worktree.create` asks for; a refusal when it names
/// nothing the repository has.
async fn choose(
    git: &vessel_workspace::Git,
    root: &Path,
    default_branch: Option<vessel_proto::workspace::RefName>,
    asked: WorktreeBranch,
    id: WorktreeId,
) -> Result<Result<Chosen, WorktreeRefusal>, WorkspaceError> {
    Ok(Ok(match asked {
        WorktreeBranch::New { name, base } => {
            let explicit = base.is_some();
            let base = base.or_else(|| default_branch.map(|name| WorktreeBase::Ref { name }));
            let commit = match &base {
                Some(base) => git.resolve(root, base).await?,
                None => git.head(root).await?,
            };
            let Some(commit) = commit else {
                return Ok(Err(if explicit {
                    WorktreeRefusal::NoSuchBase
                } else {
                    WorktreeRefusal::NoCommit
                }));
            };
            let name = match name {
                Some(name) => name,
                None => branch_for(id)?,
            };
            Chosen {
                start: WorktreeStart {
                    commit: commit.clone(),
                    name: match base {
                        Some(WorktreeBase::Ref { name }) => Some(name),
                        Some(WorktreeBase::Commit { .. }) | None => None,
                    },
                },
                branch: NewBranch::Create {
                    name: name.clone(),
                    start: commit,
                },
                created: Some(name),
                restore: None,
            }
        }
        WorktreeBranch::Existing { name } => {
            let named = WorktreeBase::Ref {
                name: name.as_ref_name(),
            };
            let Some(commit) = git.resolve(root, &named).await? else {
                return Ok(Err(WorktreeRefusal::NoSuchBranch));
            };
            Chosen {
                start: WorktreeStart {
                    commit,
                    name: Some(name.as_ref_name()),
                },
                branch: NewBranch::Existing { name },
                created: None,
                restore: None,
            }
        }
        WorktreeBranch::Snapshot { worktree, name } => {
            let snapshots = git.snapshots(root).await?;
            let Some(snapshot) = snapshots
                .snapshots
                .into_iter()
                .find(|snapshot| snapshot.worktree == worktree)
            else {
                return Ok(Err(WorktreeRefusal::NoSuchSnapshot));
            };
            let name = match name {
                Some(name) => name,
                None => branch_for(id)?,
            };
            Chosen {
                start: WorktreeStart {
                    commit: snapshot.head.clone(),
                    name: None,
                },
                branch: NewBranch::Create {
                    name: name.clone(),
                    start: snapshot.head,
                },
                created: Some(name),
                restore: Some(snapshot.commit),
            }
        }
    }))
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
        (SetupChoice::Skip, ProjectCommands::Listed { setup, .. }) if !setup.is_empty() => {
            Planned::Skipped
        }
        (SetupChoice::Skip, _) | (SetupChoice::Run, ProjectCommands::None) => Planned::Nothing,
        (SetupChoice::Run, ProjectCommands::Listed { setup, .. }) if setup.is_empty() => {
            Planned::Nothing
        }
        (
            SetupChoice::Run,
            ProjectCommands::Listed {
                approved: true,
                setup,
                ..
            },
        ) => Planned::Run(setup.clone()),
        (SetupChoice::Run, ProjectCommands::Listed { .. } | ProjectCommands::Invalid { .. }) => {
            return None;
        }
    })
}

/// Whether one more worktree would pass the node's bound, or an agent's.
fn over_limit(
    worktrees: &std::collections::HashMap<WorktreeId, WorktreeEntry>,
    creator: &Creator,
) -> bool {
    if worktrees.len() >= MAX_WORKTREES {
        return true;
    }
    matches!(creator, Creator::Agent { .. })
        && worktrees
            .values()
            .filter(|entry| entry.info.created_by == *creator)
            .count()
            >= MAX_PER_AGENT
}

/// What setup a new worktree gets.
enum Planned {
    Nothing,
    Skipped,
    Run(Vec<vessel_proto::workspace::ProjectCommand>),
}

/// A worktree's own sessions: the live ones whose folder is in it.
fn with_sessions(mut info: WorktreeInfo, live: &Live) -> WorktreeInfo {
    info.sessions = live
        .iter()
        .filter(|(_, cwd)| cwd.starts_with(&info.path))
        .map(|(id, _)| *id)
        .collect();
    info
}

/// The branch a new worktree gets when none is named.
fn branch_for(worktree: WorktreeId) -> Result<BranchName, WorkspaceError> {
    format!("vessel/{}", short(worktree))
        .parse()
        .map_err(|_never| WorkspaceError::Invalid("no branch name for the worktree".to_owned()))
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
