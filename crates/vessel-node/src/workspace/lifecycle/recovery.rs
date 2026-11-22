//! Recovery consumes retained identities and exact receipts. It never
//! replays filesystem creation or assumes absence from a reused cwd.

use std::sync::Arc;

use chrono::Utc;
use vessel_proto::WorkspaceId;
use vessel_proto::registry::{
    RegistryRevision, WorkspaceBacking, WorkspaceDescriptor, WorkspaceLifecycle, WorkspacePin,
};
use vessel_store::{
    RegistryRecord, RegistryReservation, RegistryResourceReadiness, RegistryStartState, ReservationStage, StoreError,
    WorkspaceArchivePurpose,
};

use super::WorkspaceActors;
use crate::workspace::{WorkspaceError, Workspaces};

/// What the next caller can safely do with the existing provision.
#[derive(Debug)]
pub(crate) enum ReconciledProvision {
    Published(WorkspaceDescriptor),
    /// Nothing was made. A reservation, if any, can now retire.
    Absent {
        reserved: bool,
    },
}

impl Workspaces {
    pub(crate) async fn reconcile_workspace_provision(
        self: &Arc<Self>,
        id: WorkspaceId,
    ) -> Result<ReconciledProvision, WorkspaceError> {
        if let Some(record) = self.store.registry_workspace(id).await? {
            return Ok(ReconciledProvision::Published(record.descriptor));
        }
        let Some(reserved) = self.store.registry_reservation(id).await? else {
            return Ok(ReconciledProvision::Absent { reserved: false });
        };
        match reserved.stage {
            ReservationStage::Retired => {
                return Ok(ReconciledProvision::Absent { reserved: false });
            }
            ReservationStage::Ready => return Err(StoreError::RegistryMissing(id).into()),
            ReservationStage::Reserved => {
                return Ok(ReconciledProvision::Absent { reserved: true });
            }
            ReservationStage::Creating | ReservationStage::Uncertain => {}
        }
        let Some(worktree) = reserved.worktree else {
            // Local uses the admitted source directory. Publishing the
            // descriptor is the only effect of creating this workspace.
            return Ok(ReconciledProvision::Absent { reserved: true });
        };
        // A stored backing receipt is written only after Git has made
        // the exact worktree. A missing receipt cannot prove Git did
        // nothing, so that reservation still holds its capacity.
        let admission = self.admit_project(reserved.intent.project).await?;
        let repaired = self.recover_workspace_backing(reserved, worktree).await;
        drop(admission);
        repaired.map(ReconciledProvision::Published)
    }

    async fn recover_workspace_backing(
        &self,
        reserved: RegistryReservation,
        worktree: vessel_proto::WorktreeId,
    ) -> Result<WorkspaceDescriptor, WorkspaceError> {
        let id = reserved.workspace;
        let recovery = self
            .store
            .workspace()
            .await?
            .worktrees
            .into_iter()
            .find(|row| {
                row.id == worktree
                    && row.project == reserved.intent.project
                    && row.created_for.is_none()
            })
            .ok_or_else(|| {
                WorkspaceError::Conflict(
                    "workspace backing could not be confirmed; its reservation is retained"
                        .to_owned(),
                )
            })?;
        if reserved.stage == ReservationStage::Uncertain {
            self.store
                .set_registry_reservation_stage(
                    id,
                    ReservationStage::Uncertain,
                    ReservationStage::Creating,
                )
                .await?;
        }
        let descriptor = WorkspaceDescriptor {
            id,
            project: reserved.intent.project,
            path: recovery.path.clone(),
            title: reserved.intent.title,
            automation: reserved.intent.automation,
            pin: WorkspacePin::Unpinned,
            backing: WorkspaceBacking::Managed { worktree },
            lifecycle: WorkspaceLifecycle::Active,
            visibility: reserved.intent.visibility,
            created_at: recovery.created_at.min(Utc::now()),
            revision: RegistryRevision::ZERO,
        };
        self.store
            .complete_registry_workspace(RegistryRecord {
                descriptor: descriptor.clone(),
                recovery: Some(recovery),
            })
            .await?;
        Ok(descriptor)
    }
}

impl Workspaces {
    /// A crash can occur after a caller released its pin but before the
    /// separate retirement write. Revisit archived descriptors on opening.
    pub(crate) async fn retire_reconciled_archives(&self) {
        let Ok(registry) = self.store.workspace_registry().await else {
            return;
        };
        for descriptor in registry.workspaces {
            if !matches!(descriptor.lifecycle, WorkspaceLifecycle::Archived { .. })
                || !self
                    .store
                    .registry_reservation(descriptor.id)
                    .await
                    .is_ok_and(|row| row.is_some_and(|row| row.stage == ReservationStage::Ready))
                || !self.archived_backing_gone(&descriptor).await
            {
                continue;
            }
            if let Err(error) = self
                .store
                .retire_registry_workspace(
                    descriptor.id,
                    vessel_store::ProvisionRetirement::CleanArchive,
                    Utc::now(),
                )
                .await
            {
                tracing::debug!(workspace = %descriptor.id, %error, "archived workspace still has a retained obligation");
            }
        }
    }

    pub(crate) async fn archived_backing_gone(&self, workspace: &WorkspaceDescriptor) -> bool {
        match workspace.backing {
            WorkspaceBacking::Local => true,
            WorkspaceBacking::Managed { worktree } => {
                self.store
                    .workspace()
                    .await
                    .is_ok_and(|stored| !stored.worktrees.iter().any(|row| row.id == worktree))
                    && tokio::fs::try_exists(&workspace.path)
                        .await
                        .is_ok_and(|exists| !exists)
            }
        }
    }
}

impl Workspaces {
    /// A new runtime's empty actor table cannot prove that a previous
    /// process group exited. Require an exact join receipt before removing
    /// backing. Unconfirmed work retains its lease and workspace slot.
    pub(crate) async fn reconcile_workspace_starts(
        &self,
        workspace: WorkspaceId,
        actors: &impl WorkspaceActors,
    ) -> Result<(), WorkspaceError> {
        for member in self.store.registry_members(workspace).await? {
            let run = self.store.registry_resource_run(member.resource).await?;
            if !actors.live(member.resource)
                && !run.is_some_and(|run| run.joined || run.readiness == RegistryResourceReadiness::Failed)
                && !actors.joined(member.resource).await?
            {
                return Err(WorkspaceError::Conflict(
                    "a previous workspace actor's stop is unconfirmed".to_owned(),
                ));
            }
        }
        for start in self.store.pending_registry_starts(workspace).await? {
            if start.state == RegistryStartState::Uncertain {
                if actors.live(start.lease.resource()) {
                    self.store.complete_registry_start(start.lease).await?;
                } else {
                    self.store.fail_registry_start(start.lease).await?;
                }
            }
        }
        Ok(())
    }

    /// Explicit archive claims have the same recovery even if they have no
    /// schedule binding. The corresponding effect is live only after boot.
    pub(crate) async fn resume_explicit_archives(self: &Arc<Self>, actors: &impl WorkspaceActors) {
        let Ok(registry) = self.store.workspace_registry().await else {
            return;
        };
        for workspace in registry.workspaces {
            if !matches!(workspace.lifecycle, WorkspaceLifecycle::Archiving { .. }) {
                continue;
            }
            let replayed = async {
                let Some(known) = self.store.registry_archive(workspace.id).await? else {
                    return Ok(());
                };
                if known.claim.purpose == WorkspaceArchivePurpose::Explicit {
                    self.reconcile_workspace_starts(workspace.id, actors)
                        .await?;
                    let _archived = self
                        .resume_workspace_archive(known.claim, None, actors)
                        .await?;
                }
                Ok::<(), WorkspaceError>(())
            }
            .await;
            if let Err(error) = replayed {
                tracing::warn!(workspace = %workspace.id, %error, "an explicit workspace archive remains unsettled");
            }
        }
    }
}
