//! A source and saved-login reads must share the actual configured node's work floor.
use super::*;
use std::{
    path::Path,
    sync::{Arc, Condvar, Mutex},
};
use vessel_harness::{
    AccountSourceAccess, AccountUsageProbe, HarnessEnv, LimitEnvironment, RecordedUsageFormat,
};
use vessel_proto::usage_reports::{
    BuiltinUsageSource, SetUsageSourceLookup, UsageDisclosureVersion,
};
const MIXED_ADMISSION_BUDGET: std::time::Duration = std::time::Duration::from_secs(30);
const ASYNC_WORKERS: usize = 2;
const BLOCKING_WORKERS: usize = 1;
struct Gate {
    entered: tokio::sync::Semaphore,
    released: Mutex<bool>,
    changed: Condvar,
}
impl Gate {
    fn new() -> Self {
        Self {
            entered: tokio::sync::Semaphore::new(0),
            released: Mutex::new(false),
            changed: Condvar::new(),
        }
    }
    fn read(&self, name: &str) -> Option<std::ffi::OsString> {
        if name != "COPILOT_TOKEN" {
            return None;
        }
        self.entered.add_permits(1);
        let mut released = self.released.lock().unwrap();
        while !*released {
            released = self.changed.wait(released).unwrap();
        }
        Some("synthetic-mixed-admission-token".into())
    }
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.changed.notify_all();
    }
}
struct Release(Arc<Gate>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}

async fn configured(
    root: &Path,
    gate: Arc<Gate>,
    stop: CancellationToken,
    tasks: &TaskTracker,
) -> Limits {
    let body = root.join("mixed-body.json");
    std::fs::write(&body, br#"{"copilot_plan":"Synthetic"}"#).unwrap();
    let mut config = crate::Config::new(DataDir::new(root), HarnessEnv::new(root.to_owned()));
    config.limits = LimitProbe::Recorded {
        root: root.to_owned(),
        body: body.clone(),
        status: 200,
        retry_after: None,
    };
    config.usage_sources = AccountUsageProbe::Recorded {
        root: root.to_owned(),
        access: AccountSourceAccess::new(root.to_owned(), move |name| gate.read(name)),
        source: BuiltinUsageSource::Copilot,
        body,
        status: 200,
        retry_after: None,
        format: RecordedUsageFormat::Vendor,
    };
    // Only configured actors are started: no default TCP listener, daemon or real login.
    let limits = Limits::configured(&config, stop, tasks).await;
    assert_eq!(
        limits.policy(ExternalLookup::On).await.unwrap(),
        ExternalLookup::On
    );
    let global = limits.observation().await.unwrap();
    let _consent = limits
        .sources
        .set(
            SetUsageSourceLookup {
                source: "copilot".parse().unwrap(),
                policy: ExternalLookup::On,
                disclosure_version: UsageDisclosureVersion::INITIAL,
            },
            &global,
        )
        .await
        .unwrap();
    limits
}
async fn queue(
    limits: &Limits,
    binding: LimitBinding,
) -> oneshot::Receiver<Result<UsageLimits, WireError>> {
    let (reply, response) = oneshot::channel();
    limits
        .commands
        .send(Command::Read {
            account: binding.account().clone(),
            binding: Ok(binding),
            refresh: LimitRefresh::Refresh,
            observed: None,
            reply,
        })
        .await
        .unwrap();
    response
}
fn bindings(root: &Path) -> [LimitBinding; 2] {
    let profiles = vessel_harness::parse_profiles(
        r#"[profiles.first]
harness = "codex"
[profiles.second]
harness = "codex"
"#,
        Some(root),
    )
    .unwrap();
    ["first", "second"].map(|name| {
        vessel_harness::limits_binding(
            &LimitEnvironment::new(),
            &HarnessEnv::new(root.to_owned()),
            profiles.get(&name.parse().unwrap()).unwrap(),
        )
        .unwrap()
    })
}
#[test]
fn configured_sources_and_saved_logins_cannot_admit_a_third_actual_reader() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(ASYNC_WORKERS)
        .max_blocking_threads(BLOCKING_WORKERS)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        tokio::time::timeout(MIXED_ADMISSION_BUDGET, async {
            let root = tempfile::tempdir().unwrap();
            let gate = Arc::new(Gate::new());
            let _release = Release(Arc::clone(&gate));
            let stop = CancellationToken::new();
            let tasks = TaskTracker::new();
            let limits = configured(root.path(), Arc::clone(&gate), stop.clone(), &tasks).await;
            let global = limits.observation().await.unwrap();
            let sources = limits.sources.clone();
            let source = tokio::spawn(async move { sources.refresh_all(&global).await });
            let entered = gate.entered.acquire().await.unwrap();
            entered.forget();
            // The sole blocking worker is occupied by the real source callback.
            // One saved reader can queue with the other permit; another must refuse.
            let [a, b] = bindings(root.path());
            let mut first = queue(&limits, a).await;
            let mut second = queue(&limits, b).await;
            let (first_finished, refused) = tokio::select! {
                result = &mut first => (true, result.unwrap().unwrap()),
                result = &mut second => (false, result.unwrap().unwrap()),
            };
            assert!(matches!(
                refused.state,
                AccountLimitState::Failed {
                    reason: LimitFailure::Busy,
                    ..
                }
            ));
            gate.release();
            let _remaining = if first_finished {
                second.await
            } else {
                first.await
            }
            .unwrap()
            .unwrap();
            let _source = source.await.unwrap();
            stop.cancel();
            let _closed = tasks.close();
            tasks.wait().await;
        })
        .await
        .expect("mixed source and saved-login work did not drain");
    });
}
