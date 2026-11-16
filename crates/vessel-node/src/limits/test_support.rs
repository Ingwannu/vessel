//! Fileless actor fixture for the report binding barrier regression.
use super::*;
use std::{ffi::OsString, future::Future, path::Path, time::Duration};
use vessel_harness::{HarnessEnv, LimitEnvironment, Profile};
use vessel_proto::HarnessKind;
pub(crate) struct Cached {
    pub(crate) limits: Limits,
    pub(crate) a: LimitBinding,
    pub(crate) b: LimitBinding,
    stop: CancellationToken,
    tasks: TaskTracker,
}
impl Drop for Cached {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}
pub(crate) fn snapshot(plan: &str) -> LimitSnapshot {
    LimitSnapshot {
        at: chrono::DateTime::UNIX_EPOCH,
        plan: Some(plan.parse().unwrap()),
        windows: Vec::new(),
        credits: None,
        extra_usage: None,
        allowed: None,
        limit_reached: None,
        incomplete: false,
    }
}
fn binding(root: &Path, side: &str) -> LimitBinding {
    let home = root.join(side);
    let homes = HarnessEnv::from_vars(root.to_owned(), |key| {
        if key == "CODEX_HOME" {
            Some(OsString::from(&home))
        } else {
            None
        }
    });
    vessel_harness::limits_binding(
        &LimitEnvironment::new(),
        &homes,
        &Profile::default_for(HarnessKind::Codex),
    )
    .unwrap()
}
impl Cached {
    pub(crate) fn start(root: &Path, state_a: AccountLimitState) -> Self {
        Self::with_probe(root, state_a, LimitProbe::Disabled)
    }
    pub(crate) fn with_probe(root: &Path, state_a: AccountLimitState, probe: LimitProbe) -> Self {
        let a = binding(root, "a");
        let b = binding(root, "b");
        assert_ne!(
            a, b,
            "the same account must have distinct resolved login homes"
        );
        let account = a.account().clone();
        assert_eq!(account, *b.account());
        let stop = CancellationToken::new();
        let tasks = TaskTracker::new();
        let (commands, receiver) = mpsc::channel(COMMANDS);
        let owner = Owner {
            dir: DataDir::new(root),
            policy: ExternalLookup::On,
            probe,
            cache: vec![
                Entry {
                    account: account.clone(),
                    binding: a.clone(),
                    identity: None,
                    state: state_a,
                },
                Entry {
                    account,
                    binding: b.clone(),
                    identity: None,
                    state: AccountLimitState::Ready {
                        snapshot: snapshot("B"),
                    },
                },
            ],
            generation: CancellationToken::new(),
            jobs: JoinSet::new(),
            context: LimitContext::new(),
        };
        drop(tasks.spawn(owner.run(receiver, stop.clone())));
        Self {
            limits: Limits {
                commands,
                sources: crate::usage_reports::Sources::default(),
            },
            a,
            b,
            stop,
            tasks,
        }
    }
    pub(crate) async fn finish(&self) {
        self.stop.cancel();
        let _closed = self.tasks.close();
        self.tasks.wait().await;
    }
}

// New usage fixtures name their phase waits within the canonical case safety net.

pub(crate) const PHASE_BUDGET: Duration = Duration::from_secs(10);

pub(crate) trait PhaseWait: Future + Sized {
    fn phase(self, name: &'static str) -> impl Future<Output = Self::Output> {
        self.phase_for(name, PHASE_BUDGET)
    }
    fn phase_for(self, name: &'static str, budget: Duration) -> impl Future<Output = Self::Output> {
        // Keep named deadlines from copying large node setup futures onto the
        // test stack. This allocation does not poll or start its clock.
        let operation = Box::pin(self);
        async move {
            tokio::time::timeout(budget, operation)
                .await
                .unwrap_or_else(|cause| panic!("{name}: {cause}"))
        }
    }
}
impl<F: Future> PhaseWait for F {}
