//! Real atomic file writes and runtime apply, with the reply future removed.
use super::*;
use std::{future::Future, sync::Mutex as SyncMutex, time::Duration};
use tokio::sync::oneshot;
use vessel_harness::{HarnessEnv, Lookup};
use vessel_proto::parking::IdleMinutes;
use vessel_session::{
    DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_RING_CAPACITY, RuntimeConfig,
};
use vessel_store::DataDir;

const SAFETY_NET: Duration = Duration::from_secs(60);
const STEP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Default)]
pub(super) struct Gates {
    write: SyncMutex<Option<WriteGate>>,
    apply: SyncMutex<Option<ApplyGate>>,
}
#[derive(Debug)]
pub(super) struct WriteGate {
    entered: oneshot::Sender<()>,
    release: std::sync::mpsc::Receiver<()>,
}
impl WriteGate {
    const RELEASE_CAPACITY: usize = 1;
    pub(super) fn wait(self) {
        let _receiver = self.entered.send(());
        self.release.recv().unwrap();
    }
}
#[derive(Debug)]
struct ApplyGate {
    entered: oneshot::Sender<()>,
    release: oneshot::Receiver<()>,
}
impl Gates {
    pub(super) fn write(&self) -> Option<WriteGate> {
        self.write.lock().unwrap().take()
    }
    pub(super) async fn before_apply(&self) {
        let gate = self.apply.lock().unwrap().take();
        if let Some(gate) = gate {
            let _receiver = gate.entered.send(());
            gate.release.await.unwrap();
        }
    }
    fn arm_write(&self) -> (oneshot::Receiver<()>, std::sync::mpsc::SyncSender<()>) {
        let (entered, waiting) = oneshot::channel();
        let (release, held) = std::sync::mpsc::sync_channel(WriteGate::RELEASE_CAPACITY);
        *self.write.lock().unwrap() = Some(WriteGate {
            entered,
            release: held,
        });
        (waiting, release)
    }
    fn arm_apply(&self) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered, waiting) = oneshot::channel();
        let (release, held) = oneshot::channel();
        *self.apply.lock().unwrap() = Some(ApplyGate {
            entered,
            release: held,
        });
        (waiting, release)
    }
}

struct Fixture {
    _directory: tempfile::TempDir,
    data: DataDir,
    runtime: Arc<SessionRuntime>,
    settings: Arc<Settings>,
    tasks: TaskTracker,
    stop: CancellationToken,
}
impl Fixture {
    async fn new(saved: Option<SavedNodeSettings>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let data = DataDir::new(directory.path().join("data"));
        data.prepare().unwrap();
        if let Some(saved) = saved {
            data.write_settings_record(&saved).unwrap();
        }
        let mut config = crate::config::Config::new(
            data.clone(),
            HarnessEnv::new(directory.path().join("home")),
        );
        // A distinct desired value makes lost runtime apply observable.
        config.park_after = None;
        config.park_after_given = false;
        let runtime = Arc::new(SessionRuntime::new(
            RuntimeConfig {
                harness_lookup: Lookup::new(Vec::new()),
                harness_env: config.harness_env.clone(),
                ring_capacity: DEFAULT_RING_CAPACITY,
                durable_capacity: DEFAULT_DURABLE_CAPACITY,
                agent_handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
                park_after: None,
                private_dir: None,
            },
            CancellationToken::new(),
        ));
        let tasks = TaskTracker::new();
        let stop = CancellationToken::new();
        let opening = Opening::read(&config).await.unwrap();
        let settings = Arc::new(Settings::open(opening, Arc::clone(&runtime), &tasks, &stop));
        Self {
            _directory: directory,
            data,
            runtime,
            settings,
            tasks,
            stop,
        }
    }
    async fn finish(self) {
        self.settings.drain().await;
        self.runtime.shutdown().await;
        self.stop.cancel();
        let _closed = self.tasks.close();
        self.tasks.wait().await;
    }
}
fn step<T>(label: &str, future: impl Future<Output = T>) -> impl Future<Output = T> {
    let future = Box::pin(future);
    async move {
        tokio::time::timeout(STEP_TIMEOUT, future)
            .await
            .unwrap_or_else(|_| panic!("settings step timed out: {label}"))
    }
}
fn idle() -> ParkAfter {
    ParkAfter::Idle {
        minutes: IdleMinutes::new(5).unwrap(),
    }
}
fn patch(after: ParkAfter) -> SetNodeSettings {
    SetNodeSettings {
        park_after: Some(after),
        drivers: None,
    }
}

mod cancellation;
mod legacy;
mod queue;

fn within<T>(future: impl Future<Output = T>) -> impl Future<Output = T> {
    let future = Box::pin(future);
    async move {
        vessel_testkit::within(SAFETY_NET, future)
            .await
            .expect("settings test timed out")
    }
}
