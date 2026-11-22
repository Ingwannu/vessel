//! Reconcile exact run-owned effects and retire only after releasing their pin.

use std::path::PathBuf;

use chrono::Utc;
use vessel_proto::automation::{AfterRun, RunArchive};
use vessel_proto::registry::{WorkspaceBacking, WorkspaceLifecycle, WorkspaceResource};
use vessel_proto::SessionId;
use vessel_session::IdleStop;
use vessel_store::{
    ArchiveDefer, ProvisionRetirement, RunBindingStage, ScheduleRunBinding, WorkspaceArchivePurpose,
};

use super::NodeService;
use crate::error::ServiceError;
use crate::workspace::{
    ArchiveResult, Occupant, ReconciledProvision, WorkspaceActors, WorkspaceError,
};

/// A pin can be released only after archiving or a proven lack of effects.
pub(super) enum IsolationCleanup {
    Settled {
        archive: Option<RunArchive>,
        retirement: Option<ProvisionRetirement>,
    },
    Deferred(ArchiveDefer),
}

impl IsolationCleanup {
    pub(super) fn disposition(&self) -> Option<RunArchive> {
        match self {
            Self::Settled { archive, .. } => *archive,
            Self::Deferred(ArchiveDefer::BusySession) => Some(RunArchive::DeferredBusy),
            Self::Deferred(ArchiveDefer::OtherMembers) => Some(RunArchive::DeferredOtherMembers),
        }
    }
}

impl NodeService {
    /// Clean up a failed start even if its original policy was Keep.
    pub(super) async fn cleanup_isolation(
        &self,
        binding: ScheduleRunBinding,
    ) -> Result<(), WorkspaceError> {
        let result = self.dispose_isolation(binding).await?;
        self.settle_isolation(binding, result).await
    }

    pub(super) async fn dispose_isolation(
        &self,
        binding: ScheduleRunBinding,
    ) -> Result<IsolationCleanup, WorkspaceError> {
        if binding.stage == RunBindingStage::Started && binding.after == AfterRun::Keep {
            return Ok(IsolationCleanup::Settled { archive: None, retirement: None });
        }
        let workspace = match self.workspaces.reconcile_workspace_provision(binding.workspace).await? {
            ReconciledProvision::Published(workspace) => workspace,
            ReconciledProvision::Absent { reserved } => {
                return Ok(IsolationCleanup::Settled {
                    archive: None,
                    retirement: reserved.then_some(ProvisionRetirement::Unpublished),
                });
            }
        };
        let stop = Some(binding.session);
        let result = match workspace.lifecycle {
            WorkspaceLifecycle::Active => {
                self.workspaces.archive_workspace(
                    workspace.id,
                    workspace.revision,
                    WorkspaceArchivePurpose::Automatic,
                    stop,
                    self,
                ).await?
            }
            WorkspaceLifecycle::Archiving { .. } => {
                let record = self.sessions.store().registry_archive(workspace.id).await?
                    .ok_or(vessel_store::StoreError::RegistryConflict)?;
                self.workspaces.resume_workspace_archive(record.claim, stop, self).await?
            }
            WorkspaceLifecycle::Archived { .. } => {
                // Before retirement, an absent durable Worktree and absent path
                // verify the earlier removal reached its recorded boundary.
                let clean = match workspace.backing {
                    WorkspaceBacking::Local => true,
                    WorkspaceBacking::Managed { worktree } => {
                        !self.sessions.store().workspace().await?.worktrees.iter().any(|row| row.id == worktree)
                            && tokio::fs::try_exists(&workspace.path).await.is_ok_and(|exists| !exists)
                    }
                };
                return Ok(archived(clean));
            }
            WorkspaceLifecycle::Restoring { .. } => return Err(vessel_store::StoreError::RegistryConflict.into()),
        };
        Ok(match result {
            ArchiveResult::Archived { kept } => archived(kept.is_empty()),
            ArchiveResult::Deferred(reason) => IsolationCleanup::Deferred(reason),
        })
    }

    pub(super) async fn settle_isolation(
        &self,
        binding: ScheduleRunBinding,
        result: IsolationCleanup,
    ) -> Result<(), WorkspaceError> {
        let store = self.sessions.store();
        match result {
            IsolationCleanup::Settled { retirement, .. } => {
                store.finish_schedule_run_binding(binding).await?;
                if let Some(evidence) = retirement {
                    store.retire_registry_workspace(binding.workspace, evidence, Utc::now()).await?;
                }
            }
            IsolationCleanup::Deferred(reason) => {
                let stage = match reason {
                    ArchiveDefer::BusySession => RunBindingStage::DeferredBusy,
                    ArchiveDefer::OtherMembers => RunBindingStage::DeferredMembers,
                };
                if binding.stage != stage {
                    store.advance_schedule_run_binding(binding, stage).await?;
                }
            }
        }
        Ok(())
    }
}

fn archived(clean: bool) -> IsolationCleanup {
    IsolationCleanup::Settled {
        archive: Some(if clean { RunArchive::Archived } else { RunArchive::BackingKept }),
        retirement: clean.then_some(ProvisionRetirement::CleanArchive),
    }
}

impl WorkspaceActors for NodeService {
    fn live(&self, resource: WorkspaceResource) -> bool {
        match resource {
            WorkspaceResource::Session { session } => self.sessions.live(session).is_some(),
            WorkspaceResource::Terminal { terminal } => self.terminals.get(terminal)
                .is_ok_and(|entry| entry.handle.info().is_live()),
        }
    }

    fn occupied_folders(&self) -> Vec<(Occupant, PathBuf)> {
        let mut occupied = self.sessions.live_folders();
        occupied.extend(self.terminals.live_folders());
        occupied
    }

    async fn stop_idle(&self, session: SessionId) -> Result<IdleStop, WorkspaceError> {
        match self.sessions.archive_idle(session).await {
            Ok(result) => {
                if result == IdleStop::Stopped {
                    self.unwatch_session(session);
                }
                Ok(result)
            }
            Err(ServiceError::NotFound(_)) => Ok(IdleStop::Stopped),
            Err(error) => {
                tracing::warn!(%session, %error, "the isolated actor's stop was not confirmed");
                Err(WorkspaceError::Conflict("the workspace's session stop is unconfirmed".to_owned()))
            }
        }
    }

    async fn stop_member(&self, resource: WorkspaceResource) -> Result<(), WorkspaceError> {
        match resource {
            WorkspaceResource::Session { session } => match self.put_away(session).await {
                Ok(_) | Err(ServiceError::NotFound(_)) => {}
                Err(error) => {
                    tracing::warn!(%session, %error, "cannot archive an exact workspace session");
                    return Err(WorkspaceError::Conflict("the workspace's session stop is unconfirmed".to_owned()));
                }
            },
            WorkspaceResource::Terminal { terminal } => {
                if let Ok(entry) = self.terminals.get(terminal) {
                    entry.handle.kill();
                    tokio::time::timeout(crate::terminals::CLOSE_WAIT, entry.handle.ended()).await
                        .map_err(|_| WorkspaceError::Conflict("the workspace's terminal stop is unconfirmed".to_owned()))?;
                }
            }
        }
        Ok(())
    }
}
