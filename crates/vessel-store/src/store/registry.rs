//! Async workspace registry handle; blocking SQL stays on existing store threads.

use super::Store;
use crate::{RegistryRecord, RegistryReservation, ReservationStage, StoreError, StoredRegistry};
use crate::{
    registry::{self, RegistryWrite},
    writer::Write,
};
use vessel_proto::WorkspaceId;
use vessel_proto::registry::{RegistryRevision, WorkspaceResource};

impl Store {
    /// Coalesced wake-up after committed registry writes; re-read authoritative state.
    /// Does not poll and never contains paths, payloads or secrets.
    #[must_use]
    pub fn registry_changes(&self) -> tokio::sync::watch::Receiver<()> {
        self.registry.clone()
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
