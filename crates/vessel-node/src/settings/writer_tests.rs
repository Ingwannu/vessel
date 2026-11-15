//! Pause the real blocking writer, rather than a fake or the RPC before spawn.

use std::sync::Mutex as StdMutex;
use std::task::Poll;
use std::time::Duration;

use tokio::sync::oneshot;

use super::*;

pub(super) const WRITE_TEST_BUDGET: Duration = Duration::from_secs(10);

#[derive(Debug, Default)]
pub(crate) struct GateSlot(StdMutex<Option<Gate>>);

impl GateSlot {
    pub(crate) fn take(&self) -> Option<Gate> {
        self.0.lock().unwrap().take()
    }

    pub(crate) fn install(&self) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered, observing) = oneshot::channel();
        let (release, released) = oneshot::channel();
        *self.0.lock().unwrap() = Some(Gate { entered, released });
        (observing, release)
    }
}

#[derive(Debug)]
pub(crate) struct Gate {
    entered: oneshot::Sender<()>,
    released: oneshot::Receiver<()>,
}

impl Gate {
    pub(crate) async fn wait(self) {
        self.entered.send(()).unwrap();
        self.released.await.unwrap();
    }

    pub(super) fn pause(self) {
        self.entered.send(()).unwrap();
        // This runs inside spawn_blocking, before the actual file replacement.
        self.released.blocking_recv().unwrap();
    }
}

pub(super) fn runtime(root: &std::path::Path) -> Arc<SessionRuntime> {
    runtime_with_stop(root, tokio_util::sync::CancellationToken::new())
}

pub(super) fn runtime_with_stop(
    root: &std::path::Path,
    stop: tokio_util::sync::CancellationToken,
) -> Arc<SessionRuntime> {
    let runtime = SessionRuntime::new(
        vessel_session::RuntimeConfig {
            harness_lookup: vessel_harness::Lookup::new(std::iter::empty::<std::path::PathBuf>()),
            harness_env: vessel_harness::HarnessEnv::new(root.to_owned()),
            ring_capacity: vessel_session::DEFAULT_RING_CAPACITY,
            durable_capacity: vessel_session::DEFAULT_DURABLE_CAPACITY,
            agent_handshake_timeout: vessel_session::DEFAULT_HANDSHAKE_TIMEOUT,
            park_after: ParkAfter::DEFAULT.duration(),
            private_dir: None,
        },
        stop,
    );
    Arc::new(runtime)
}

pub(super) async fn settings(root: &std::path::Path) -> Arc<Settings> {
    Arc::new(Settings::open(DataDir::new(root), runtime(root), ParkAfter::DEFAULT, false).await)
}

pub(super) fn patch(park_after: ParkAfter) -> SetNodeSettings {
    SetNodeSettings {
        park_after: Some(park_after),
        ..SetNodeSettings::default()
    }
}

#[derive(Clone, Copy)]
pub(super) enum RecoveryChoice {
    On,
    Off,
}

pub(super) fn recovery(choice: RecoveryChoice) -> SetNodeSettings {
    SetNodeSettings {
        restart_recovery: Some(vessel_proto::restart::SetRestartRecovery {
            enabled: Some(match choice {
                RecoveryChoice::On => true,
                RecoveryChoice::Off => false,
            }),
            prompt: None,
        }),
        ..SetNodeSettings::default()
    }
}

#[tokio::test]
async fn cancelled_caller_cannot_release_the_real_writer_or_reorder_application() {
    let result = tokio::time::timeout(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let settings = settings(root.path()).await;
        let (entered, release) = settings.state.gate.install();
        let first = Arc::clone(&settings);
        let caller = tokio::spawn(async move { first.change(patch(ParkAfter::Never)).await });
        entered.await.unwrap();
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());

        // This is false with the old RPC-owned guard: cancellation lets the
        // next write overtake the still-paused real filesystem replacement.
        assert!(settings.state.saved.try_lock().is_err());
        assert_eq!(
            settings.state.runtime.park_after(),
            ParkAfter::DEFAULT.duration()
        );
        let mut second = Box::pin(settings.change(patch(ParkAfter::DEFAULT)));
        assert!(matches!(futures_util::poll!(&mut second), Poll::Pending));
        release.send(()).unwrap();
        let applied = second.await.unwrap();
        settings.shutdown().await;
        assert_eq!(applied.park_after, ParkAfter::DEFAULT);
        assert_eq!(settings.current().await, applied);
        assert_eq!(
            settings.state.runtime.park_after(),
            applied.park_after.duration()
        );
        assert_eq!(
            settings
                .state
                .data_dir
                .read_settings()
                .unwrap()
                .unwrap()
                .public_settings()
                .park_after,
            Some(applied.park_after)
        );
    })
    .await;
    assert!(
        result.is_ok(),
        "cancelled settings writer exceeded {WRITE_TEST_BUDGET:?}"
    );
}

#[tokio::test]
async fn shutdown_joins_cancelled_writers_and_refuses_late_mutations() {
    let result = tokio::time::timeout(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let settings = settings(root.path()).await;
        let (entered, release) = settings.state.gate.install();
        let first = Arc::clone(&settings);
        let caller = tokio::spawn(async move { first.change(patch(ParkAfter::Never)).await });
        entered.await.unwrap();
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
        let mut shutdown = Box::pin(settings.shutdown());
        assert!(matches!(futures_util::poll!(&mut shutdown), Poll::Pending));
        release.send(()).unwrap();
        shutdown.await;
        assert_eq!(settings.current().await.park_after, ParkAfter::Never);
        assert_eq!(settings.state.runtime.park_after(), None);
        assert_eq!(
            settings
                .state
                .data_dir
                .read_settings()
                .unwrap()
                .unwrap()
                .public_settings()
                .park_after,
            Some(ParkAfter::Never)
        );
        assert!(matches!(
            settings.change(patch(ParkAfter::DEFAULT)).await,
            Err(SettingsError::Stopping)
        ));
        assert_eq!(settings.current().await.park_after, ParkAfter::Never);
    })
    .await;
    assert!(
        result.is_ok(),
        "settings shutdown exceeded {WRITE_TEST_BUDGET:?}"
    );
}

#[tokio::test]
async fn a_noop_keeps_original_file_bytes_and_future_fields_in_place() {
    let result = tokio::time::timeout(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let path = DataDir::new(root.path()).settings_path();
        let bytes = br#"{ "park_after" : {"kind":"never"}, "future" : {"keep":"canary"} }"#;
        std::fs::write(&path, bytes).unwrap();
        let settings = settings(root.path()).await;
        let before = settings.current().await;
        let after = settings.change(patch(ParkAfter::Never)).await.unwrap();
        settings.shutdown().await;
        assert_eq!(after, before);
        assert_eq!(
            settings.state.runtime.park_after(),
            before.park_after.duration()
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let files = std::fs::read_dir(root.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(files, vec![path]);
    })
    .await;
    assert!(
        result.is_ok(),
        "saved settings no-op exceeded {WRITE_TEST_BUDGET:?}"
    );
}
