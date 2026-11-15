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
                let write = match publication {
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
                };
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

pub(crate) fn reason(error: StoreError) -> CheckpointError {
    if let StoreError::Checkpoint(CheckpointStoreError::Refused(reason)) = error {
        reason
    } else {
        CheckpointError::Unknown
    }
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
            .clean_capture(key.worktree, key.capture)
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
