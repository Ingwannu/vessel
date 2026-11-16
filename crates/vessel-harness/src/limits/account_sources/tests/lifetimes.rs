//! A failed phase must release the actual physical callback, not just its waiter.
use super::*;
use futures_util::FutureExt as _;

const RELEASE_FAILURE_POINT: &str = "exercise failure cleanup after physical entry";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_awaiters_does_not_release_admission_while_blocking_reads_run() {
    vessel_testkit::within(
        CASE_BUDGET,
        Box::pin(async {
            let root = tempfile::tempdir().unwrap();
            let gate = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
            let release = Release(Arc::clone(&gate));
            let entered = Arc::new(tokio::sync::Semaphore::new(0));
            let probe = Arc::new(gated_probe(root.path(), &gate, &entered));
            let context = LimitContext::new();
            let mut waiting = Vec::new();
            for _ in 0..2 {
                let probe = Arc::clone(&probe);
                let context = context.clone();
                waiting.push(tokio::spawn(async move {
                    probe
                        .read(
                            &context,
                            BuiltinUsageSource::Cursor,
                            Utc::now(),
                            &CancellationToken::new(),
                        )
                        .await
                }));
            }
            let started = entered
                .acquire_many(2)
                .phase("two physical credential readers entered")
                .await
                .unwrap();
            started.forget();
            for task in waiting {
                task.abort();
                let _cancelled = task.phase("usage phase: task").await;
            }
            assert!(matches!(context.admit(), Err(LimitFailure::Busy)));
            release.release();
            context.drain().phase("physical credential drain").await;
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_phase_releases_the_actual_reader_before_its_join_and_drain() {
    vessel_testkit::within(
        CASE_BUDGET,
        Box::pin(async {
            let root = tempfile::tempdir().unwrap();
            let gate = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
            let entered = Arc::new(tokio::sync::Semaphore::new(0));
            let context = LimitContext::new();
            let mut worker = None;
            let failed = std::panic::AssertUnwindSafe(async {
                let _release = Release(Arc::clone(&gate));
                let probe = gated_probe(root.path(), &gate, &entered);
                let context = context.clone();
                worker = Some(tokio::spawn(async move {
                    probe
                        .read(
                            &context,
                            BuiltinUsageSource::Cursor,
                            Utc::now(),
                            &CancellationToken::new(),
                        )
                        .await
                }));
                entered
                    .acquire()
                    .phase("one physical credential reader entered")
                    .await
                    .unwrap()
                    .forget();
                std::panic::panic_any(RELEASE_FAILURE_POINT);
            })
            .catch_unwind()
            .phase("deliberately failing reader phase")
            .await;
            let failure = failed.expect_err("the deliberate failure point must unwind");
            assert_eq!(failure.downcast_ref::<&str>(), Some(&RELEASE_FAILURE_POINT));
            let observed = worker
                .take()
                .unwrap()
                .phase("reader completion after failure cleanup")
                .await
                .unwrap();
            assert!(matches!(
                observed.result,
                Ok(AccountUsageOutcome::Available(_))
            ));
            context
                .drain()
                .phase("physical reader drain after failure cleanup")
                .await;
            assert!(context.admit().is_ok());
        }),
    )
    .await
    .unwrap();
}

fn gated_probe(
    root: &Path,
    gate: &Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
    entered: &Arc<tokio::sync::Semaphore>,
) -> AccountUsageProbe {
    write(
        root,
        ".config/Cursor/User/globalStorage/state.vscdb",
        b"fixture callback only",
    );
    let blocked = Arc::clone(gate);
    let notified = Arc::clone(entered);
    let access = access(root).with_cursor_reader(move |_path| {
        notified.add_permits(1);
        let (lock, changed) = &*blocked;
        let mut released = lock.lock().unwrap();
        while !*released {
            released = changed.wait(released).unwrap();
        }
        Ok(Some(zeroize::Zeroizing::new(
            "fixture-cursor-key".to_owned(),
        )))
    });
    recorded(root, BuiltinUsageSource::Cursor, access, b"{}")
}
