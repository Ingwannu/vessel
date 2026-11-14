//! Real file replacement paused on either side of the caller-cancellation gap.
use super::*;
use std::{sync::Mutex as SyncMutex, time::Duration};
use tokio::sync::oneshot;
use vessel_harness::{HarnessEnv, Lookup};
use vessel_proto::parking::IdleMinutes;
use vessel_session::{
    DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, DEFAULT_RING_CAPACITY, RuntimeConfig,
};

#[derive(Debug, Default)]
pub(super) struct Gates {
    write: SyncMutex<Option<WriteGate>>,
    apply: SyncMutex<Option<ApplyGate>>,
    cleanup: SyncMutex<Option<ApplyGate>>,
}
#[derive(Debug)]
pub(super) struct WriteGate {
    entered: oneshot::Sender<()>,
    release: std::sync::mpsc::Receiver<()>,
}
impl WriteGate {
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
    pub(super) fn arm_write(&self) -> (oneshot::Receiver<()>, std::sync::mpsc::SyncSender<()>) {
        let (entered, waiting) = oneshot::channel();
        let (release, held) = std::sync::mpsc::sync_channel(1);
        *self.write.lock().unwrap() = Some(WriteGate {
            entered,
            release: held,
        });
        (waiting, release)
    }
    pub(super) async fn before_cleanup(&self) {
        let gate = self.cleanup.lock().unwrap().take();
        if let Some(gate) = gate {
            let _receiver = gate.entered.send(());
            gate.release.await.unwrap();
        }
    }
    pub(super) fn arm_cleanup(&self) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered, waiting) = oneshot::channel();
        let (release, held) = oneshot::channel();
        *self.cleanup.lock().unwrap() = Some(ApplyGate {
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
    _dir: tempfile::TempDir,
    data: DataDir,
    settings: Arc<Settings>,
    tasks: TaskTracker,
    stop: CancellationToken,
}
impl Fixture {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let data = DataDir::new(dir.path());
        let stop = CancellationToken::new();
        let tasks = TaskTracker::new();
        let runtime = Arc::new(SessionRuntime::new(
            RuntimeConfig {
                harness_lookup: Lookup::new(Vec::new()),
                harness_env: HarnessEnv::new(dir.path().to_path_buf()),
                ring_capacity: DEFAULT_RING_CAPACITY,
                durable_capacity: DEFAULT_DURABLE_CAPACITY,
                agent_handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
                park_after: None,
                private_dir: None,
            },
            CancellationToken::new(),
        ));
        let settings = Arc::new(Settings::open(
            Opening {
                data_dir: data.clone(),
                store: Store::open(&data, vessel_store::StoreConfig::default())
                    .await
                    .unwrap()
                    .store,
                saved: read_saved(&data),
                policy: Arc::new(Policy::new(
                    vessel_proto::restart::RestartRecovery::default(),
                )),
            },
            runtime,
            ParkAfter::Never,
            false,
            &tasks,
            &stop,
        ));
        Self {
            _dir: dir,
            data,
            settings,
            tasks,
            stop,
        }
    }
    async fn finish(&self) {
        self.stop.cancel();
        self.settings.drain().await;
        let _closed = self.tasks.close();
        self.tasks.wait().await;
        self.settings.engine.runtime.shutdown().await;
    }
}
fn change(minutes: u16) -> SetNodeSettings {
    SetNodeSettings {
        park_after: Some(ParkAfter::Idle {
            minutes: IdleMinutes::new(minutes).unwrap(),
        }),
        ..SetNodeSettings::default()
    }
}

#[tokio::test]
async fn lost_caller_during_real_write_does_not_drop_apply_or_reorder_queued_changes() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let f = Fixture::new().await;
        let (entered, release) = f.settings.engine.gates.arm_write();
        let caller = {
            let settings = Arc::clone(&f.settings);
            tokio::spawn(async move { settings.change(change(1)).await })
        };
        entered.await.unwrap();
        assert_eq!(f.data.read_settings().unwrap(), None);
        assert_eq!(f.settings.engine.runtime.park_after(), None);
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
        let mut replies = Vec::new();
        for minutes in 2..=9 {
            let (reply, receive) = oneshot::channel();
            f.settings
                .changes
                .try_send(Change {
                    request: change(minutes),
                    reply,
                })
                .unwrap();
            replies.push((minutes, receive));
        }
        assert!(matches!(
            f.settings.change(change(10)).await,
            Err(SettingsError::Full)
        ));
        release.send(()).unwrap();
        for (minutes, receive) in replies {
            assert_eq!(
                receive.await.unwrap().unwrap().park_after,
                change(minutes).park_after.unwrap()
            );
        }
        assert_eq!(f.data.read_settings().unwrap(), Some(change(9)));
        assert_eq!(
            f.settings.current().await.park_after,
            change(9).park_after.unwrap()
        );
        assert_eq!(
            f.settings.engine.runtime.park_after(),
            Some(Duration::from_secs(9 * 60))
        );
        f.finish().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn shutdown_drains_apply_after_the_caller_dies_with_the_file_already_saved() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let f = Fixture::new().await;
        let (entered, release) = f.settings.engine.gates.arm_apply();
        let caller = {
            let settings = Arc::clone(&f.settings);
            tokio::spawn(async move { settings.change(change(15)).await })
        };
        entered.await.unwrap();
        assert_eq!(f.data.read_settings().unwrap(), Some(change(15)));
        assert_eq!(f.settings.engine.runtime.park_after(), None);
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
        f.stop.cancel();
        assert!(matches!(
            f.settings.change(change(3)).await,
            Err(SettingsError::Closed)
        ));
        release.send(()).unwrap();
        f.finish().await;
        assert_eq!(
            f.settings.current().await.park_after,
            change(15).park_after.unwrap()
        );
        assert_eq!(
            f.settings.engine.runtime.park_after(),
            Some(Duration::from_secs(15 * 60))
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn failed_file_replacement_changes_neither_saved_nor_runtime_values() {
    let f = Fixture::new().await;
    std::fs::create_dir(f.data.settings_path()).unwrap();
    assert!(matches!(
        f.settings.change(change(3)).await,
        Err(SettingsError::Write(_))
    ));
    assert_eq!(f.settings.current().await.park_after, ParkAfter::Never);
    assert_eq!(f.settings.engine.runtime.park_after(), None);
    f.finish().await;
}

mod recovery;
