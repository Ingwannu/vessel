//! Diagnostic source used by the bounded node collector (ADR 0074).
//! Source owners omit private paths, arbitrary text and credentials.
//! [Decision Log]
//! A caller deadline cannot stop a queued/running filesystem operation.
//! Its lease therefore travels into every source job. A task token acquired
//! before the stop check closes the admission-to-spawn shutdown gap; the
//! semaphore alone would let shutdown finish before those jobs are registered.
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};
use tokio_util::{
    sync::CancellationToken,
    task::{TaskTracker, task_tracker::TaskTrackerToken},
};

// Node-wide cap includes work whose caller was cancelled or timed out.
use vessel_proto::diagnostics::MAX_DIAGNOSTIC_COLLECTIONS as COLLECTION_PERMITS;

#[derive(Debug, thiserror::Error)]
pub(super) enum Refused {
    #[error("diagnostic collectors are busy (limit {limit})", limit = COLLECTION_PERMITS)]
    Busy(#[source] TryAcquireError),
    #[error("the node is stopping")]
    Stopping,
}

/// Every source job retains a clone until its blocking operation completes.
#[derive(Debug)]
pub(super) struct Lease {
    _task: TaskTrackerToken,
    _permit: OwnedSemaphorePermit,
}
#[derive(Debug)]
pub(super) struct Admission {
    slots: Arc<Semaphore>,
    stop: CancellationToken,
    tasks: TaskTracker,
}
impl Admission {
    pub(super) fn new(stop: &CancellationToken, tasks: &TaskTracker) -> Self {
        Self {
            slots: Arc::new(Semaphore::new(COLLECTION_PERMITS)),
            stop: stop.clone(),
            tasks: tasks.clone(),
        }
    }
    pub(super) fn admit(&self) -> Result<Arc<Lease>, Refused> {
        // Node shutdown cancels before closing/waiting on this same tracker.
        // If wait has already completed, this request sees stop and does no
        // work; otherwise the token keeps it pending through job registration.
        let task = self.tasks.token();
        if self.stop.is_cancelled() {
            return Err(Refused::Stopping);
        }
        let permit = Arc::clone(&self.slots)
            .try_acquire_owned()
            .map_err(Refused::Busy)?;
        if self.stop.is_cancelled() {
            return Err(Refused::Stopping);
        }
        Ok(Arc::new(Lease {
            _task: task,
            _permit: permit,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::super::deadline::{self, phase};
    use super::*;
    use std::{future::Future, pin::Pin, task::Poll};
    use tokio::sync::oneshot;

    async fn is_pending<F: Future>(mut future: Pin<&mut F>) -> bool {
        std::future::poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx).is_pending())).await
    }
    #[tokio::test]
    async fn clones_retain_capacity_and_refusal_names_the_bound() {
        let stop = CancellationToken::new();
        let tasks = TaskTracker::new();
        let admission = Admission::new(&stop, &tasks);
        let mut leases = Vec::new();
        for _ in 0..COLLECTION_PERMITS {
            leases.push(admission.admit().unwrap());
        }
        let held_by_work = Arc::clone(leases.first().unwrap());
        let error = admission.admit().unwrap_err();
        assert!(matches!(error, Refused::Busy(_)));
        assert!(error.to_string().contains(&COLLECTION_PERMITS.to_string()));
        leases.clear();
        let mut replacements = Vec::new();
        for _ in 0..COLLECTION_PERMITS.saturating_sub(1) {
            replacements.push(admission.admit().unwrap());
        }
        assert!(matches!(admission.admit(), Err(Refused::Busy(_))));
        drop(held_by_work);
        replacements.push(admission.admit().unwrap());
        drop(replacements);
        assert!(tasks.is_empty());
    }
    #[tokio::test]
    async fn admitted_before_spawn_keeps_shutdown_waiting() {
        deadline::run(async {
            let stop = CancellationToken::new();
            let tasks = TaskTracker::new();
            let admission = Admission::new(&stop, &tasks);
            // Admission is complete, but no source task has been spawned yet.
            let lease = admission.admit().unwrap();
            stop.cancel();
            assert!(tasks.close());
            let mut wait = Box::pin(tasks.wait());
            assert!(is_pending(wait.as_mut()).await);
            assert!(matches!(admission.admit(), Err(Refused::Stopping)));
            drop(lease);
            phase("shutdown tracker drains", wait).await;
            // Requests after a completed shutdown do not leave a tracker token.
            assert!(matches!(admission.admit(), Err(Refused::Stopping)));
            assert!(tasks.is_empty());
        })
        .await;
    }
    #[tokio::test]
    async fn caller_cancellation_does_not_release_running_work() {
        deadline::run(async {
            let stop = CancellationToken::new();
            let tasks = TaskTracker::new();
            let admission = Arc::new(Admission::new(&stop, &tasks));
            let (entered, ready) = oneshot::channel();
            let (release, finish) = oneshot::channel();
            let caller_admission = Arc::clone(&admission);
            let caller_tasks = tasks.clone();
            let caller = tokio::spawn(async move {
                let lease = caller_admission.admit().unwrap();
                let source_lease = Arc::clone(&lease);
                // The real tracked source task survives its caller's JoinHandle.
                let source = caller_tasks.spawn(async move {
                    let _lease = source_lease;
                    entered.send(()).unwrap();
                    phase("source is released", finish).await.unwrap();
                });
                phase("source task joins", source).await.unwrap();
                drop(lease);
            });
            phase("source enters", ready).await.unwrap();
            caller.abort();
            assert!(
                phase("aborted caller joins", caller)
                    .await
                    .unwrap_err()
                    .is_cancelled()
            );
            let mut others = Vec::new();
            for _ in 0..COLLECTION_PERMITS.saturating_sub(1) {
                others.push(admission.admit().unwrap());
            }
            assert!(matches!(admission.admit(), Err(Refused::Busy(_))));
            stop.cancel();
            assert!(tasks.close());
            let mut wait = Box::pin(tasks.wait());
            assert!(is_pending(wait.as_mut()).await);
            drop(others);
            release.send(()).unwrap();
            phase("shutdown tracker drains", wait).await;
        })
        .await;
    }
    #[test]
    fn queued_blocking_work_retains_its_lease_and_skips_io_after_stop() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .max_blocking_threads(1)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(deadline::run(async {
            let stop = CancellationToken::new();
            let tasks = TaskTracker::new();
            let admission = Arc::new(Admission::new(&stop, &tasks));
            let (entered, ready) = oneshot::channel();
            let (release, finish) = std::sync::mpsc::channel();
            // Occupy the actual blocking pool so the next operation queues.
            let blocker = tokio::task::spawn_blocking(move || {
                entered.send(()).unwrap();
                // The timed future owns release: cancellation closes this
                // channel too, so runtime destruction cannot retain this job.
                // A closed sender is cleanup, not a second test failure.
                let _released = finish.recv();
            });
            phase("source enters", ready).await.unwrap();
            let (queued, registered) = oneshot::channel();
            let io_executed = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let source_called = Arc::clone(&io_executed);
            let source_stop = stop.clone();
            let caller_admission = Arc::clone(&admission);
            let source_tasks = tasks.clone();
            let caller = tokio::spawn(async move {
                let lease = caller_admission.admit().unwrap();
                let source_lease = Arc::clone(&lease);
                let source = source_tasks.spawn(async move {
                    let work = tokio::task::spawn_blocking(move || {
                        let _lease = source_lease;
                        if !source_stop.is_cancelled() {
                            source_called.store(true, std::sync::atomic::Ordering::SeqCst);
                        }
                    });
                    queued.send(()).unwrap();
                    phase("blocking source joins", work).await.unwrap();
                });
                phase("source task joins", source).await.unwrap();
                drop(lease);
            });
            phase("blocking source is queued", registered)
                .await
                .unwrap();
            caller.abort();
            assert!(
                phase("aborted caller joins", caller)
                    .await
                    .unwrap_err()
                    .is_cancelled()
            );
            let mut others = Vec::new();
            for _ in 0..COLLECTION_PERMITS.saturating_sub(1) {
                others.push(admission.admit().unwrap());
            }
            assert!(matches!(admission.admit(), Err(Refused::Busy(_))));
            drop(others);
            stop.cancel();
            assert!(tasks.close());
            let mut wait = Box::pin(tasks.wait());
            assert!(is_pending(wait.as_mut()).await);
            release.send(()).unwrap();
            phase("pool blocker joins", blocker).await.unwrap();
            phase("shutdown tracker drains", wait).await;
            assert!(!io_executed.load(std::sync::atomic::Ordering::SeqCst));
        }));
    }
}
