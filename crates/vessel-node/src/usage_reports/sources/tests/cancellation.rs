use super::*;
use std::sync::{Condvar, Mutex};
struct Gate {
    waiting: AtomicBool,
    released: Mutex<bool>,
    changed: Condvar,
    entered: tokio::sync::Semaphore,
    calls: AtomicUsize,
}
impl Gate {
    fn new() -> Self {
        Self {
            waiting: AtomicBool::new(false),
            released: Mutex::new(false),
            changed: Condvar::new(),
            entered: tokio::sync::Semaphore::new(0),
            calls: AtomicUsize::new(0),
        }
    }
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.changed.notify_all();
    }
    fn read(&self, name: &str) -> Option<std::ffi::OsString> {
        if name != "COPILOT_TOKEN" {
            return None;
        }
        let _prior = self.calls.fetch_add(1, Ordering::SeqCst);
        if self.waiting.load(Ordering::SeqCst) {
            self.entered.add_permits(1);
            let mut released = self.released.lock().unwrap();
            while !*released {
                released = self.changed.wait(released).unwrap();
            }
        }
        Some("fixture-gated-token".into())
    }
}
struct Release(Arc<Gate>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn identical_refreshes_coalesce_and_revocation_fences_late_work_and_copied_snapshots() {
    let gate = Arc::new(Gate::new());
    let _release = Release(Arc::clone(&gate));
    let read = Arc::clone(&gate);
    let fixture = Fixture::start(move |name| read.read(name)).await;
    let _on = fixture.allow(ExternalLookup::On).await;
    assert!(
        fixture
            .sources
            .refresh_all(&fixture.global)
            .await
            .is_empty()
    );
    let original = fixture.sources.snapshot(&fixture.global).await.unwrap();
    let id = original.reports[0].id().clone();
    gate.waiting.store(true, Ordering::SeqCst);
    let sources = fixture.sources.clone();
    let global = fixture.global.clone();
    let refreshing = tokio::spawn(async move { sources.refresh(Source::Copilot, &global).await });
    let entered = tokio::time::timeout(TEST_BUDGET, gate.entered.acquire())
        .await
        .unwrap()
        .unwrap();
    entered.forget();
    let (reply, response) = oneshot::channel();
    fixture
        .sources
        .0
        .as_ref()
        .unwrap()
        .try_send(Command::Refresh {
            source: Source::Copilot,
            global: fixture.global.clone(),
            reply,
        })
        .unwrap();
    let pending = fixture.sources.snapshot(&fixture.global).await.unwrap();
    assert!(matches!(
        pending.reports[0].state,
        SourceReportState::Refreshing {
            origin: vessel_proto::usage_reports::UsageRefreshOrigin::Available,
            retained: Some(_),
            ..
        }
    ));
    assert_eq!(
        gate.calls.load(Ordering::SeqCst),
        2,
        "second waiter must join the existing actual read"
    );
    let off = tokio::time::timeout(TEST_BUDGET, fixture.allow(ExternalLookup::Off))
        .await
        .unwrap();
    assert_eq!(off.availability, UsageSourceAvailability::SourceOff);
    assert!(!original.still_current() && !pending.still_current());
    assert!(
        fixture
            .sources
            .snapshot(&fixture.global)
            .await
            .unwrap()
            .reports
            .is_empty()
    );
    gate.release();
    assert!(
        tokio::time::timeout(TEST_BUDGET, refreshing)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert!(response.await.is_err());
    assert!(
        fixture
            .sources
            .inspect(
                &id,
                &fixture.global,
                vessel_proto::limits::LimitRefresh::Cached
            )
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .sources
            .snapshot(&fixture.global)
            .await
            .unwrap()
            .reports
            .is_empty()
    );
    fixture.finish().await;
}
