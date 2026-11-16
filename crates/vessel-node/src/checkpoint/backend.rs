//! Real Linux capture SQL/filesystem assembly installed by the bounded actor bridge.
//! No second writer or unbounded filesystem task is introduced here.
use super::runner::{self, Client, Disposition, Publication};
use crate::workspace::Workspaces;
use std::sync::Arc;
use vessel_proto::checkpoint::{CheckpointError, CheckpointId};
use vessel_store::{CheckpointStoreError, Store, StoreError, checkpoint::CheckpointWrite};
use vessel_workspace::checkpoint::{Cleanup, IgnoreDefaults, Storage};

pub(super) fn start_shared(
    workspaces: Arc<Workspaces>,
    store: Store,
    defaults: IgnoreDefaults,
    workers: super::workers::Workers,
) -> (Client, tokio::task::JoinHandle<()>) {
    let runtime = tokio::runtime::Handle::current();
    let reader = Arc::clone(&workspaces);
    let cleaner = workspaces;
    let cleanup_store = store.clone();
    let cleanup_runtime = runtime.clone();
    runner::start_shared(
        workers,
        move |key, physical, cancel| {
            reader.capture_checkpoint(&runtime, key, physical, &defaults, &cancel)
        },
        move |key, publication| {
            let store = store.clone();
            async move {
                let write = publication_write(key, publication);
                store.checkpoint_write(write).await.map_err(reason)
            }
        },
        move |key, disposition| {
            cleanup_runtime.block_on(async {
                match disposition {
                    Disposition::Keep => {}
                    Disposition::NoReservation => return Ok(()),
                    Disposition::Discard(failure) => {
                        cleanup_store
                            .checkpoint_write(CheckpointWrite::Unavailable {
                                key,
                                reason: failure,
                            })
                            .await
                            .map_err(reason)?;
                        // Already on the runner's retained blocking worker. This async
                        // future only drives the sole writer, not another I/O task.
                        cleaner.clean_checkpoint(key)?;
                    }
                    // Reserve/activation may have committed. Never destroy unknown
                    // Ready bytes or release the worker before reconciliation.
                    Disposition::Recovery => return Err(CheckpointError::RecoveryPending),
                }
                cleanup_store
                    .checkpoint_write(CheckpointWrite::CaptureSettled { key })
                    .await
                    .map_err(reason)
            })
        },
    )
}

fn publication_write(
    key: vessel_proto::checkpoint::CaptureKey,
    publication: Publication,
) -> CheckpointWrite {
    match publication {
        Publication::Reserve(_escrow) => CheckpointWrite::ReserveCapture {
            key,
            at: chrono::Utc::now(),
        },
        Publication::Candidate(snapshot) => CheckpointWrite::Candidate {
            key,
            snapshot: Box::new(snapshot),
        },
        Publication::Activate => CheckpointWrite::Activate {
            key,
            checkpoint: CheckpointId::random(),
        },
    }
}

pub(crate) fn reason(error: StoreError) -> CheckpointError {
    let reason = match &error {
        StoreError::Checkpoint(CheckpointStoreError::Refused(reason)) => *reason,
        StoreError::Checkpoint(
            CheckpointStoreError::Duration(_)
            | CheckpointStoreError::Id(_)
            | CheckpointStoreError::Sql(_)
            | CheckpointStoreError::Json(_),
        )
        | StoreError::Io { .. }
        | StoreError::NotPrivate { .. }
        | StoreError::Locked { .. }
        | StoreError::Random(_)
        | StoreError::InvalidNodeId { .. }
        | StoreError::InvalidNodeKey { .. }
        | StoreError::InvalidToken { .. }
        | StoreError::NewerSchema { .. }
        | StoreError::Sqlite(_)
        | StoreError::Commit(_)
        | StoreError::RegistryMissing(_)
        | StoreError::RegistryInactive(_)
        | StoreError::RegistryConflict
        | StoreError::RegistryFull
        | StoreError::RegistryRecordTooLarge
        | StoreError::RegistryRevisionExhausted
        | StoreError::RegistryRevisionRange(_)
        | StoreError::RegistryDecode(_)
        | StoreError::Corrupt { .. }
        | StoreError::CorruptId { .. }
        | StoreError::CorruptDevice { .. }
        | StoreError::CorruptAttachment { .. }
        | StoreError::CorruptMessage { .. }
        | StoreError::UnknownDevice(_)
        | StoreError::DeviceExists(_)
        | StoreError::Encode(_)
        | StoreError::OutOfRange(_)
        | StoreError::UnknownSession(_)
        | StoreError::SessionExists(_)
        | StoreError::SessionLive(_)
        | StoreError::SessionEnded(_)
        | StoreError::Attachment(_)
        | StoreError::OutOfOrder { .. }
        | StoreError::InboxGone
        | StoreError::PushDenied
        | StoreError::PushFull
        | StoreError::CorruptPush { .. }
        | StoreError::NotChild(_)
        | StoreError::InvalidParentage(_)
        | StoreError::Closed
        | StoreError::Thread(_)
        | StoreError::UsageBusy(_)
        | StoreError::Task(_) => CheckpointError::Unknown,
    };
    // This ownership boundary deliberately discards driver paths/SQL sources;
    // callers expose only the stable refusal, never the private source text.
    drop(error);
    reason
}

/// Called on the retained cleanup worker after durable publication invalidation.
/// Errors keep both persistent escrow and the runner's worker/root reservation.
pub(crate) fn clean(
    storage: &Storage,
    worktree: vessel_proto::WorktreeId,
    capture: vessel_proto::checkpoint::CaptureId,
) -> Result<(), CheckpointError> {
    loop {
        match storage
            .clean_capture(worktree, capture)
            .map_err(|error| error.reason())?
        {
            Cleanup::Pending => {}
            Cleanup::Complete => return Ok(()),
        }
    }
}

#[cfg(test)]
pub(crate) fn start(
    workspaces: Arc<Workspaces>,
    store: Store,
    defaults: IgnoreDefaults,
) -> (Client, tokio::task::JoinHandle<()>) {
    start_shared(
        workspaces,
        store,
        defaults,
        super::workers::Workers::default(),
    )
}
