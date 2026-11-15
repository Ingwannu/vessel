//! Backing/descriptor orchestration over the sole registry and project hold.
//! No initial agent, terminal, bootstrap or public creation RPC is activated.

mod backing;

use super::{WorkspaceError, Workspaces, projects::admission::ProjectAdmission};
use std::sync::Arc;
use vessel_proto::creation::{Certainty, Stage, WorkspaceClaim};
use vessel_proto::registry::{WorkspaceDescriptor, WorkspaceIsolation, WorkspaceLifecycle};
use vessel_proto::workspace::{ProjectCommands, WorktreeRefusal};
use vessel_proto::{Creator, WorkspaceId, WorkspaceOperationId, WorktreeId};
use vessel_store::{RegistryReservation, ReservationStage, StoreError, WorkspaceProvision};

/// Backing readiness alone does not attest setup completion or resource launch.
#[derive(Debug)]
pub(crate) enum Provisioned {
    Ready(Box<WorkspaceDescriptor>),
    NeedsApproval(Box<ProjectCommands>),
    Refused {
        reason: WorktreeRefusal,
        certainty: Certainty,
    },
    InProgress(WorkspaceId),
    Uncertain(WorkspaceId),
}

impl Workspaces {
    /// Requires the committed receipt marker; no caller-supplied workspace id.
    /// The private caller still owns current principal/generation authorization.
    pub(crate) async fn provision_workspace(
        self: &Arc<Self>,
        creator: Creator,
        operation: WorkspaceOperationId,
        intent: WorkspaceProvision,
    ) -> Result<Provisioned, WorkspaceError> {
        let receipt = self
            .store
            .creation_operation(operation)
            .await?
            .ok_or_else(|| WorkspaceError::Conflict("creation receipt is absent".to_owned()))?;
        if receipt.in_flight() != Some(Stage::Workspace)
            || receipt.snapshot().reserved.workspace_claim != WorkspaceClaim::Create
        {
            return Err(WorkspaceError::Conflict(
                "workspace provisioning has no committed creation marker".to_owned(),
            ));
        }
        let id = receipt.snapshot().reserved.workspace;
        if let Some(found) = self.provision_status(id, &intent).await? {
            return Ok(found);
        }
        if self.stop.is_cancelled() {
            return Err(WorkspaceError::Conflict(
                "workspace creation stopped".to_owned(),
            ));
        }
        // Reserve the shared bounded hold BEFORE detaching. Cancellation cannot
        // grow an unbounded queue of background admission waiters.
        let admission = self.admit_project(intent.project).await?;
        let owner = Arc::clone(self);
        self.tasks
            .spawn(async move {
                owner
                    .provision_admitted(creator, id, intent, admission)
                    .await
            })
            .await
            .map_err(WorkspaceError::project_task)?
    }

    async fn provision_admitted(
        self: &Arc<Self>,
        creator: Creator,
        id: WorkspaceId,
        intent: WorkspaceProvision,
        admission: Arc<ProjectAdmission>,
    ) -> Result<Provisioned, WorkspaceError> {
        // Another retained owner may have settled while this caller waited.
        if let Some(found) = self.provision_status(id, &intent).await? {
            return Ok(found);
        }
        admission.require(self, intent.project)?;
        let reservation = self.reserve_provision(id, intent).await?;
        if self.stop.is_cancelled() {
            return Err(WorkspaceError::Conflict(
                "workspace creation stopped".to_owned(),
            ));
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
            Err(StoreError::RegistryConflict) => {
                return Ok(Provisioned::InProgress(id));
            }
            Err(error) => return Err(error.into()),
        }
        if self.stop.is_cancelled() {
            self.store
                .set_registry_reservation_stage(
                    id,
                    ReservationStage::Creating,
                    ReservationStage::Uncertain,
                )
                .await?;
            return Ok(Provisioned::Uncertain(id));
        }
        // This Arc remains in our frame through the final Store acknowledgement;
        // the backing task receives only a clone, and never re-admits the hold.
        let made = self.make_backing(creator, &reservation, &admission).await;
        match made {
            Ok(backing::Made::Ready(record)) => {
                self.store.complete_registry_workspace(*record).await?;
                let ready = self.store.registry_workspace(id).await?.ok_or_else(|| {
                    WorkspaceError::Conflict("completed workspace descriptor is absent".to_owned())
                })?;
                Ok(Provisioned::Ready(Box::new(ready.descriptor)))
            }
            Ok(backing::Made::Approval(commands)) => {
                self.store
                    .set_registry_reservation_stage(
                        id,
                        ReservationStage::Creating,
                        ReservationStage::Reserved,
                    )
                    .await?;
                Ok(Provisioned::NeedsApproval(commands))
            }
            result => {
                // Generic backing errors/refusals cannot prove no filesystem
                // effect. Retain the fixed id and refuse automatic re-creation.
                self.store
                    .set_registry_reservation_stage(
                        id,
                        ReservationStage::Creating,
                        ReservationStage::Uncertain,
                    )
                    .await?;
                match result {
                    Ok(backing::Made::Refused(reason)) => Ok(Provisioned::Refused {
                        reason,
                        certainty: Certainty::Unknown,
                    }),
                    Err(error) => Err(error),
                    Ok(backing::Made::Ready(_) | backing::Made::Approval(_)) => Err(
                        WorkspaceError::Conflict("workspace result changed".to_owned()),
                    ),
                }
            }
        }
    }

    async fn provision_status(
        &self,
        id: WorkspaceId,
        intent: &WorkspaceProvision,
    ) -> Result<Option<Provisioned>, WorkspaceError> {
        let Some(reservation) = self.store.registry_reservation(id).await? else {
            return Ok(None);
        };
        if reservation.intent != *intent {
            return Err(WorkspaceError::Conflict(
                "workspace provision intent changed".to_owned(),
            ));
        }
        match reservation.stage {
            ReservationStage::Reserved => Ok(None),
            ReservationStage::Creating => Ok(Some(Provisioned::InProgress(id))),
            ReservationStage::Uncertain => Ok(Some(Provisioned::Uncertain(id))),
            ReservationStage::Ready => {
                let Some(record) = self.store.registry_workspace(id).await? else {
                    return Ok(Some(Provisioned::Uncertain(id)));
                };
                if record.descriptor.lifecycle != WorkspaceLifecycle::Active {
                    return Err(WorkspaceError::Conflict(
                        "workspace is not active".to_owned(),
                    ));
                }
                Ok(Some(Provisioned::Ready(Box::new(record.descriptor))))
            }
        }
    }

    async fn reserve_provision(
        &self,
        id: WorkspaceId,
        intent: WorkspaceProvision,
    ) -> Result<RegistryReservation, WorkspaceError> {
        if let Some(found) = self.store.registry_reservation(id).await? {
            return if found.intent == intent {
                Ok(found)
            } else {
                Err(WorkspaceError::Conflict(
                    "workspace provision intent changed".to_owned(),
                ))
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
        self.store
            .reserve_registry_workspace(proposed.clone())
            .await?;
        Ok(proposed)
    }
}
