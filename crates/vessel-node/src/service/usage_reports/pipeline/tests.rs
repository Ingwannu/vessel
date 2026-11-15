//! Forces outer A / current metadata B at the actor read / final A, without timers.
use super::*;
use crate::limits::test_support::{Cached, snapshot};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::{Barrier, RwLock};
use vessel_proto::{
    limits::{AccountLimitState, LimitFailure},
    usage_reports::SourceReportState,
};
const BINDING_REGRESSION_BUDGET: std::time::Duration = std::time::Duration::from_secs(20);
struct Fake {
    current: RwLock<Binding>,
    registry: Registry,
    limits: Limits,
    captures: AtomicUsize,
    outer: Barrier,
    before_read: Barrier,
    inner_done: Barrier,
    before_final: Barrier,
}
impl Bindings for Fake {
    async fn capture(&self, _scope: LimitsScope) -> Result<(Binding, HarnessRef), WireError> {
        let phase = self.captures.fetch_add(1, Ordering::SeqCst);
        if phase == 0 {
            let captured = self.current.read().await.clone();
            let _barrier = self.outer.wait().await;
            let _barrier = self.before_read.wait().await;
            let harness = captured.account.harness.clone();
            Ok((captured, harness))
        } else {
            assert_eq!(
                phase, 1,
                "no hidden selector re-resolution may be introduced"
            );
            let _barrier = self.inner_done.wait().await;
            let _barrier = self.before_final.wait().await;
            let captured = self.current.read().await.clone();
            let harness = captured.account.harness.clone();
            Ok((captured, harness))
        }
    }
    fn registry(&self) -> &Registry {
        &self.registry
    }
    fn limits(&self) -> &Limits {
        &self.limits
    }
    fn visibility(&self) -> CostVisibility {
        CostVisibility::Permitted
    }
}
#[tokio::test]
async fn configuration_a_b_a_never_publishes_bs_values_under_as_id() {
    for state in [
        AccountLimitState::Ready {
            snapshot: snapshot("A"),
        },
        AccountLimitState::Failed {
            reason: LimitFailure::Unreachable,
            retained: Some(snapshot("A")),
            retry_at: None,
        },
    ] {
        tokio::time::timeout(BINDING_REGRESSION_BUDGET, async {
            let root = tempfile::Builder::new()
                .prefix("vessel-test-usage-captured-")
                .tempdir()
                .unwrap();
            let cached = Cached::start(root.path(), state);
            let account = cached.a.account().clone();
            let a = Binding {
                account: account.clone(),
                scope: LimitsScope::Account {
                    account: account.clone(),
                },
                proof: Ok(cached.a.clone()),
            };
            let b = Binding {
                account: account.clone(),
                scope: LimitsScope::Account { account },
                proof: Ok(cached.b.clone()),
            };
            let fake = Arc::new(Fake {
                current: RwLock::new(a.clone()),
                registry: Registry::default(),
                limits: cached.limits.clone(),
                captures: AtomicUsize::new(0),
                outer: Barrier::new(2),
                before_read: Barrier::new(2),
                inner_done: Barrier::new(2),
                before_final: Barrier::new(2),
            });
            let changing = Arc::clone(&fake);
            let original = a.clone();
            let controller = tokio::spawn(async move {
                let _barrier = changing.outer.wait().await;
                *changing.current.write().await = b;
                let _barrier = changing.before_read.wait().await;
                let _barrier = changing.inner_done.wait().await;
                *changing.current.write().await = original;
                let _barrier = changing.before_final.wait().await;
            });
            let policy = cached.limits.observation().await.unwrap();
            let report = saved_report(
                fake.as_ref(),
                a.scope.clone(),
                LimitRefresh::Cached,
                &policy,
            )
            .await
            .unwrap();
            controller.await.unwrap();
            let known = fake.registry.resolve(report.id()).await.unwrap();
            assert_eq!(known, a);
            let success = match report.state {
                SourceReportState::Available { observed } => observed,
                SourceReportState::Unavailable { retained, .. } => retained.unwrap(),
                SourceReportState::NotRequested
                | SourceReportState::Refreshing { .. }
                | SourceReportState::Failed { .. } => {
                    panic!("expected current or retained success")
                }
            };
            assert_eq!(success.report.plan.unwrap().as_str(), "A");
            assert_eq!(success.at, chrono::DateTime::UNIX_EPOCH);
            assert_eq!(fake.captures.load(Ordering::SeqCst), 2);
            cached.finish().await;
        })
        .await
        .expect("capture barriers did not settle");
    }
}

mod global_revocation;
