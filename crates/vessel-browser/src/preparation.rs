//! Finite ownership of actual blocking preparation, including after caller cancellation.
//!
//! A timeout can stop waiting for getaddrinfo/getifaddrs, not their OS work.
//! Both the process-wide permit and the profile tracker token therefore live
//! inside the actual closure, through queued/started execution and result drop.
//! A10 staged with the browser actor; its product wiring commit must preserve
//! this owner instead of spawning another untracked resolver/interface task.

use std::sync::{Arc, OnceLock};

use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

const MAX_PREPARATION_WORKERS: usize = 4;
static GLOBAL_PREPARATION: OnceLock<Arc<Semaphore>> = OnceLock::new();

#[derive(Debug, thiserror::Error)]
pub(crate) enum PreparationError {
    #[error("browser preparation capacity is occupied")]
    Busy,
    #[error("browser profile preparation is closed")]
    Closed,
    #[error("browser preparation task failed")]
    Task(#[source] tokio::task::JoinError),
}

/// One profile's work scope shares a finite process-wide blocking budget.
#[derive(Clone)]
pub(crate) struct Preparation {
    gate: Arc<Semaphore>,
    tasks: TaskTracker,
    cancel: CancellationToken,
    registration: Arc<Mutex<Admission>>,
}
enum Admission {
    Open,
    Closed,
}
impl Preparation {
    pub(crate) fn for_profile(cancel: CancellationToken) -> Self {
        let gate =
            GLOBAL_PREPARATION.get_or_init(|| Arc::new(Semaphore::new(MAX_PREPARATION_WORKERS)));
        Self::with_gate(cancel, Arc::clone(gate))
    }

    pub(crate) fn with_gate(cancel: CancellationToken, gate: Arc<Semaphore>) -> Self {
        Self {
            gate,
            tasks: TaskTracker::new(),
            cancel,
            registration: Arc::new(Mutex::new(Admission::Open)),
        }
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    pub(crate) async fn run<F, R>(&self, work: F) -> Result<R, PreparationError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        if self.is_cancelled() {
            return Err(PreparationError::Closed);
        }
        let task = {
            // Registration and tracker closing must serialize: close cannot
            // observe empty and return just before an admitted job is tracked.
            // No guard is retained across OS work or any later await.
            let registration = self.registration.lock().await;
            if matches!(*registration, Admission::Closed) || self.is_cancelled() {
                return Err(PreparationError::Closed);
            }
            let permit =
                Arc::clone(&self.gate)
                    .try_acquire_owned()
                    .map_err(|error| match error {
                        tokio::sync::TryAcquireError::Closed => PreparationError::Closed,
                        tokio::sync::TryAcquireError::NoPermits => PreparationError::Busy,
                    })?;
            let cancel = self.cancel.clone();
            // The tracker token and permit live in the actual OS-work closure,
            // including when its caller drops the detached JoinHandle.
            self.tasks.spawn_blocking(move || {
                let _permit = permit;
                if cancel.is_cancelled() {
                    Err(PreparationError::Closed)
                } else {
                    Ok(work())
                }
            })
        };
        let result = tokio::select! {
            ()=self.cancel.cancelled()=>return Err(PreparationError::Closed),
            result=task=>result.map_err(PreparationError::Task)??,
        };
        if self.is_cancelled() {
            return Err(PreparationError::Closed);
        }
        Ok(result)
    }

    pub(crate) async fn finish(&self) {
        self.cancel.cancel();
        {
            let mut registration = self.registration.lock().await;
            *registration = Admission::Closed;
            let _closed = self.tasks.close();
        }
        self.tasks.wait().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc as blocking;
    use std::time::Duration;
    use tokio::sync::oneshot;

    const FIXTURE_DEADLINE: Duration = Duration::from_secs(20);
    const CALLER_DEADLINE: Duration = Duration::from_millis(1);

    // Exercise the common owner used by both real getaddrinfo and getifaddrs.
    // This does not induce an OS hang; the barrier makes actual closure lifetime explicit.
    async fn held_preparation_keeps_capacity(cancel_profile: bool) {
        let gate = Arc::new(Semaphore::new(1));
        let cancel = CancellationToken::new();
        let scope = Preparation::with_gate(cancel.clone(), Arc::clone(&gate));
        let caller_scope = scope.clone();
        let (started, entered) = oneshot::channel();
        let (release, held) = blocking::channel();
        let caller = tokio::spawn(async move {
            caller_scope
                .run(move || {
                    let _gone = started.send(());
                    held.recv().unwrap();
                })
                .await
        });
        entered.await.unwrap();
        if cancel_profile {
            cancel.cancel();
            assert!(matches!(
                caller.await.unwrap(),
                Err(PreparationError::Closed)
            ));
        } else {
            caller.abort();
            assert!(caller.await.unwrap_err().is_cancelled());
        }
        let replacement = Preparation::with_gate(CancellationToken::new(), Arc::clone(&gate));
        assert!(matches!(
            replacement.run(|| ()).await,
            Err(PreparationError::Busy)
        ));
        let finishing_scope = scope.clone();
        let finishing = tokio::spawn(async move { finishing_scope.finish().await });
        assert_eq!(gate.available_permits(), 0);
        assert!(!finishing.is_finished());
        release.send(()).unwrap();
        finishing.await.unwrap();
        assert_eq!(gate.available_permits(), 1);
        replacement.run(|| ()).await.unwrap();
        replacement.finish().await;
    }

    #[tokio::test]
    async fn cancelled_resolver_waiter_does_not_reuse_its_actual_worker_slot() {
        tokio::time::timeout(FIXTURE_DEADLINE, held_preparation_keeps_capacity(false))
            .await
            .unwrap();
    }
    #[tokio::test]
    async fn closed_profile_waits_for_held_interface_work_and_blocks_replacement() {
        tokio::time::timeout(FIXTURE_DEADLINE, held_preparation_keeps_capacity(true))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn timed_out_preparation_never_publishes_a_late_result_or_releases_a_live_worker() {
        tokio::time::timeout(FIXTURE_DEADLINE, async {
            let gate = Arc::new(Semaphore::new(1));
            let scope = Preparation::with_gate(CancellationToken::new(), Arc::clone(&gate));
            let caller_scope = scope.clone();
            let (started, entered) = oneshot::channel();
            let (release, held) = blocking::channel();
            let published = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let observer = Arc::clone(&published);
            let caller = tokio::spawn(async move {
                tokio::time::timeout(CALLER_DEADLINE, async {
                    caller_scope
                        .run(move || {
                            let _gone = started.send(());
                            held.recv().unwrap();
                        })
                        .await
                        .unwrap();
                    // A socket publication/dial may happen only after the awaited preparation.
                    observer.store(true, std::sync::atomic::Ordering::Release);
                })
                .await
            });
            entered.await.unwrap();
            assert!(caller.await.unwrap().is_err());
            assert_eq!(gate.available_permits(), 0);
            assert!(matches!(
                scope.run(|| ()).await,
                Err(PreparationError::Busy)
            ));
            release.send(()).unwrap();
            scope.finish().await;
            assert_eq!(gate.available_permits(), 1);
            assert!(!published.load(std::sync::atomic::Ordering::Acquire));
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn close_cannot_miss_a_registration_waiting_at_its_boundary() {
        tokio::time::timeout(FIXTURE_DEADLINE, async {
            let gate = Arc::new(Semaphore::new(1));
            let scope = Preparation::with_gate(CancellationToken::new(), Arc::clone(&gate));
            let registration = scope.registration.lock().await;
            let used = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let marker = Arc::clone(&used);
            let caller_scope = scope.clone();
            let (started, entered) = oneshot::channel();
            let caller = tokio::spawn(async move {
                let _gone = started.send(());
                caller_scope
                    .run(move || marker.store(true, std::sync::atomic::Ordering::Release))
                    .await
            });
            entered.await.unwrap();
            let finish_scope = scope.clone();
            let closing = tokio::spawn(async move { finish_scope.finish().await });
            scope.cancel.cancelled().await;
            drop(registration);
            assert!(matches!(
                caller.await.unwrap(),
                Err(PreparationError::Closed)
            ));
            closing.await.unwrap();
            assert!(!used.load(std::sync::atomic::Ordering::Acquire));
            assert_eq!(gate.available_permits(), 1);
        })
        .await
        .unwrap();
    }
}
