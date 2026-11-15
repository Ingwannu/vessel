//! Real retained admission, including blocking work and queued completion.

use super::*;
use std::{future::Future, time::Duration};

const PHASE: Duration = Duration::from_secs(5);
const SAFETY: Duration = Duration::from_secs(20);
async fn phase<T>(name: &str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(PHASE, future)
        .await
        .unwrap_or_else(|_| panic!("phase timed out: {name}"))
}

#[tokio::test]
async fn cancellation_retains_all_blocking_slots_while_apply_stays_independent() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            let tasks = TaskTracker::new();
            let stopping = CancellationToken::new();
            let jobs = Arc::new(LaunchProfileJobs::new(tasks.clone(), stopping.clone()));
            let (started, mut running) = tokio::sync::mpsc::channel(LaunchProfileJobs::MAX_ACTIVE);
            // These release senders belong to the timed test body. If a phase
            // fails, dropping them releases every blocking receiver before the
            // test runtime can wait for its blocking pool to shut down.
            let mut releases = Vec::new();
            let mut callers = Vec::new();
            for _ in 0..LaunchProfileJobs::MAX_ACTIVE {
                let (release, held) = std::sync::mpsc::channel();
                releases.push(release);
                let jobs = Arc::clone(&jobs);
                let started = started.clone();
                callers.push(tokio::spawn(async move {
                    jobs.run(Lane::Metadata, move || async move {
                        let result = tokio::task::spawn_blocking(move || {
                            started.blocking_send(()).unwrap();
                            held.recv()
                        })
                        .await
                        .map_err(ServiceError::Task)?;
                        if result.is_err() {
                            return Err(ServiceError::Draining);
                        }
                        Ok(())
                    })
                    .await
                }));
            }
            drop(started);
            for _ in 0..LaunchProfileJobs::MAX_ACTIVE {
                phase("blocking work admitted", running.recv())
                    .await
                    .unwrap();
            }
            for caller in callers {
                caller.abort();
                assert!(
                    phase("cancelled caller exits", caller)
                        .await
                        .unwrap_err()
                        .is_cancelled()
                );
            }
            assert_eq!(jobs.metadata.available_permits(), 0);
            assert!(matches!(
                phase(
                    "metadata admission refusal",
                    jobs.run(Lane::Metadata, || async { Ok(()) })
                )
                .await,
                Err(ServiceError::LaunchBusy(_))
            ));
            assert!(
                phase(
                    "independent apply admission",
                    jobs.run(Lane::Apply, || async { Ok(()) })
                )
                .await
                .is_ok()
            );
            for release in releases {
                release.send(()).unwrap();
            }
            stopping.cancel();
            let _closed = tasks.close();
            phase("actual blocking work and cleanup drain", tasks.wait()).await;
            assert_eq!(
                jobs.metadata.available_permits(),
                LaunchProfileJobs::MAX_ACTIVE
            );
            assert!(matches!(
                phase(
                    "metadata admission refusal",
                    jobs.run(Lane::Metadata, || async { Ok(()) })
                )
                .await,
                Err(ServiceError::Draining)
            ));
        }),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn completion_channels_retain_admission_until_consumed_or_dropped() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            let tasks = TaskTracker::new();
            let jobs = LaunchProfileJobs::new(tasks.clone(), CancellationToken::new());
            let mut answers = Vec::new();
            for _ in 0..LaunchProfileJobs::MAX_ACTIVE {
                let mut answer = Box::pin(jobs.run(Lane::Metadata, || async { Ok(()) }));
                assert!(futures_util::poll!(&mut answer).is_pending());
                answers.push(answer);
            }
            let _closed = tasks.close();
            phase("queued completions produced", tasks.wait()).await;
            assert_eq!(jobs.metadata.available_permits(), 0);
            assert!(matches!(
                phase(
                    "metadata admission refusal",
                    jobs.run(Lane::Metadata, || async { Ok(()) })
                )
                .await,
                Err(ServiceError::LaunchBusy(_))
            ));
            drop(answers);
            assert_eq!(
                jobs.metadata.available_permits(),
                LaunchProfileJobs::MAX_ACTIVE
            );
        }),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn admitted_work_is_tracked_before_shutdown_can_observe_an_empty_tracker() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            let tasks = TaskTracker::new();
            let stopping = CancellationToken::new();
            let jobs = LaunchProfileJobs::new(tasks.clone(), stopping.clone());
            let admission = jobs.admit(Lane::Metadata).unwrap();
            stopping.cancel();
            let _closed = tasks.close();
            let mut drained = Box::pin(tasks.wait());
            assert!(futures_util::poll!(&mut drained).is_pending());
            assert!(matches!(
                jobs.admit(Lane::Apply),
                Err(ServiceError::Draining)
            ));
            drop(admission);
            phase("pre-spawn admission releases teardown", drained).await;
        }),
    )
    .await
    .unwrap();
}
