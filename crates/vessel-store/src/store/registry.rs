//! Async workspace registry handle; blocking SQL stays on existing store threads.

use super::Store;
use crate::{RegistryRecord, RegistryReservation, ReservationStage, StoreError, StoredRegistry};
use crate::{
    registry::{self, RegistryWrite},
    writer::Write,
};
use vessel_proto::registry::{RegistryRevision, WorkspaceResource};
use vessel_proto::{ProjectId, WorkspaceId};

impl Store {
    /// Find retained descriptor OR backing-reservation ownership for legacy cleanup.
    /// This one read transaction never infers ownership from a path or live process.
    /// The caller must retain canonical project metadata admission through effects;
    /// None alone grants no deletion, lifecycle, exit or physical-root authority.
    ///
    /// # Errors
    /// Database or typed retained-state decode failure; callers must fail closed.
    pub async fn registry_project_reference(
        &self,
        project: ProjectId,
    ) -> Result<Option<WorkspaceId>, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| registry::project_reference(connection, project))
        })
        .await
    }
    /// Admit a next run for the same actor only after its supervisor proves the previous run joined.
    /// The exact predecessor must still be Ready; this check is not itself proof of process exit.
    ///
    /// # Errors
    /// Foreign/stale/unsettled predecessor, non-Active/stale workspace, or persistence failure.
    pub async fn claim_registry_restart(
        &self,
        previous: crate::WorkspaceStartLease,
        expected: RegistryRevision,
    ) -> Result<crate::WorkspaceStartLease, StoreError> {
        let next = expected
            .next()
            .ok_or(StoreError::RegistryRevisionExhausted)?;
        let lease =
            crate::WorkspaceStartLease::new(previous.workspace(), previous.resource(), next);
        self.registry_write(RegistryWrite::Restart { previous, expected })
            .await?;
        Ok(lease)
    }

    /// Recover exact retained admission metadata without inferring process state or new ownership.
    ///
    /// # Errors
    /// Database or typed stored-state decode failure.
    pub async fn registry_resource_run(
        &self,
        resource: WorkspaceResource,
    ) -> Result<Option<crate::RegistryResourceRun>, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| registry::resource_run(connection, resource))
        })
        .await
    }

    /// Release only the exact retained source/version after explicit person authorization.
    /// This does not approve a digest, run setup or retry earlier work.
    ///
    /// # Errors
    /// Changed source/revision, non-Active lifecycle, missing identity or persistence failure.
    pub async fn release_registry_automation(
        &self,
        request: vessel_proto::registry::ReleaseWorkspaceAutomation,
    ) -> Result<(), StoreError> {
        self.registry_write(RegistryWrite::Release(request)).await
    }

    /// Admit a preallocated actor id before spawn, with an archive fence in the same transaction.
    ///
    /// # Errors
    /// Non-Active/stale workspace, another owner/lease, exhausted revision, database failure.
    pub async fn claim_registry_start(
        &self,
        workspace: WorkspaceId,
        resource: WorkspaceResource,
        expected: RegistryRevision,
    ) -> Result<crate::WorkspaceStartLease, StoreError> {
        let revision = expected
            .next()
            .ok_or(StoreError::RegistryRevisionExhausted)?;
        self.registry_write(RegistryWrite::Claim {
            workspace,
            resource,
            expected,
        })
        .await?;
        Ok(crate::WorkspaceStartLease::new(
            workspace, resource, revision,
        ))
    }
    /// Settle only after actor registration; a prior admission may settle during Archiving.
    ///
    /// # Errors
    /// Stale/missing lease, exhausted revision or database failure.
    pub async fn complete_registry_start(
        &self,
        lease: crate::WorkspaceStartLease,
    ) -> Result<(), StoreError> {
        self.registry_write(RegistryWrite::Settle {
            lease,
            settled: registry::RegistryStartSettlement::Ready,
        })
        .await
    }
    /// Only a proven failed/not-spawned start may be failed and made retryable.
    ///
    /// # Errors
    /// Stale/missing lease, exhausted revision or database failure.
    pub async fn fail_registry_start(
        &self,
        lease: crate::WorkspaceStartLease,
    ) -> Result<(), StoreError> {
        self.registry_write(RegistryWrite::Settle {
            lease,
            settled: registry::RegistryStartSettlement::Failed,
        })
        .await
    }
    /// Archive waits for these exact admissions after claiming Archiving, before stopping resources.
    ///
    /// # Errors
    /// Invalid stored state or database failure.
    pub async fn pending_registry_starts(
        &self,
        workspace: WorkspaceId,
    ) -> Result<Vec<crate::PendingRegistryStart>, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| registry::pending_starts(connection, workspace))
        })
        .await
    }

    /// Coalesced wake-up after committed registry writes; re-read authoritative state.
    /// Does not poll and never contains paths, payloads or secrets.
    #[must_use]
    pub fn registry_changes(&self) -> tokio::sync::watch::Receiver<()> {
        let mut changes = self.registry.clone();
        // Clone preserves the source receiver's unseen history; a fresh long-poll must not spin.
        // Mark before the caller reads its snapshot, so concurrent future commits remain visible.
        changes.mark_unchanged();
        changes
    }

    /// Resolve explicit ownership; a shared cwd is never a membership claim.
    ///
    /// # Errors
    /// The database or stored typed identifier failed to decode.
    pub async fn registry_resource_owner(
        &self,
        resource: WorkspaceResource,
    ) -> Result<Option<WorkspaceId>, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| registry::resource_owner(connection, resource))
        })
        .await
    }

    /// Startup-only reconciliation: an interrupted side effect cannot be reissued blindly.
    ///
    /// # Errors
    /// The database write failed; callers must not start registry provisioning then.
    pub async fn recover_registry_reservations(&self) -> Result<(), StoreError> {
        self.registry_write(RegistryWrite::RecoverCreating).await
    }

    /// Publish the descriptor and complete its backing reservation atomically.
    ///
    /// # Errors
    /// Missing/non-Creating reservation, duplicate descriptor, or database failure.
    pub async fn complete_registry_workspace(
        &self,
        record: RegistryRecord,
    ) -> Result<(), StoreError> {
        self.registry_write(RegistryWrite::Completed(Box::new(record)))
            .await
    }
    /// Atomically read placement and explicit session/terminal ownership.
    ///
    /// # Errors
    /// Database failure or invalid stored typed state; nothing is silently skipped.
    pub async fn workspace_registry(&self) -> Result<StoredRegistry, StoreError> {
        self.read(|reader| reader.with(registry::load)).await
    }
    /// Inspect exact identity including archived records and recovery metadata.
    ///
    /// # Errors
    /// Database/read decoding failures.
    pub async fn registry_workspace(
        &self,
        id: WorkspaceId,
    ) -> Result<Option<RegistryRecord>, StoreError> {
        self.read(move |reader| reader.with(|connection| registry::workspace(connection, id)))
            .await
    }
    /// CAS a new or changed descriptor; input revision must be zero or expected+1.
    ///
    /// # Errors
    /// Stale revision, altered immutable placement identity or a database failure.
    pub async fn put_registry_workspace(
        &self,
        record: RegistryRecord,
        expected: Option<RegistryRevision>,
    ) -> Result<(), StoreError> {
        self.registry_write(RegistryWrite::Workspace {
            record: Box::new(record),
            expected,
        })
        .await
    }
    /// Admit a member only while lifecycle is Active at the expected revision.
    ///
    /// # Errors
    /// Stale/archiving/archived identity, conflicting existing owner, database failure.
    pub async fn set_registry_membership(
        &self,
        workspace: WorkspaceId,
        resource: WorkspaceResource,
        expected: RegistryRevision,
    ) -> Result<(), StoreError> {
        self.registry_write(RegistryWrite::Membership {
            workspace,
            resource,
            expected,
        })
        .await
    }
    /// Persist fixed backing identity before filesystem work; identical retries are idempotent.
    ///
    /// # Errors
    /// Reused identity with changed intent/backing, or database failure.
    pub async fn reserve_registry_workspace(
        &self,
        reservation: RegistryReservation,
    ) -> Result<(), StoreError> {
        self.registry_write(RegistryWrite::Reserve(reservation))
            .await
    }
    /// Read reservation independently of whether provisioning completed its descriptor.
    ///
    /// # Errors
    /// Database or typed stored-state decoding failure.
    pub async fn registry_reservation(
        &self,
        id: WorkspaceId,
    ) -> Result<Option<RegistryReservation>, StoreError> {
        self.read(move |reader| reader.with(|connection| registry::reservation(connection, id)))
            .await
    }
    /// CAS provisioning's side-effect admission; interrupted Creating is never blindly replayed.
    ///
    /// # Errors
    /// Missing/wrong-stage reservation or a database failure.
    pub async fn set_registry_reservation_stage(
        &self,
        workspace: WorkspaceId,
        expected: ReservationStage,
        stage: ReservationStage,
    ) -> Result<(), StoreError> {
        self.registry_write(RegistryWrite::Stage {
            workspace,
            expected,
            stage,
        })
        .await
    }
    async fn registry_write(&self, mutation: RegistryWrite) -> Result<(), StoreError> {
        self.write(|done| Write::Registry { mutation, done }).await
    }
}
