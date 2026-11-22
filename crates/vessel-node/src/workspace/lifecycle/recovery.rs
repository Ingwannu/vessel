//! Recovery consumes retained identities and exact receipts. It never
//! replays filesystem creation or assumes absence from a reused cwd.

use std::sync::Arc;

use chrono::Utc;
use vessel_proto::WorkspaceId;
use vessel_proto::registry::{
    RegistryRevision, WorkspaceBacking, WorkspaceDescriptor, WorkspaceLifecycle, WorkspacePin,
};
use vessel_store::{RegistryRecord, RegistryReservation, ReservationStage, StoreError};

use crate::workspace::{WorkspaceError, Workspaces};

/// What the next caller can safely do with the existing provision.
#[derive(Debug)]
pub(crate) enum ReconciledProvision {
    Published(WorkspaceDescriptor),
    /// Nothing was made. A reservation, if any, can now retire.
    Absent { reserved: bool },
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
            ReservationStage::Retired => return Ok(ReconciledProvision::Absent { reserved: false }),
            ReservationStage::Ready => return Err(StoreError::RegistryMissing(id).into()),
            ReservationStage::Reserved => return Ok(ReconciledProvision::Absent { reserved: true }),
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
            .find(|row| row.id == worktree && row.project == reserved.intent.project && row.created_for.is_none())
            .ok_or_else(|| WorkspaceError::Conflict(
                "workspace backing could not be confirmed; its reservation is retained".to_owned()
            ))?;
        if reserved.stage == ReservationStage::Uncertain {
            self.store.set_registry_reservation_stage(
                id, ReservationStage::Uncertain, ReservationStage::Creating
            ).await?;
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
        self.store.complete_registry_workspace(RegistryRecord {
            descriptor: descriptor.clone(),
            recovery: Some(recovery),
        }).await?;
        Ok(descriptor)
    }
}
