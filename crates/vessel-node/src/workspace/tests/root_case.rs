//! Assertion deadlines never substitute for actual fixture-owner settlement.
use super::{Fixture, start};
use crate::workspace::WorkspaceError;
use futures_util::FutureExt as _;
use std::{future::Future, panic::AssertUnwindSafe, sync::mpsc, time::Duration};
use tokio::task::JoinHandle;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_store::{Store, StoreConfig};

const ROOT_CASE_TIMEOUT: Duration = Duration::from_secs(30);
const ROOT_PHASE_TIMEOUT: Duration = Duration::from_secs(5);

pub(in crate::workspace) async fn phase<T>(name: &str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(ROOT_PHASE_TIMEOUT, future)
        .await
        .unwrap_or_else(|_| panic!("root admission phase {name} exceeded {ROOT_PHASE_TIMEOUT:?}"))
}

/// Keep the very same cleanup future alive after its diagnostic deadline. Kernel
/// I/O cannot be aborted safely; a late receipt is still a test failure.
pub(in crate::workspace) async fn settle<T>(
    name: &str,
    future: impl Future<Output = T>,
) -> (T, bool) {
    let began = tokio::time::Instant::now();
    tokio::pin!(future);
    match tokio::time::timeout(ROOT_PHASE_TIMEOUT, &mut future).await {
        Ok(value) => (value, true),
        Err(_elapsed) => {
            tracing::error!(phase = name, limit = ?ROOT_PHASE_TIMEOUT, "root fixture phase deadline exceeded; retaining actual owner");
            let value = future.await;
            tracing::error!(phase = name, elapsed = ?began.elapsed(), "root fixture late owner actually settled");
            (value, false)
        }
    }
}

#[derive(Default)]
pub(super) struct Held {
    releases: Vec<mpsc::Sender<()>>,
    callers: Vec<Option<JoinHandle<Result<(), WorkspaceError>>>>,
}
impl Held {
    pub(super) fn retain(
        &mut self,
        release: mpsc::Sender<()>,
        caller: JoinHandle<Result<(), WorkspaceError>>,
    ) {
        self.releases.push(release);
        self.callers.push(Some(caller));
    }
    pub(super) fn release(&mut self) {
        for release in self.releases.drain(..) {
            let _gone = release.send(());
        }
    }
    pub(super) async fn abort_callers(&mut self) {
        for slot in &mut self.callers {
            if let Some(caller) = slot {
                caller.abort();
                assert!(
                    phase("caller close", caller)
                        .await
                        .unwrap_err()
                        .is_cancelled()
                );
                *slot = None;
            }
        }
    }
}

pub(super) async fn run(body: impl AsyncFnOnce(&Fixture, &mut Held)) {
    // Setup is retained outside the timed/caught assertions as well. If it is
    // late, finish constructing it so the real store/tracker can be closed.
    let began = tokio::time::Instant::now();
    let (temp, env, data) = Fixture::prepare();
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let (opened, store_on_time) =
        settle("store setup", Store::open(&data, StoreConfig::default())).await;
    let store = opened.unwrap().store;
    let (workspaces, workspace_on_time) = settle(
        "workspace setup",
        start(&temp, &data, &env, &store, &tasks, &stop),
    )
    .await;
    let fixture = Fixture {
        temp,
        env,
        data,
        store,
        tasks,
        stop,
        workspaces,
    };
    let mut held = Held::default();
    let outcome = AssertUnwindSafe(vessel_testkit::within(
        ROOT_CASE_TIMEOUT.saturating_sub(began.elapsed()),
        async {
            assert!(
                store_on_time && workspace_on_time,
                "root fixture phase exceeded {ROOT_PHASE_TIMEOUT:?}: store={store_on_time}, workspace={workspace_on_time}"
            );
            body(&fixture, &mut held).await;
        },
    ))
    .catch_unwind()
    .await;
    // Release the SAME barriers even after assertion panic or case timeout.
    held.release();
    fixture.stop.cancel();
    let mut callers_settled = true;
    for caller in held.callers.into_iter().flatten() {
        let (joined, on_time) = settle("caller close", caller).await;
        callers_settled &= on_time
            && match joined {
                Ok(result) => result.is_ok(),
                Err(error) => error.is_cancelled(),
            };
    }
    let _closed = fixture.tasks.close();
    let ((), drained) = settle("tracker drain", fixture.tasks.wait()).await;
    let ((), store_closed) = settle("store close", fixture.store.close()).await;
    match outcome {
        Ok(result) => result.unwrap(),
        Err(panic) => std::panic::resume_unwind(panic),
    }
    assert!(
        callers_settled,
        "caller close phase failed or exceeded {ROOT_PHASE_TIMEOUT:?}"
    );
    assert!(
        drained,
        "tracker drain phase exceeded {ROOT_PHASE_TIMEOUT:?}"
    );
    assert!(
        store_closed,
        "store close phase exceeded {ROOT_PHASE_TIMEOUT:?}"
    );
}
