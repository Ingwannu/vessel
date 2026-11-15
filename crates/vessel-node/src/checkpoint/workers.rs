//! One node-wide checkpoint I/O budget, shared by capture, restore and collectors.
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use vessel_proto::checkpoint::{CHECKPOINT_WORKERS, CheckpointError};

#[derive(Debug, Clone)]
pub(crate) struct Workers(Arc<Semaphore>);
#[derive(Debug)]
pub(crate) struct WorkerLease {
    _permit: OwnedSemaphorePermit,
}
impl Default for Workers {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(CHECKPOINT_WORKERS)))
    }
}
impl Workers {
    /// No spawned waiters. Actual blocking closures retain a lease even if their
    /// async caller disappears; failed cleanup also keeps the coordinator's copy.
    pub(crate) fn claim(&self) -> Result<Arc<WorkerLease>, CheckpointError> {
        Arc::clone(&self.0)
            .try_acquire_owned()
            .map(|permit| Arc::new(WorkerLease { _permit: permit }))
            .map_err(|_busy_or_closed| CheckpointError::Busy)
    }
}

/// The tracker keeps polling actual work after cancellation; the RPC waiter may
/// return at its own deadline without joining this future or its filesystem I/O.
pub(super) async fn cancel_at<F: std::future::Future>(
    deadline: tokio::time::Instant,
    cancellation: &tokio_util::sync::CancellationToken,
    work: F,
) -> F::Output {
    tokio::pin!(work);
    tokio::select! {
        biased;
        output = &mut work => output,
        () = tokio::time::sleep_until(deadline) => {
            cancellation.cancel();
            work.await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::{capture, quota::ReservedBytes, runner};
    use vessel_proto::{
        EventSeq, SessionId, WorktreeId,
        checkpoint::{Boundary, CaptureGeneration, CaptureId, CaptureKey},
    };
    #[tokio::test(start_paused = true)]
    async fn checkpoint_owned_deadline_cancels_without_dropping_actual_work() {
        let cancellation = tokio_util::sync::CancellationToken::new();
        let owned = cancellation.clone();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let (entered, entry) = tokio::sync::oneshot::channel();
        let (release, held) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            cancel_at(deadline, &owned, async {
                entered.send(()).unwrap();
                held.await.unwrap();
                assert!(owned.is_cancelled());
            })
            .await;
        });
        entry.await.unwrap();
        tokio::time::advance(std::time::Duration::from_secs(30)).await;
        cancellation.cancelled().await;
        assert!(
            !task.is_finished(),
            "deadline must not drop held actual work"
        );
        release.send(()).unwrap();
        task.await.unwrap();
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn checkpoint_shared_worker_budget_survives_capture_owner_loss_during_blocked_io() {
        vessel_testkit::within(std::time::Duration::from_secs(60), async {
            let workers = Workers::default();
            let _collector = workers.claim().unwrap();
            let (release, held) = std::sync::mpsc::channel();
            let held = std::sync::Mutex::new(Some(held));
            let (entered, entry) = tokio::sync::oneshot::channel();
            let entered = std::sync::Mutex::new(Some(entered));
            let (client, owner) = runner::start_shared(
                workers.clone(),
                move |_key, _root, _cancel| {
                    entered.lock().unwrap().take().unwrap().send(()).unwrap();
                    held.lock().unwrap().take().unwrap().recv().unwrap();
                    Err(CheckpointError::CaptureTimedOut)
                },
                |_key, _phase| async { Ok(()) },
                |_key, _disposition| Ok(()),
            );
            let key = CaptureKey {
                session: SessionId::random(),
                worktree: WorktreeId::random(),
                capture: CaptureId::random(),
                generation: CaptureGeneration::FIRST,
                boundary: Boundary::BeforeTurn {
                    prompt: EventSeq::ZERO,
                },
            };
            let caller = tokio::spawn(async move {
                client
                    .capture_at_root(
                        capture::model_root(key.worktree),
                        key,
                        ReservedBytes::new(1).unwrap(),
                        tokio::time::Instant::now() + std::time::Duration::from_secs(30),
                    )
                    .await
            });
            entry.await.unwrap();
            assert!(matches!(workers.claim(), Err(CheckpointError::Busy)));
            // Only the test's pure model owner is aborted. The real blocking
            // closure retains the shared budget independently of that owner.
            owner.abort();
            assert!(owner.await.unwrap_err().is_cancelled());
            assert!(caller.await.unwrap().is_err());
            assert!(matches!(workers.claim(), Err(CheckpointError::Busy)));
            release.send(()).unwrap();
            let _returned = Arc::clone(&workers.0).acquire_owned().await.unwrap();
        })
        .await
        .unwrap();
    }
}
