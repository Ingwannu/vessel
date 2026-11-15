//! Async receipt APIs delegate all writes to the existing committed writer.

use vessel_proto::WorkspaceOperationId;
use vessel_proto::creation::{Advance, CreationError, Receipt, Revision};

use crate::creation::{Admission, Identity, Write as CreationWrite};
use crate::{Store, StoreError, writer::Write};

impl Store {
    /// Atomically admit an intent or return the existing matching operation.
    /// # Errors
    /// A key/intent or reserved-id conflict, capacity refusal, or storage failure.
    pub async fn admit_creation(&self, admission: Admission) -> Result<Receipt, StoreError> {
        let identity = admission.identity.clone();
        self.write(|done| Write::Creation {
            change: Box::new(CreationWrite::Admit(admission)),
            done,
        })
        .await?;
        self.creation(&identity)
            .await?
            .ok_or_else(|| CreationError::NotFound.into())
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
    /// # Errors
    /// A stale revision, invalid transition, missing operation or storage failure.
    pub async fn advance_creation(
        &self,
        operation: WorkspaceOperationId,
        expected: Revision,
        update: Advance,
    ) -> Result<Receipt, StoreError> {
        self.write(|done| Write::Creation {
            change: Box::new(CreationWrite::Advance {
                operation,
                expected,
                update,
            }),
            done,
        })
        .await?;
        self.creation_operation(operation)
            .await?
            .ok_or_else(|| CreationError::NotFound.into())
    }
}
