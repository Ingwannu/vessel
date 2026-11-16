use super::*;
use crate::limits::test_support::PhaseWait;
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
    vessel_testkit::within(
        TEST_BUDGET,
        Box::pin(async {
            let gate = Arc::new(Gate::new());
            let _release = Release(Arc::clone(&gate));
            let read = Arc::clone(&gate);
            let fixture = Fixture::start(move |name| read.read(name)).await;
            let _on = fixture.allow(ExternalLookup::On).await;
            assert!(fixture.refresh_all().await.is_empty());
            let original = fixture.snapshot().await.unwrap();
            let id = original.reports[0].id().clone();
            gate.waiting.store(true, Ordering::SeqCst);
            let sources = fixture.sources.clone();
            let global = fixture.global.clone();
            let refreshing =
                tokio::spawn(async move { sources.refresh(Source::Copilot, &global).await });
            let entered = gate
                .entered
                .acquire()
                .phase("physical reader entry")
                .await
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
            let pending = fixture.snapshot().await.unwrap();
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
            let off = fixture.allow(ExternalLookup::Off).await;
            assert_eq!(off.availability, UsageSourceAvailability::SourceOff);
            assert!(!original.still_current() && !pending.still_current());
            assert!(fixture.snapshot().await.unwrap().reports.is_empty());
            gate.release();
            assert!(
                refreshing
                    .phase("refresh task settlement")
                    .await
                    .unwrap()
                    .is_err()
            );
            assert!(response.phase("usage assertion: response").await.is_err());
            assert!(
                fixture
                    .sources
                    .inspect(
                        &id,
                        &fixture.global,
                        vessel_proto::limits::LimitRefresh::Cached
                    )
                    .phase("usage assertion: fixture .sources .inspect")
                    .await
                    .unwrap()
                    .is_none()
            );
            assert!(fixture.snapshot().await.unwrap().reports.is_empty());
            fixture.finish().await;
        }),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refreshing_after_a_failure_does_not_promote_the_retained_success() {
    vessel_testkit::within(
        TEST_BUDGET,
        Box::pin(async {
            let gate = Arc::new(Gate::new());
            let _release = Release(Arc::clone(&gate));
            let read = Arc::clone(&gate);
            let fixture = Fixture::start(move |name| read.read(name)).await;
            let _on = fixture.allow(ExternalLookup::On).await;
            assert!(fixture.refresh_all().await.is_empty());
            std::fs::write(fixture.body(), b"malformed synthetic body").unwrap();
            assert!(fixture.refresh_all().await.is_empty());
            let failed = fixture.snapshot().await.unwrap();
            let SourceReportState::Failed {
                retained: Some(original),
                ..
            } = &failed.reports[0].state
            else {
                panic!("retained failure")
            };
            gate.waiting.store(true, Ordering::SeqCst);
            let sources = fixture.sources.clone();
            let global = fixture.global.clone();
            let refreshing =
                tokio::spawn(async move { sources.refresh(Source::Copilot, &global).await });
            let entered = gate
                .entered
                .acquire()
                .phase("physical reader entry")
                .await
                .unwrap();
            entered.forget();
            let pending = fixture.snapshot().await.unwrap();
            let SourceReportState::Refreshing {
                origin,
                retained: Some(retained),
                ..
            } = &pending.reports[0].state
            else {
                panic!("pending retained failure")
            };
            assert_eq!(
                *origin,
                vessel_proto::usage_reports::UsageRefreshOrigin::Unavailable
            );
            assert_eq!(retained, original);
            gate.release();
            assert!(
                refreshing
                    .phase("refresh task settlement")
                    .await
                    .unwrap()
                    .is_ok()
            );
            fixture.finish().await;
        }),
    )
    .await
    .unwrap();
}
