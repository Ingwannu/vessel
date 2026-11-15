//! One bounded admission gate; the retained driver, never its reply, owns a slot.
//! Staged with the driver until current authority/count/start leases are wired.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use vessel_proto::WorkspaceOperationId;
use vessel_proto::creation::{Certainty, CreationError, Fingerprint, Phase, Receipt};
use vessel_store::{CreationAdmission, CreationIdentity, Store, StoreError};

const MAX_ACTIVE_CREATIONS: usize = 64;
const CREATION_ADMISSION_WAIT: Duration = Duration::from_secs(5);

#[derive(Default)]
struct Active {
    operations: Mutex<HashMap<WorkspaceOperationId, Arc<()>>>,
}

impl Active {
    fn lock(&self) -> MutexGuard<'_, HashMap<WorkspaceOperationId, Arc<()>>> {
        // This book owns no authority or persisted state. Driver drops must
        // release their own marker even after an unrelated caller panicked.
        match self.operations.lock() {
            Ok(book) => book,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

struct Intake {
    admission: tokio::sync::Mutex<()>,
    slots: Arc<Semaphore>,
    active: Arc<Active>,
}

impl Default for Intake {
    fn default() -> Self {
        Self {
            admission: tokio::sync::Mutex::new(()),
            slots: Arc::new(Semaphore::new(MAX_ACTIVE_CREATIONS)),
            active: Arc::default(),
        }
    }
}

enum Admitted<L> {
    Observe(Receipt),
    Start(Driver<L>),
}

/// The callback's opaque lease belongs to the existing count/authority owner.
/// Its Drop rolls back only a reservation actually acquired by this attempt.
struct Driver<L> {
    receipt: Receipt,
    resource: Option<L>,
    marker: Arc<()>,
    active: Arc<Active>,
    _slot: OwnedSemaphorePermit,
}

impl<L> Drop for Driver<L> {
    fn drop(&mut self) {
        // A new continuation may acquire count only after this attempt's
        // reservation has actually settled/rolled back, not during field Drop.
        drop(self.resource.take());
        let operation = self.receipt.snapshot().reserved.operation;
        let mut book = self.active.lock();
        if book
            .get(&operation)
            .is_some_and(|marker| Arc::ptr_eq(marker, &self.marker))
        {
            let _retired = book.remove(&operation);
        }
    }
}

#[derive(Debug)]
enum IntakeError<E> {
    Busy,
    Capacity,
    Store(StoreError),
    Prepare(E),
}

impl<E> From<StoreError> for IntakeError<E> {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl Intake {
    /// `prepare` rechecks current authority and takes a shared resource/count
    /// lease, without launching anything. Receipt identity is canonical before
    /// it runs; it must never replace an existing/restored child entry.
    async fn admit<L, E, F, Fut>(
        &self,
        store: &Store,
        candidate: CreationAdmission,
        prepare: F,
    ) -> Result<Admitted<L>, IntakeError<E>>
    where
        F: FnOnce(Receipt) -> Fut,
        Fut: Future<Output = Result<L, E>>,
    {
        let fingerprint = candidate.receipt.fingerprint();
        if let Some(receipt) = self.lookup(store, &candidate.identity, fingerprint).await?
            && self.observe_only(&receipt)
        {
            return Ok(Admitted::Observe(receipt));
        }
        // No resource/count lease is held while waiting. A duplicate which
        // waited behind the winner reuses its committed ids before capacity.
        let _gate = tokio::time::timeout(CREATION_ADMISSION_WAIT, self.admission.lock())
            .await
            .map_err(|_elapsed| IntakeError::Busy)?;
        let existing = self.lookup(store, &candidate.identity, fingerprint).await?;
        if let Some(receipt) = &existing
            && self.observe_only(receipt)
        {
            return Ok(Admitted::Observe(receipt.clone()));
        }
        if self.active.lock().len() >= MAX_ACTIVE_CREATIONS {
            return Err(IntakeError::Capacity);
        }
        let slot = Arc::clone(&self.slots)
            .try_acquire_owned()
            .map_err(|_full| IntakeError::Capacity)?;
        let receipt = existing.as_ref().unwrap_or(&candidate.receipt).clone();
        let reserved = receipt.snapshot().reserved.clone();
        let resource = prepare(receipt).await.map_err(IntakeError::Prepare)?;
        let receipt = match existing {
            Some(receipt) => receipt,
            None => store.admit_creation(candidate).await?,
        };
        // A different internal writer may have reconciled the same key while
        // admission was awaiting its commit. Never attach our lease to its ids.
        if receipt.snapshot().reserved != reserved {
            return Ok(Admitted::Observe(receipt));
        }
        let marker = Arc::new(());
        let operation = receipt.snapshot().reserved.operation;
        if let std::collections::hash_map::Entry::Vacant(entry) =
            self.active.lock().entry(operation)
        {
            let _marker = entry.insert(Arc::clone(&marker));
        } else {
            return Ok(Admitted::Observe(receipt));
        }
        Ok(Admitted::Start(Driver {
            receipt,
            resource: Some(resource),
            marker,
            active: Arc::clone(&self.active),
            _slot: slot,
        }))
    }

    async fn lookup<E>(
        &self,
        store: &Store,
        identity: &CreationIdentity,
        fingerprint: Fingerprint,
    ) -> Result<Option<Receipt>, IntakeError<E>> {
        let receipt = store.creation(identity).await?;
        if receipt
            .as_ref()
            .is_some_and(|receipt| receipt.fingerprint() != fingerprint)
        {
            return Err(IntakeError::Store(CreationError::IntentConflict.into()));
        }
        Ok(receipt)
    }

    fn observe_only(&self, receipt: &Receipt) -> bool {
        receipt.in_flight().is_some()
            || self
                .active
                .lock()
                .contains_key(&receipt.snapshot().reserved.operation)
            || matches!(
                receipt.snapshot().phase,
                Phase::Completed
                    | Phase::Failed {
                        certainty: Certainty::Unknown,
                        ..
                    }
            )
    }
}

#[cfg(test)]
mod tests;
