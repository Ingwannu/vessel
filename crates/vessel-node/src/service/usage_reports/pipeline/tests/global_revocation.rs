//! Hold the real report after registry identity and before actual actor admission.
use super::*;
use crate::limits::test_support::{PHASE_BUDGET, PhaseWait};
use std::sync::{Condvar, Mutex, atomic::AtomicBool};
use tokio::sync::Semaphore;
use vessel_harness::LimitProbe;
use vessel_proto::{ErrorCode, limits::ExternalLookup};

struct Gate {
    armed: AtomicBool,
    entered: Semaphore,
    released: Mutex<bool>,
    wake: Condvar,
}
impl Gate {
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.wake.notify_all();
    }
    fn hold_once(&self) {
        if self.armed.swap(false, Ordering::SeqCst) {
            self.entered.add_permits(1);
            let (_guard, deadline) = self
                .wake
                .wait_timeout_while(self.released.lock().unwrap(), PHASE_BUDGET, |released| {
                    !*released
                })
                .unwrap();
            assert!(
                !deadline.timed_out(),
                "policy controller never released actor admission"
            );
        }
    }
}
struct Release(Arc<Gate>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}
struct Paused {
    binding: Binding,
    registry: Registry,
    limits: Limits,
    gate: Arc<Gate>,
}
impl Bindings for Paused {
    async fn capture(&self, _scope: LimitsScope) -> Result<(Binding, HarnessRef), WireError> {
        Ok((self.binding.clone(), self.binding.account.harness.clone()))
    }
    fn registry(&self) -> &Registry {
        &self.registry
    }
    fn limits(&self) -> &Limits {
        self.gate.hold_once();
        &self.limits
    }
    fn visibility(&self) -> CostVisibility {
        CostVisibility::Permitted
    }
}
fn recorded(root: &std::path::Path) -> Cached {
    let home = root.join("a");
    std::fs::create_dir(&home).unwrap();
    std::fs::write(home.join("auth.json"), br#"{"tokens":{"access_token":"synthetic-offline-access","account_id":"synthetic-offline-account"}}"#).unwrap();
    let body = root.join("response.json");
    std::fs::write(&body, vessel_testkit::limits::CODEX_USAGE).unwrap();
    Cached::with_probe(
        root,
        AccountLimitState::NotRequested,
        LimitProbe::Recorded {
            root: root.to_owned(),
            body,
            status: 200,
            retry_after: None,
        },
    )
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_old_report_cannot_start_a_probe_after_off_on_before_actor_admission() {
    vessel_testkit::within(BINDING_REGRESSION_BUDGET, Box::pin(check_revocation()))
        .await
        .expect("saved report policy admission did not settle");
}
async fn check_revocation() {
    let root = tempfile::tempdir().unwrap();
    let cached = recorded(root.path());
    let account = cached.a.account().clone();
    let scope = LimitsScope::Account {
        account: account.clone(),
    };
    let gate = Arc::new(Gate {
        armed: AtomicBool::new(true),
        entered: Semaphore::new(0),
        released: Mutex::new(false),
        wake: Condvar::new(),
    });
    let _release = Release(Arc::clone(&gate));
    let reader = Arc::new(Paused {
        binding: Binding {
            account: account.clone(),
            scope: scope.clone(),
            proof: Ok(cached.a.clone()),
        },
        registry: Registry::default(),
        limits: cached.limits.clone(),
        gate: Arc::clone(&gate),
    });
    let old = cached
        .limits
        .observation()
        .phase("usage phase: cached.limits.observation")
        .await
        .unwrap();
    let pending_reader = Arc::clone(&reader);
    let pending_scope = scope.clone();
    let pending = tokio::spawn(async move {
        saved_report(
            pending_reader.as_ref(),
            pending_scope,
            LimitRefresh::Refresh,
            &old,
        )
        .await
    });
    gate.entered
        .acquire()
        .phase("usage phase: gate.entered.acquire")
        .await
        .unwrap()
        .forget();
    assert_eq!(
        cached
            .limits
            .policy(ExternalLookup::Off)
            .phase("usage assertion: cached.limits.policy")
            .await
            .unwrap(),
        ExternalLookup::Off
    );
    assert_eq!(
        cached
            .limits
            .policy(ExternalLookup::On)
            .phase("usage assertion: cached.limits.policy")
            .await
            .unwrap(),
        ExternalLookup::On
    );
    gate.release();
    assert_eq!(
        pending
            .phase("usage assertion: pending")
            .await
            .unwrap()
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    // Final publication refusal alone is insufficient: the legacy cache would
    // already contain a successful physical probe under the vulnerable path.
    let idle = cached
        .limits
        .read(account, Ok(cached.a.clone()), LimitRefresh::Cached)
        .phase("usage phase: cached .limits .read")
        .await
        .unwrap();
    assert_eq!(idle.state, AccountLimitState::NotRequested);
    let current = cached
        .limits
        .observation()
        .phase("usage phase: cached.limits.observation")
        .await
        .unwrap();
    let report = saved_report(reader.as_ref(), scope, LimitRefresh::Refresh, &current)
        .phase("usage phase: saved_report")
        .await
        .unwrap();
    assert!(matches!(report.state, SourceReportState::Available { .. }));
    cached.finish().phase("usage phase: cached.finish").await;
}
