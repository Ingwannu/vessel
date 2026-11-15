//! Persist backing ids before existing worktree creation and finalize them atomically.

use super::{Provisioned, RegistryError};
use crate::workspace::Workspaces;
use chrono::Utc;
use std::sync::Arc;
use vessel_proto::registry::{
    RegistryRevision, WorkspaceBacking, WorkspaceDescriptor, WorkspaceIsolation, WorkspaceLifecycle,
};
use vessel_proto::workspace::{CreateWorktree, CreateWorktreeOutcome, WorkspaceRoot};
use vessel_proto::{Creator, WorkspaceId, WorktreeId};
use vessel_store::{
    RegistryRecord, RegistryReservation, ReservationStage, StoreError, WorkspaceProvision,
};

enum Made {
    Ready(Box<RegistryRecord>),
    Approval(vessel_proto::workspace::ProjectCommands),
    Refused(vessel_proto::workspace::WorktreeRefusal),
}

impl Workspaces {
    /// Empty workspace provisioning is independent of any initial agent's eventual outcome.
    pub(crate) async fn provision_workspace(
        self: &Arc<Self>,
        creator: Creator,
        id: WorkspaceId,
        intent: WorkspaceProvision,
    ) -> Result<Provisioned, RegistryError> {
        let owner = Arc::clone(self);
        self.tasks
            .spawn(async move { owner.provision(creator, id, intent).await })
            .await
            .map_err(RegistryError::Task)?
    }

    async fn provision(
        self: &Arc<Self>,
        creator: Creator,
        id: WorkspaceId,
        intent: WorkspaceProvision,
    ) -> Result<Provisioned, RegistryError> {
        // Project authority/root replacement checks precede every reservation or spawn.
        let root = self
            .resolve(WorkspaceRoot::Project {
                project: intent.project,
            })
            .await?;
        let reservation = self.registry_reserve(id, intent).await?;
        if let Some(record) = self.store.registry_workspace(id).await? {
            return Ok(Provisioned::Ready(record.descriptor));
        }
        match reservation.stage {
            ReservationStage::Reserved => {}
            ReservationStage::Creating => return Ok(Provisioned::InProgress(id)),
            ReservationStage::Ready | ReservationStage::Uncertain => {
                return Ok(Provisioned::Uncertain(id));
            }
        }
        match self
            .store
            .set_registry_reservation_stage(
                id,
                ReservationStage::Reserved,
                ReservationStage::Creating,
            )
            .await
        {
            Ok(()) => {}
            Err(StoreError::RegistryConflict) => return Ok(Provisioned::InProgress(id)),
            Err(error) => return Err(error.into()),
        }
        let made = self
            .registry_make(creator, &reservation, root.root.path())
            .await;
        match made {
            Ok(Made::Ready(record)) => {
                let descriptor = record.descriptor.clone();
                self.store.complete_registry_workspace(*record).await?;
                Ok(Provisioned::Ready(descriptor))
            }
            Ok(Made::Approval(commands)) => {
                self.store
                    .set_registry_reservation_stage(
                        id,
                        ReservationStage::Creating,
                        ReservationStage::Reserved,
                    )
                    .await?;
                Ok(Provisioned::NeedsApproval(commands))
            }
            Ok(Made::Refused(reason)) => {
                self.store
                    .set_registry_reservation_stage(
                        id,
                        ReservationStage::Creating,
                        ReservationStage::Uncertain,
                    )
                    .await?;
                Ok(Provisioned::Refused(reason))
            }
            Err(error) => {
                self.store
                    .set_registry_reservation_stage(
                        id,
                        ReservationStage::Creating,
                        ReservationStage::Uncertain,
                    )
                    .await?;
                Err(error)
            }
        }
    }

    async fn registry_reserve(
        &self,
        id: WorkspaceId,
        intent: WorkspaceProvision,
    ) -> Result<RegistryReservation, RegistryError> {
        if let Some(found) = self.store.registry_reservation(id).await? {
            return if found.intent == intent {
                Ok(found)
            } else {
                Err(RegistryError::IntentConflict)
            };
        }
        let worktree = match intent.isolation {
            WorkspaceIsolation::Local => None,
            WorkspaceIsolation::Worktree { .. } => Some(WorktreeId::random()),
        };
        let proposed = RegistryReservation {
            workspace: id,
            intent,
            worktree,
            stage: ReservationStage::Reserved,
        };
        match self
            .store
            .reserve_registry_workspace(proposed.clone())
            .await
        {
            Ok(()) => Ok(proposed),
            Err(StoreError::RegistryConflict) => {
                let found = self
                    .store
                    .registry_reservation(id)
                    .await?
                    .ok_or(RegistryError::IntentConflict)?;
                if found.intent == proposed.intent {
                    Ok(found)
                } else {
                    Err(RegistryError::IntentConflict)
                }
            }
            Err(error) => Err(error.into()),
        }
    }

    async fn registry_make(
        self: &Arc<Self>,
        creator: Creator,
        reservation: &RegistryReservation,
        path: &std::path::Path,
    ) -> Result<Made, RegistryError> {
        let (path, backing, recovery) = match &reservation.intent.isolation {
            WorkspaceIsolation::Local => (path.to_path_buf(), WorkspaceBacking::Local, None),
            WorkspaceIsolation::Worktree { branch, setup } => {
                let id = reservation.worktree.ok_or(RegistryError::MissingBacking)?;
                let made = self
                    .create_worktree_with_id(
                        creator,
                        CreateWorktree {
                            project: reservation.intent.project,
                            branch: branch.clone(),
                            setup: *setup,
                        },
                        None,
                        id,
                    )
                    .await?;
                match made {
                    CreateWorktreeOutcome::Created { worktree } => (
                        worktree.path.clone(),
                        WorkspaceBacking::Managed {
                            worktree: worktree.id,
                        },
                        Some(worktree),
                    ),
                    CreateWorktreeOutcome::NeedsApproval { commands } => {
                        return Ok(Made::Approval(commands));
                    }
                    CreateWorktreeOutcome::Refused { reason } => return Ok(Made::Refused(reason)),
                }
            }
        };
        Ok(Made::Ready(Box::new(RegistryRecord {
            descriptor: WorkspaceDescriptor {
                id: reservation.workspace,
                project: reservation.intent.project,
                path,
                title: reservation.intent.title.clone(),
                backing,
                lifecycle: WorkspaceLifecycle::Active,
                visibility: reservation.intent.visibility,
                created_at: Utc::now(),
                revision: RegistryRevision::ZERO,
            },
            recovery,
        })))
    }
}
