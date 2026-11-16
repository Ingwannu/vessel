//! Async receipt APIs delegate all writes to the existing committed writer.

use vessel_proto::WorkspaceOperationId;
use vessel_proto::creation::{Advance, Receipt, Revision};

use crate::creation::{Admission, Identity, Write as CreationWrite};
use crate::{Store, StoreError, writer::Write};

#[cfg(test)]
mod tests;

impl Store {
    /// Coalesced notifications after committed receipt writes. Subscribe before
    /// reading the initial snapshot, then re-read authoritative SQL after each
    /// wake-up. Intermediate revisions may coalesce; no payload is retained here.
    #[must_use]
    pub fn creation_changes(&self) -> tokio::sync::watch::Receiver<()> {
        let mut changes = self.creation.clone();
        drop(changes.borrow_and_update());
        changes
    }

    /// Atomically admit an intent or return the existing matching operation.
    /// # Errors
    /// A key/intent or reserved-id conflict, capacity refusal, or storage failure.
    pub async fn admit_creation(&self, admission: Admission) -> Result<Receipt, StoreError> {
        self.write_creation(CreationWrite::Admit(admission)).await
    }

    /// Observe by caller key; absence is distinct from interrupted work.
    /// # Errors
    /// An unreadable database or a corrupt receipt; never a silently dropped row.
    pub async fn creation(&self, identity: &Identity) -> Result<Option<Receipt>, StoreError> {
        let identity = identity.clone();
        self.read(move |reader| {
            reader.with(|connection| crate::creation::lookup(connection, &identity))
        })
        .await
    }

    /// Observe a reserved operation by its stable shared operation identity.
    /// # Errors
    /// An unreadable database or corrupt receipt.
    pub async fn creation_operation(
        &self,
        operation: WorkspaceOperationId,
    ) -> Result<Option<Receipt>, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| crate::creation::operation(connection, operation))
        })
        .await
    }

    /// Commit a stage marker/result with compare-and-swap before proceeding.
    /// Returns that committed mutation, never a later reader snapshot.
    /// # Errors
    /// A stale revision, invalid transition, missing operation or storage failure.
    pub async fn advance_creation(
        &self,
        operation: WorkspaceOperationId,
        expected: Revision,
        update: Advance,
    ) -> Result<Receipt, StoreError> {
        self.write_creation(CreationWrite::Advance {
            operation,
            expected,
            update,
        })
        .await
    }
    async fn write_creation(&self, change: CreationWrite) -> Result<Receipt, StoreError> {
        let (done, committed) = tokio::sync::oneshot::channel();
        self.send(Write::Creation {
            change: Box::new(change),
            done,
        })
        .await?;
        committed.await.map_err(|_dropped| StoreError::Closed)?
    }
}
