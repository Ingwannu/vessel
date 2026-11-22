//! Retain the existing project admission from intent through publication.
//! Backing and setup have separate admission so an approved script always
//! runs against a durable workspace identity.

use std::sync::Arc;

use chrono::Utc;
use vessel_proto::registry::{
    RegistryRevision, WorkspaceBacking, WorkspaceDescriptor, WorkspaceIsolation,
    WorkspaceLifecycle, WorkspacePin, WorkspaceResource,
};
use vessel_proto::workspace::{
    CreateWorktree, CreateWorktreeOutcome, SessionPlace, SetupChoice, SetupState, WorktreeInfo,
};
use vessel_proto::{Creator, WorkspaceId, WorktreeId};
use vessel_store::{
    RegistryRecord, RegistryReservation, ReservationStage, StoreError, WorkspaceProvision,
    WorkspaceStartLease,
};

use crate::workspace::projects::admission::ProjectAdmission;
use crate::workspace::{WorkspaceError, Workspaces};

/// Durable independent identity and its ordinary typed execution place.
#[derive(Debug)]
pub(crate) struct Provisioned {
    pub(crate) descriptor: WorkspaceDescriptor,
    pub(crate) place: SessionPlace,
}

impl Provisioned {
    pub(super) fn of(descriptor: WorkspaceDescriptor) -> Self {
        let place = match descriptor.backing {
            WorkspaceBacking::Local => SessionPlace::Project {
                project: descriptor.project,
                dir: None,
            },
            WorkspaceBacking::Managed { worktree } => SessionPlace::Worktree { worktree },
        };
        Self { descriptor, place }
    }
}

impl Workspaces {
    /// Reserve backing before effects. A disconnected caller can recover a
    /// Ready receipt but cannot replay an unconfirmed creation.
    pub(crate) async fn provision_workspace(
        self: &Arc<Self>,
        id: WorkspaceId,
        admission: Arc<ProjectAdmission>,
        intent: WorkspaceProvision,
        creator: Creator,
    ) -> Result<Provisioned, WorkspaceError> {
        admission.require(self, intent.project)?;
        let workspaces = Arc::clone(self);
        self.tasks
            .spawn(async move {
                workspaces
                    .provision_under(id, admission, intent, creator)
                    .await
            })
            .await
            .map_err(WorkspaceError::project_task)?
    }

    async fn provision_under(
        self: &Arc<Self>,
        id: WorkspaceId,
        admission: Arc<ProjectAdmission>,
        intent: WorkspaceProvision,
        creator: Creator,
    ) -> Result<Provisioned, WorkspaceError> {
        let retained = self.store.registry_reservation(id).await?;
        let worktree = retained.as_ref().map_or_else(
            || match intent.isolation {
                WorkspaceIsolation::Local | WorkspaceIsolation::Unknown => None,
                WorkspaceIsolation::Worktree { .. } => Some(WorktreeId::random()),
            },
            |reservation| reservation.worktree,
        );
        self.store
            .reserve_registry_workspace(RegistryReservation {
                workspace: id,
                worktree,
                stage: ReservationStage::Reserved,
                intent: intent.clone(),
            })
            .await?;
        match retained.map(|row| row.stage) {
            Some(ReservationStage::Ready) => {
                let record = self
                    .store
                    .registry_workspace(id)
                    .await?
                    .ok_or(StoreError::RegistryMissing(id))?;
                return Ok(Provisioned::of(record.descriptor));
            }
            Some(ReservationStage::Creating | ReservationStage::Uncertain) => {
                return Err(WorkspaceError::Conflict(
                    "workspace provision is unconfirmed; the exact backing must be reconciled"
                        .to_owned(),
                ));
            }
            Some(ReservationStage::Retired) => return Err(StoreError::RegistryFinished(id).into()),
            None | Some(ReservationStage::Reserved) => {}
        }
        self.store
            .set_registry_reservation_stage(
                id,
                ReservationStage::Reserved,
                ReservationStage::Creating,
            )
            .await?;
        let project_root = admission.root().path().to_path_buf();
        let (recovery, setup) = match (intent.isolation, worktree) {
            (WorkspaceIsolation::Local, None) => (None, SetupChoice::Skip),
            (WorkspaceIsolation::Worktree { branch, setup }, Some(worktree)) => {
                let request = CreateWorktree {
                    project: intent.project,
                    branch,
                    setup: SetupChoice::Skip,
                };
                let info = self
                    .provision_backing(id, Arc::clone(&admission), creator, request, worktree)
                    .await?;
                (Some(info), setup)
            }
            _ => return Err(StoreError::RegistryConflict.into()),
        };
        let backing =
            recovery
                .as_ref()
                .map_or(WorkspaceBacking::Local, |info| WorkspaceBacking::Managed {
                    worktree: info.id,
                });
        let path = recovery
            .as_ref()
            .map_or(project_root, |info| info.path.clone());
        let descriptor = WorkspaceDescriptor {
            id,
            project: intent.project,
            path,
            title: intent.title,
            automation: intent.automation,
            pin: WorkspacePin::Unpinned,
            backing,
            lifecycle: WorkspaceLifecycle::Active,
            visibility: intent.visibility,
            created_at: Utc::now(),
            revision: RegistryRevision::ZERO,
        };
        self.store
            .complete_registry_workspace(RegistryRecord {
                descriptor: descriptor.clone(),
                recovery,
            })
            .await?;
        drop(admission);
        if setup == SetupChoice::Run
            && let Some(worktree) = worktree
        {
            let _setup = self.run_setup(worktree).await?;
            self.wait_for_provision_setup(worktree).await?;
        }
        Ok(Provisioned::of(descriptor))
    }

    async fn provision_backing(
        self: &Arc<Self>,
        id: WorkspaceId,
        admission: Arc<ProjectAdmission>,
        creator: Creator,
        request: CreateWorktree,
        worktree: WorktreeId,
    ) -> Result<WorktreeInfo, WorkspaceError> {
        let result = self
            .create_worktree_admitted(admission, creator, request, None, worktree)
            .await;
        match result {
            Ok(CreateWorktreeOutcome::Created { worktree }) => Ok(worktree),
            Ok(outcome) => {
                self.store
                    .set_registry_reservation_stage(
                        id,
                        ReservationStage::Creating,
                        ReservationStage::Reserved,
                    )
                    .await?;
                Err(WorkspaceError::Conflict(format!(
                    "the workspace backing could not be made: {outcome:?}"
                )))
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

    async fn wait_for_provision_setup(&self, worktree: WorktreeId) -> Result<(), WorkspaceError> {
        let mut changes = {
            let state = self.state();
            state
                .worktrees
                .get(&worktree)
                .ok_or(WorkspaceError::UnknownWorktree(worktree))?
                .written
                .subscribe()
        };
        loop {
            if !matches!(self.worktree(worktree)?.setup, SetupState::Running { .. }) {
                return Ok(());
            }
            tokio::select! {
                result = changes.changed() => result.map_err(|_| WorkspaceError::UnknownWorktree(worktree))?,
                () = self.stop.cancelled() => return Err(WorkspaceError::Conflict(
                    "the node stopped while setting up the workspace".to_owned()
                )),
            }
        }
    }

    /// Admission and completion are the same Store leases used by other workspace callers.
    pub(crate) async fn admit_workspace_start(
        &self,
        workplace: &Provisioned,
        resource: WorkspaceResource,
    ) -> Result<WorkspaceStartLease, WorkspaceError> {
        let id = workplace.descriptor.id;
        let current = self
            .store
            .registry_workspace(id)
            .await?
            .ok_or(StoreError::RegistryMissing(id))?;
        Ok(self
            .store
            .claim_registry_start(id, resource, current.descriptor.revision)
            .await?)
    }
}
