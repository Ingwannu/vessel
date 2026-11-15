//! Removing a worktree without losing work: a person's `worktree.remove`,
//! and tidying the worktree made for a session (`retire`), with the
//! sessions tidying may still look at (`tidy_candidates`). It decides
//! nothing about when to tidy (`crate::tidying` does), and never removes
//! a worktree a live session or a live terminal is in.
//!
//! [Decision Log]
//! - Removal follows one rule for a person's `worktree.remove` and for
//!   tidying (`retire`): inspect, teardown (when approved), inspect again,
//!   then git's own non-forced removal. Only `discard`, or a snapshot kept
//!   first, forces it. A worktree a live session is in, or whose setup or
//!   teardown runs, is kept (`in_use`).
//! - Removing a worktree runs as a task of the node's, awaited by the
//!   call: a caller that goes away (a cancelled call, a closed page)
//!   leaves no worktree claimed by a removal that stopped halfway.
//! - What runs in a worktree is read after the claim, never before (ADR
//!   0030), and a session, a terminal or a script's run starting there
//!   holds the worktree by the "starting" count until it is live or
//!   registered (`place`, `start_in`, `hold_folder`); a start checks the
//!   claim after taking the count. So, whatever the order, the removal
//!   sees the start (`kept { in_use }`) or the start sees the claim and is
//!   refused before anything runs: teardown never runs under it. (Track C
//!   checked after its start instead, which left a window: a reading
//!   before the start's reservation and a claim after its last check. The
//!   join of wave 2's first half closed it with B2's count; the e2e test
//!   `meeting` races the two.)
//! - Tidying is `retire`, called when the session a worktree was made for
//!   is deleted or dropped by retention, never when it only ends or is
//!   archived (`crate::tidying` says when, and why). A worktree it keeps
//!   says why in `kept`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use vessel_proto::conversation::TidiedWorktree;
use vessel_proto::workspace::{
    IfLost, KeptReason, ProjectCommands, RemoveWorktreeOutcome, Snapshot, WorktreeInfo,
};
use vessel_proto::{SessionId, WorktreeId};

use crate::workspace::projects::Occupant;
use crate::workspace::setup::Kind;
use crate::workspace::{WorkspaceError, Workspaces};

/// Which worktrees made for sessions a tidying looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TidyScope {
    /// Every one: the node's start, and after retention dropped sessions.
    Every,
    /// Only those kept because a session was in them: after a deletion,
    /// which may have let one go.
    Kept,
}

/// How a removal ended, inside this module.
enum Removal {
    Done(Option<Snapshot>),
    Kept(Vec<KeptReason>),
}

impl Workspaces {
    /// `worktree.remove`: by the rule, `if_lost` saying what to do with work
    /// that would be lost.
    ///
    /// # Errors
    /// `not_found`; `unsupported` without git; git's failures.
    pub(crate) async fn remove_worktree(
        self: &Arc<Self>,
        worktree: WorktreeId,
        if_lost: IfLost,
        live: impl FnOnce() -> Vec<(Occupant, PathBuf)>,
    ) -> Result<RemoveWorktreeOutcome, WorkspaceError> {
        Ok(match self.remove(worktree, if_lost, live).await? {
            Removal::Done(snapshot) => RemoveWorktreeOutcome::Removed { snapshot },
            Removal::Kept(reasons) => RemoveWorktreeOutcome::Kept { reasons },
        })
    }

    /// Tidy the worktree made for `session` (now deleted or dropped by
    /// retention), unless another live session is in it or something would
    /// be lost; a kept one records why. `None` when the session had no
    /// worktree of its own.
    pub(crate) async fn retire(
        self: &Arc<Self>,
        session: SessionId,
        live: impl FnOnce() -> Vec<(Occupant, PathBuf)>,
    ) -> Option<TidiedWorktree> {
        let worktree = self
            .state()
            .worktrees
            .values()
            .find(|entry| entry.info.created_for == Some(session))
            .map(|entry| entry.info.id)?;
        let others = move || {
            let mut live = live();
            live.retain(|(occupant, _)| *occupant != Occupant::Session(session));
            live
        };
        let outcome = match self.remove(worktree, IfLost::Keep, others).await {
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
        Some(TidiedWorktree { worktree, outcome })
    }

    /// The sessions whose worktrees tidying may still remove, among those
    /// `scope` names; never one tidying kept for the work it holds, which
    /// waits for a person (`crate::tidying`), nor one git could not
    /// inspect: trying again would run its teardown again at every prune
    /// and every deletion, for git to refuse again.
    pub(crate) fn tidy_candidates(&self, scope: TidyScope) -> Vec<SessionId> {
        self.state()
            .worktrees
            .values()
            .filter(|entry| match entry.info.kept {
                None => scope == TidyScope::Every,
                Some(KeptReason::InUse) => true,
                Some(
                    KeptReason::InspectionFailed
                    | KeptReason::Uncommitted { .. }
                    | KeptReason::UniqueCommits { .. }
                    | KeptReason::Operation { .. }
                    | KeptReason::Submodules
                    | KeptReason::FlaggedFiles
                    | KeptReason::WorktreeRefs
                    | KeptReason::Unknown,
                ) => false,
            })
            .filter_map(|entry| entry.info.created_for)
            .collect()
    }

    /// Claim the worktree, remove it by the rule, release it when kept.
    /// The removal runs as a task of the node's: a caller that goes away
    /// mid-teardown neither stops it halfway nor leaves the worktree
    /// claimed.
    ///
    /// What runs where (`live`: live sessions, live terminals and the
    /// terminals starting) is read after the claim, not before: one that
    /// began between a reading and the claim would be seen by neither
    /// (ADR 0030). Once claimed, no `place`, start again, terminal or
    /// script gets in (each checks the claim after it counts itself, so
    /// it is either read here or refused there; ADR 0031); a session that
    /// passed its check before is `starting`.
    async fn remove(
        self: &Arc<Self>,
        worktree: WorktreeId,
        if_lost: IfLost,
        live: impl FnOnce() -> Vec<(Occupant, PathBuf)>,
    ) -> Result<Removal, WorkspaceError> {
        let git = self.git()?.clone();
        let project = self
            .state()
            .worktrees
            .get(&worktree)
            .map(|entry| entry.info.project)
            .ok_or(WorkspaceError::UnknownWorktree(worktree))?;
        // Serialize against provisioning before checking ownership. The task
        // retains this exact bounded hold through teardown, git and Store ack.
        let admission = self.admit_project(project).await?;
        self.require_no_registry_reference(project).await?;
        let claim = self.stop.child_token();
        let (info, root, written) = {
            let mut state = self.state();
            let root = {
                let entry = state
                    .worktrees
                    .get(&worktree)
                    .ok_or(WorkspaceError::UnknownWorktree(worktree))?;
                if entry.info.project != project {
                    return Err(WorkspaceError::UnknownWorktree(worktree));
                }
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
            // A session that passed its place's check and is not live yet
            // counts as one in it (ADR 0030).
            if entry.running.is_some() || entry.starting > 0 {
                return Ok(Removal::Kept(vec![KeptReason::InUse]));
            }
            entry.running = Some(claim.clone());
            (entry.info.clone(), root, Arc::clone(&entry.written))
        };
        let in_use = live().iter().any(|(_, cwd)| cwd.starts_with(&info.path));
        if in_use {
            if let Some(entry) = self.state().worktrees.get_mut(&worktree) {
                entry.running = None;
            }
            return Ok(Removal::Kept(vec![KeptReason::InUse]));
        }
        let workspaces = Arc::clone(self);
        self.tasks
            .spawn(async move {
                let _admission = admission;
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
        // git inspects the worktree with its repository's configuration:
        // not when its `.git` names another (`gitfile::registered`). A
        // forced removal runs git in the project only.
        let checked = (path.clone(), root.to_path_buf());
        let registered = tokio::task::spawn_blocking(move || {
            vessel_workspace::git::gitfile::registered(&checked.0, &checked.1)
        })
        .await
        .unwrap_or(false);
        if !registered && if_lost != IfLost::Discard {
            tracing::warn!(worktree = %info.id, "a worktree's .git names another repository; kept");
            return Ok(Removal::Kept(vec![KeptReason::InspectionFailed]));
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
}
