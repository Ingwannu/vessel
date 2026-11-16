//! Assertion deadlines never substitute for actual fixture-owner settlement.
use super::{Fixture, start};
use crate::workspace::WorkspaceError;
use futures_util::FutureExt as _;
use std::{ffi::OsString, future::Future, panic::AssertUnwindSafe, sync::mpsc, time::Duration};
use tokio::task::JoinHandle;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_store::{DataDir, Store, StoreConfig};

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

/// Partial setup owns its store/tracker/temp even if startup itself panics.
/// The caught assertion future never owns these resources or their cleanup.
struct Owners {
    temp: Option<tempfile::TempDir>,
    env: Vec<(String, OsString)>,
    data: DataDir,
    store: Option<Store>,
    tasks: TaskTracker,
    stop: CancellationToken,
    fixture: Option<Fixture>,
}
impl Owners {
    fn new() -> Self {
        let (temp, env, data) = Fixture::prepare();
        Self {
            temp: Some(temp),
            env,
            data,
            store: None,
            tasks: TaskTracker::new(),
            stop: CancellationToken::new(),
            fixture: None,
        }
    }
    async fn prepare(&mut self) -> (bool, bool) {
        let (opened, store_on_time) = settle(
            "store setup",
            Store::open(&self.data, StoreConfig::default()),
        )
        .await;
        self.store = Some(opened.unwrap().store);
        let store = self.store.as_ref().unwrap();
        let (workspaces, workspace_on_time) = settle(
            "workspace setup",
            start(
                self.temp.as_ref().unwrap(),
                &self.data,
                &self.env,
                store,
                &self.tasks,
                &self.stop,
            ),
        )
        .await;
        // No fallible/async work between moving the temp and retaining Fixture.
        self.fixture = Some(Fixture {
            temp: self.temp.take().unwrap(),
            env: self.env.clone(),
            data: self.data.clone(),
            store: store.clone(),
            tasks: self.tasks.clone(),
            stop: self.stop.clone(),
            workspaces,
        });
        (store_on_time, workspace_on_time)
    }
    async fn cleanup(&self, mut held: Held) -> (bool, bool, bool) {
        held.release();
        self.stop.cancel();
        let mut callers_settled = true;
        for caller in held.callers.into_iter().flatten() {
            let (joined, on_time) = settle("caller close", caller).await;
            callers_settled &= on_time
                && match joined {
                    Ok(result) => result.is_ok(),
                    Err(error) => error.is_cancelled(),
                };
        }
        let _closed = self.tasks.close();
        let ((), drained) = settle("tracker drain", self.tasks.wait()).await;
        let store_closed = match &self.store {
            Some(store) => settle("store close", store.close()).await.1,
            None => true,
        };
        (callers_settled, drained, store_closed)
    }
}

pub(super) async fn run(body: impl AsyncFnOnce(&Fixture, &mut Held)) {
    let began = tokio::time::Instant::now();
    let mut owners = Owners::new();
    let mut held = Held::default();
    // Catch setup too: a failed workspace start after Store::open must still
    // close that actual writer. Never drop an in-flight setup I/O future merely
    // because its diagnostic deadline passed.
    let setup = AssertUnwindSafe(owners.prepare()).catch_unwind().await;
    let outcome = match setup {
        Ok((store_on_time, workspace_on_time)) => AssertUnwindSafe(vessel_testkit::within(
            ROOT_CASE_TIMEOUT.saturating_sub(began.elapsed()), async {
                assert!(store_on_time && workspace_on_time,
                    "root fixture phase exceeded {ROOT_PHASE_TIMEOUT:?}: store={store_on_time}, workspace={workspace_on_time}");
                body(owners.fixture.as_ref().unwrap(), &mut held).await;
            })).catch_unwind().await,
        Err(panic) => Err(panic),
    };
    let (callers_settled, drained, store_closed) = owners.cleanup(held).await;
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
