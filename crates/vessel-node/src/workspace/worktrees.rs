//! Worktrees: the node's record of them, read back when the node starts
//! and listed with the live sessions in each (protocol 10, ADR 0027).
//! Making one with its setup (`create`, from what `start` resolves),
//! removing one without losing work and tidying a session's (`removal`),
//! snapshots (`snapshots`), where a session runs (`place`) and following
//! a branch's moves (`branches`) are the submodules'. Nothing here runs
//! git or a project's commands.

mod branches;
mod create;
mod place;
mod removal;
mod snapshots;
mod start;

use std::sync::Arc;

use chrono::Utc;
use vessel_proto::workspace::{SetupFailure, SetupState, WorktreeInfo, WorktreeList};
use vessel_proto::{ProjectId, WorktreeId};

use super::projects::Live;
use super::setup::note;
use super::{MAX_WORKTREES, WorkspaceError, Workspaces, WorktreeEntry};

pub(crate) use place::Placed;
pub(crate) use removal::TidyScope;

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
