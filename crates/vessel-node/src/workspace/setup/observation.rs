//! Observation/cancellation use worktree and invocation identities, never cwd.

use std::time::Duration;

use vessel_proto::workspace::{
    CancelSetup, CancelSetupOutcome, ReadSetupProgress, SetupProgressReply, SetupRunState,
};
use vessel_proto::{SetupRunId, WorktreeId};
use vessel_workspace::commands::CommandEvent;

use super::super::{MAX_WAIT_SECS, WorkspaceError, Workspaces};

impl Workspaces {
    pub(super) fn observe_setup(
        &self,
        worktree: WorktreeId,
        generation: SetupRunId,
        event: CommandEvent<'_>,
    ) {
        let mut state = self.state();
        if let Some(entry) = state.worktrees.get_mut(&worktree)
            && let Some(observation) = &mut entry.observation
            && observation.snapshot.cursor.run == generation
        {
            observation.event(event);
            entry.written.send_modify(|_| {});
        }
    }

    /// Absent snapshots stay absent across restart; legacy text is not evidence
    /// of per-command boundaries, durations or successful completion.
    pub(crate) async fn setup_progress(
        &self,
        request: ReadSetupProgress,
    ) -> Result<SetupProgressReply, WorkspaceError> {
        let (mut changed, wait) = {
            let state = self.state();
            let entry = state
                .worktrees
                .get(&request.worktree)
                .ok_or(WorkspaceError::UnknownWorktree(request.worktree))?;
            let wait = request.after.is_some_and(|after| {
                entry.observation.as_ref().is_some_and(|observation| {
                    observation.snapshot.cursor == after
                        && observation.snapshot.state == SetupRunState::Running
                })
            });
            (entry.written.subscribe(), wait)
        };
        if let Some(seconds) = request.wait.filter(|_| wait) {
            let limit = Duration::from_secs(seconds.min(MAX_WAIT_SECS).into());
            let _changed = tokio::time::timeout(limit, changed.changed()).await;
        }
        let state = self.state();
        let entry = state
            .worktrees
            .get(&request.worktree)
            .ok_or(WorkspaceError::UnknownWorktree(request.worktree))?;
        Ok(SetupProgressReply {
            progress: entry
                .observation
                .as_ref()
                .map(|observation| observation.snapshot.clone()),
        })
    }

    /// A receipt means signalled, never reaped. Only a terminal structured
    /// snapshot establishes cancellation completion. Repeated requests do not
    /// create processes, and a stale run cannot signal a newer setup/teardown.
    pub(crate) fn cancel_setup(
        &self,
        request: CancelSetup,
    ) -> Result<CancelSetupOutcome, WorkspaceError> {
        let state = self.state();
        let entry = state
            .worktrees
            .get(&request.worktree)
            .ok_or(WorkspaceError::UnknownWorktree(request.worktree))?;
        let Some(observation) = &entry.observation else {
            return Ok(CancelSetupOutcome::Stale);
        };
        if observation.snapshot.cursor.run != request.run {
            return Ok(CancelSetupOutcome::Stale);
        }
        if observation.snapshot.state != SetupRunState::Running {
            return Ok(CancelSetupOutcome::NotRunning);
        }
        let Some(cancel) = &entry.running else {
            return Ok(CancelSetupOutcome::NotRunning);
        };
        cancel.cancel();
        Ok(CancelSetupOutcome::Requested)
    }
}
