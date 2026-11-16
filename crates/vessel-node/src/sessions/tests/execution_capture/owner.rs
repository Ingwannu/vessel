//! One retained fixture owner survives assertion/setup observer loss and drains
//! actual runtime tasks before closing the Store. Signals follow real settlement.
use super::{Arc, Duration, Fixture};
use std::future::Future;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

const PHASE_BOUND: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy)]
pub(super) enum Phase {
    Setup,
    Create,
    Find,
    Ready,
    Invalidation,
    RetiredWatch,
    EndedHandle,
    RuntimeDrain,
    TaskDrain,
    StoreClose,
    OwnerJoin,
}

pub(super) async fn step<F: Future>(phase: Phase, future: F) -> F::Output {
    tokio::time::timeout(PHASE_BOUND, future)
        .await
        .unwrap_or_else(|source| panic!("capture phase {phase:?}: {source}"))
}

pub(super) struct OwnedFixture {
    fixture: Arc<Fixture>,
    finish: oneshot::Sender<()>,
    runtime_done: oneshot::Receiver<()>,
    tasks_done: oneshot::Receiver<()>,
    store_done: oneshot::Receiver<()>,
    task: JoinHandle<()>,
}

impl OwnedFixture {
    pub(super) async fn new() -> Self {
        let (ready, prepared) = oneshot::channel();
        let (finish, finished) = oneshot::channel();
        let (runtime_sent, runtime_done) = oneshot::channel();
        let (tasks_sent, tasks_done) = oneshot::channel();
        let (store_sent, store_done) = oneshot::channel();
        // Dropping an observer or the finish sender cannot cancel this owner.
        // Its Arc retains the actual runtime, tracker, Store and temporary homes.
        let task = tokio::spawn(async move {
            let fixture = Arc::new(Fixture::new().await);
            if ready.send(Arc::clone(&fixture)).is_ok() {
                let _finished_or_lost = finished.await;
            }
            fixture.runtime.shutdown().await;
            let _observer_gone = runtime_sent.send(());
            fixture.settle().await;
            let _observer_gone = tasks_sent.send(());
            fixture.store.close().await;
            let _observer_gone = store_sent.send(());
        });
        let fixture = step(Phase::Setup, prepared).await.unwrap();
        Self {
            fixture,
            finish,
            runtime_done,
            tasks_done,
            store_done,
            task,
        }
    }

    pub(super) fn get(&self) -> &Fixture {
        &self.fixture
    }

    pub(super) async fn finish(mut self) {
        drop(self.finish);
        let mut failures = Vec::new();
        for (phase, settled) in [
            (Phase::RuntimeDrain, self.runtime_done),
            (Phase::TaskDrain, self.tasks_done),
            (Phase::StoreClose, self.store_done),
        ] {
            let outcome = tokio::time::timeout(PHASE_BOUND, settled).await;
            if !matches!(outcome, Ok(Ok(()))) {
                failures.push((phase, outcome));
            }
        }
        let joined = tokio::time::timeout(PHASE_BOUND, &mut self.task).await;
        assert!(
            failures.is_empty() && matches!(joined, Ok(Ok(()))),
            "capture cleanup failed: {failures:?}; {:?}: {joined:?}; the retained owner was not aborted and no physical release is inferred from a deadline",
            Phase::OwnerJoin,
        );
    }
}
