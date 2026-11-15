//! Actual global actor revocation survives a dropped setter blocked before Clear.
use super::*;
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::sync::{Semaphore, oneshot};
use vessel_harness::AccountSourceAccess;

const REVOCATION_TEST_BUDGET: std::time::Duration = std::time::Duration::from_secs(30);
const RECORDED_REPORT: &[u8] = br#"{"plan":"Synthetic","windows":[],"balances":[],"details":[]}"#;

#[derive(Clone, Copy)]
enum Held {
    Credential,
    Http,
}
struct Control {
    credential: AtomicBool,
    entered: Semaphore,
    released: Mutex<bool>,
    changed: Condvar,
    http: AtomicBool,
    polls: AtomicUsize,
    http_started: Semaphore,
    http_dropped: Semaphore,
}
impl Control {
    fn new() -> Self {
        Self {
            credential: AtomicBool::new(false),
            entered: Semaphore::new(0),
            released: Mutex::new(false),
            changed: Condvar::new(),
            http: AtomicBool::new(false),
            polls: AtomicUsize::new(0),
            http_started: Semaphore::new(0),
            http_dropped: Semaphore::new(0),
        }
    }
    fn access(
        self: &Arc<Self>,
        home: std::path::PathBuf,
        reads: Arc<AtomicUsize>,
    ) -> AccountSourceAccess {
        let credential = Arc::clone(self);
        let http = Arc::clone(self);
        AccountSourceAccess::new(home, move |name| {
            if name != "CURSOR_ACCESS_TOKEN" {
                return None;
            }
            let _previous = reads.fetch_add(1, Ordering::SeqCst);
            if credential.credential.load(Ordering::SeqCst) {
                credential.entered.add_permits(1);
                let mut released = credential.released.lock().unwrap();
                while !*released {
                    released = credential.changed.wait(released).unwrap();
                }
            }
            Some("synthetic-global-revocation-token".into())
        })
        .with_recorded_http(move || {
            let http = Arc::clone(&http);
            Box::pin(async move {
                let _previous = http.polls.fetch_add(1, Ordering::SeqCst);
                if http.http.load(Ordering::SeqCst) {
                    let _dropped = Dropped(Arc::clone(&http));
                    http.http_started.add_permits(1);
                    std::future::pending().await
                } else {
                    Ok(RECORDED_REPORT.to_vec())
                }
            })
        })
    }
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.changed.notify_all();
    }
}
struct Release(Arc<Control>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}
struct Dropped(Arc<Control>);
impl Drop for Dropped {
    fn drop(&mut self) {
        self.0.http_dropped.add_permits(1);
    }
}

async fn acquired(semaphore: &Semaphore) {
    semaphore.acquire().await.unwrap().forget();
}

async fn revoked_without_setter_continuation(held: Held) {
    let control = Arc::new(Control::new());
    let _release = Release(Arc::clone(&control));
    let capability = Arc::clone(&control);
    let fixture = Fixture::with_access(move |home, reads| capability.access(home, reads)).await;
    fixture.allow().await;
    let original = fixture.list(&fixture.owner, LimitRefresh::Refresh).await;
    let original_id = cursor(&original).id().clone();
    assert_eq!(control.polls.load(Ordering::SeqCst), 1);
    let service = fixture.handler();
    let old_global = service.0.limits.observation().await.unwrap();
    let old_snapshot = service
        .0
        .limits
        .sources
        .snapshot(&old_global)
        .await
        .unwrap();
    match held {
        Held::Credential => control.credential.store(true, Ordering::SeqCst),
        Held::Http => control.http.store(true, Ordering::SeqCst),
    }
    let refreshing_service = fixture.handler();
    let owner = fixture.owner.acting_as(crate::principal::Principal::Owner);
    let refreshing = tokio::spawn(async move {
        refreshing_service
            .list_usage_reports(
                &owner,
                ListUsageReports {
                    scope: UsageReportScope::Machine,
                    refresh: LimitRefresh::Refresh,
                },
            )
            .await
    });
    match held {
        Held::Credential => acquired(&control.entered).await,
        Held::Http => acquired(&control.http_started).await,
    }
    cancel_setter_after_durable_off(&fixture).await;
    // This test query never prunes or reads snapshots: the owner must have
    // observed canonical revocation independently of the abandoned RPC.
    assert_eq!(service.0.limits.sources.cached_entries().await, 0);
    assert!(!old_snapshot.still_current());
    if matches!(held, Held::Http) {
        acquired(&control.http_dropped).await;
    }
    control.release();
    assert!(refreshing.await.unwrap().is_err());
    let expected = if matches!(held, Held::Credential) {
        1
    } else {
        2
    };
    assert_eq!(control.polls.load(Ordering::SeqCst), expected);
    control.credential.store(false, Ordering::SeqCst);
    control.http.store(false, Ordering::SeqCst);
    let _on = service
        .set_usage_lookup(
            &fixture.owner,
            vessel_proto::limits::SetUsageLookup {
                policy: ExternalLookup::On,
            },
        )
        .await
        .unwrap();
    let cached = fixture.list(&fixture.reader, LimitRefresh::Cached).await;
    assert!(
        cached.reports.is_empty(),
        "Off/On must not revive cached values"
    );
    assert_eq!(
        control.polls.load(Ordering::SeqCst),
        expected,
        "On is not a fetch"
    );
    let fresh = fixture.list(&fixture.owner, LimitRefresh::Refresh).await;
    assert_ne!(cursor(&fresh).id(), &original_id);
    assert!(matches!(
        cursor(&fresh).state,
        SourceReportState::Available { .. }
    ));
    assert_eq!(control.polls.load(Ordering::SeqCst), expected + 1);
    fixture.finish().await;
}

async fn cancel_setter_after_durable_off(fixture: &Fixture) {
    let service = fixture.handler();
    let holding = fixture.handler();
    let (entered, ready) = oneshot::channel();
    let (release, released) = oneshot::channel();
    let holder = tokio::spawn(async move {
        holding
            .0
            .usage_reports
            .hold_cleanup(entered, released)
            .await;
    });
    ready.await.unwrap();
    let mut setter = Box::pin(service.set_usage_lookup(
        &fixture.owner,
        vessel_proto::limits::SetUsageLookup {
            policy: ExternalLookup::Off,
        },
    ));
    assert!(futures_util::poll!(setter.as_mut()).is_pending());
    // Observe is queued after Policy: its answer establishes that the policy
    // actor finished persistence, before the blocked service cleanup can run.
    let off = service.0.limits.observation().await.unwrap();
    assert_eq!(off.policy, ExternalLookup::Off);
    assert_eq!(
        vessel_store::DataDir::new(fixture.root.path().join("data"))
            .read_usage_lookup()
            .unwrap(),
        ExternalLookup::Off
    );
    assert!(futures_util::poll!(setter.as_mut()).is_pending());
    drop(setter); // No caller continuation may submit Sources::clear.
    release.send(()).unwrap();
    holder.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn committed_global_off_cancels_held_credentials_after_the_setter_is_dropped() {
    tokio::time::timeout(
        REVOCATION_TEST_BUDGET,
        revoked_without_setter_continuation(Held::Credential),
    )
    .await
    .unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn committed_global_off_drops_active_http_after_the_setter_is_dropped() {
    tokio::time::timeout(
        REVOCATION_TEST_BUDGET,
        revoked_without_setter_continuation(Held::Http),
    )
    .await
    .unwrap();
}
