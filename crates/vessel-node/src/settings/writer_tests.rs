//! Pause the real blocking writer, rather than a fake or the RPC before spawn.

use std::sync::Mutex as StdMutex;
use std::task::Poll;
use std::time::Duration;

use tokio::sync::oneshot;

use super::*;

const WRITE_TEST_BUDGET: Duration = Duration::from_secs(10);

#[derive(Debug, Default)]
pub(super) struct GateSlot(StdMutex<Option<Gate>>);

impl GateSlot {
    pub(super) fn take(&self) -> Option<Gate> {
        self.0.lock().unwrap().take()
    }

    fn install(&self) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered, observing) = oneshot::channel();
        let (release, released) = oneshot::channel();
        *self.0.lock().unwrap() = Some(Gate { entered, released });
        (observing, release)
    }
}

#[derive(Debug)]
pub(super) struct Gate {
    entered: oneshot::Sender<()>,
    released: oneshot::Receiver<()>,
}

impl Gate {
    pub(super) fn pause(self) {
        self.entered.send(()).unwrap();
        // This runs inside spawn_blocking, before the actual file replacement.
        self.released.blocking_recv().unwrap();
    }
}

fn settings(root: &std::path::Path) -> Arc<Settings> {
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
        tokio_util::sync::CancellationToken::new(),
    );
    Arc::new(Settings::open(
        DataDir::new(root),
        Arc::new(runtime),
        ParkAfter::DEFAULT,
        false,
    ))
}

fn patch(park_after: ParkAfter) -> SetNodeSettings {
    SetNodeSettings {
        park_after: Some(park_after),
        ..SetNodeSettings::default()
    }
}

#[tokio::test]
async fn cancelled_caller_cannot_release_the_real_writer_or_reorder_application() {
    let result = tokio::time::timeout(WRITE_TEST_BUDGET, async {
        let root = tempfile::tempdir().unwrap();
        let settings = settings(root.path());
        let (entered, release) = settings.gate.install();
        let first = Arc::clone(&settings);
        let caller = tokio::spawn(async move { first.change(patch(ParkAfter::Never)).await });
        entered.await.unwrap();
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());

        // This is false with the old RPC-owned guard: cancellation lets the
        // next write overtake the still-paused real filesystem replacement.
        assert!(settings.saved.try_lock().is_err());
        assert_eq!(settings.runtime.park_after(), ParkAfter::DEFAULT.duration());
        let mut second = Box::pin(settings.change(patch(ParkAfter::DEFAULT)));
        assert!(matches!(futures_util::poll!(&mut second), Poll::Pending));
        release.send(()).unwrap();
        let applied = second.await.unwrap();
        settings.shutdown().await;
        assert_eq!(applied.park_after, ParkAfter::DEFAULT);
        assert_eq!(settings.current().await, applied);
        assert_eq!(settings.runtime.park_after(), applied.park_after.duration());
        assert_eq!(
            settings
                .data_dir
                .read_settings()
                .unwrap()
                .unwrap()
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
        let settings = settings(root.path());
        let (entered, release) = settings.gate.install();
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
        assert_eq!(settings.runtime.park_after(), None);
        assert_eq!(
            settings
                .data_dir
                .read_settings()
                .unwrap()
                .unwrap()
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
