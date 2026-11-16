//! Deterministic holds use the actual blocking closure and join owner. Only
//! numeric standard-native resolution runs; no external DNS is contacted.

mod queued;

use super::super::{MAX_HOST_BYTES, MAX_RESOLVED_ADDRESSES};
use super::*;
use futures_util::FutureExt as _;
use std::{
    sync::{
        Condvar,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

const OUTER: Duration = Duration::from_secs(30);
const PHASE: Duration = Duration::from_secs(5);
const SINGLE_SLOT: usize = 1;
const ZERO_PORT: ResolutionPort = ResolutionPort(0);
const CANARY: &str = "private-host.invalid/private-home/private-token";

async fn within(future: impl Future<Output = ()>) {
    vessel_testkit::within(OUTER, Box::pin(future))
        .await
        .expect("resolution outer safety");
}
async fn phase<T>(name: &str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(PHASE, Box::pin(future))
        .await
        .unwrap_or_else(|_| panic!("resolution phase: {name}"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Stage {
    BeforeStart,
    AfterStart,
}
struct Gate {
    released: Mutex<bool>,
    changed: Condvar,
}
impl Gate {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            released: Mutex::new(false),
            changed: Condvar::new(),
        })
    }
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.changed.notify_all();
    }
    fn wait(&self) {
        let released = self.released.lock().unwrap();
        let (released, _timeout) = self
            .changed
            .wait_timeout_while(released, PHASE, |released| !*released)
            .unwrap();
        assert!(*released, "named native worker barrier timed out");
    }
}
struct Release(Arc<Gate>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}
pub(super) struct Probe {
    at: Stage,
    gate: Arc<Gate>,
    ready: tokio::sync::mpsc::Sender<()>,
    entered_native: Arc<AtomicBool>,
    panic_after: bool,
}
impl Probe {
    pub(super) fn enter(&self, stage: Stage) {
        if stage == Stage::AfterStart {
            self.entered_native.store(true, Ordering::SeqCst);
        }
        if stage == self.at {
            self.ready.try_send(()).unwrap();
            self.gate.wait();
            assert!(!self.panic_after, "{CANARY}");
        }
    }
}
struct Held {
    release: Release,
    ready: tokio::sync::mpsc::Receiver<()>,
    entered: Arc<AtomicBool>,
}
impl Held {
    fn new() -> Self {
        let (send, ready) = tokio::sync::mpsc::channel(MAX_NATIVE_RESOLUTIONS);
        drop(send);
        Self {
            release: Release(Gate::new()),
            ready,
            entered: Arc::new(AtomicBool::new(false)),
        }
    }
    fn probes(at: Stage, count: usize) -> (Self, Vec<Probe>) {
        let (ready, receive) = tokio::sync::mpsc::channel(MAX_NATIVE_RESOLUTIONS);
        let mut held = Self::new();
        held.ready = receive;
        let probes = (0..count)
            .map(|_| Probe {
                at,
                gate: Arc::clone(&held.release.0),
                ready: ready.clone(),
                entered_native: Arc::clone(&held.entered),
                panic_after: false,
            })
            .collect();
        (held, probes)
    }
    async fn entered(&mut self, count: usize) {
        for _ in 0..count {
            phase("actual blocking worker entered barrier", self.ready.recv())
                .await
                .unwrap();
        }
    }
}

struct Task<T>(Option<tokio::task::JoinHandle<T>>);
impl<T> Task<T> {
    async fn joined(&mut self) -> Result<T, tokio::task::JoinError> {
        let result = phase("owned caller task joined", self.0.as_mut().unwrap()).await;
        drop(self.0.take());
        result
    }
    fn abort(&self) {
        self.0.as_ref().unwrap().abort();
    }
}
impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        if let Some(task) = &self.0 {
            task.abort();
        }
    }
}

fn scope(budget: &ResolutionBudget, quota: usize) -> ResolutionScope<()> {
    budget.scope(ResolutionQuota::new(quota).unwrap(), Arc::new(()))
}
fn start<Hold: Send + Sync + 'static>(
    scope: &ResolutionScope<Hold>,
    probe: Probe,
) -> Task<Result<Addresses, Failure>> {
    let handle = scope.handle();
    Task(Some(tokio::spawn(async move {
        handle
            .start(&HostName::new("127.0.0.1").unwrap(), ZERO_PORT, Some(probe))
            .await
    })))
}

#[tokio::test]
async fn numeric_standard_native_work_and_repeatable_drain_are_real() {
    within(async {
        let budget = ResolutionBudget::default();
        let scope = scope(&budget, SINGLE_SLOT);
        let result = phase(
            "numeric standard-native lookup",
            scope
                .handle()
                .resolve(&HostName::new("127.0.0.1").unwrap(), ZERO_PORT),
        )
        .await
        .unwrap();
        assert_eq!(
            result.iter().collect::<Vec<_>>(),
            vec!["127.0.0.1:0".parse().unwrap()]
        );
        assert!(!format!("{result:?}").contains("127.0.0.1"));
        let _first_receipt = phase("normal physical drain", scope.drain()).await.unwrap();
        let _repeat_receipt = phase("repeat normal physical drain", scope.drain())
            .await
            .unwrap();
        assert_eq!(scope.state(), ScopeState::Drained);
        assert_eq!(budget.0.available_permits(), MAX_NATIVE_RESOLUTIONS);
    })
    .await;
}

#[tokio::test]
async fn caller_abort_and_cancelled_drain_waiter_do_not_release_physical_work() {
    within(async {
        let budget = ResolutionBudget::default();
        let scope = scope(&budget, SINGLE_SLOT);
        let (mut held, probes) = Held::probes(Stage::AfterStart, SINGLE_SLOT);
        let mut task = start(&scope, probes.into_iter().next().unwrap());
        held.entered(SINGLE_SLOT).await;
        task.abort();
        assert!(task.joined().await.unwrap_err().is_cancelled());
        assert_eq!(
            budget.0.available_permits(),
            MAX_NATIVE_RESOLUTIONS - SINGLE_SLOT
        );
        let host = HostName::new("127.0.0.1").unwrap();
        assert_eq!(
            phase(
                "local capacity refuses without waiting",
                scope.handle().resolve(&host, ZERO_PORT)
            )
            .await
            .unwrap_err()
            .kind(),
            FailureKind::Full
        );
        let mut drain = Box::pin(scope.drain());
        assert!(drain.as_mut().now_or_never().is_none());
        drop(drain);
        assert_eq!(scope.state(), ScopeState::Draining);
        assert_eq!(
            phase(
                "closed scope refuses without waiting",
                scope.handle().resolve(&host, ZERO_PORT)
            )
            .await
            .unwrap_err()
            .kind(),
            FailureKind::Closed
        );
        held.release.0.release();
        let _resumed_receipt = phase(
            "native join after lost caller and drain waiter",
            scope.drain(),
        )
        .await
        .unwrap();
        assert_eq!(budget.0.available_permits(), MAX_NATIVE_RESOLUTIONS);
    })
    .await;
}

#[tokio::test]
async fn close_covers_admitted_work_before_the_native_start_gate() {
    within(async {
        let budget = ResolutionBudget::default();
        let scope = scope(&budget, SINGLE_SLOT);
        let (mut held, probes) = Held::probes(Stage::BeforeStart, SINGLE_SLOT);
        let mut task = start(&scope, probes.into_iter().next().unwrap());
        held.entered(SINGLE_SLOT).await;
        scope.close();
        assert_eq!(scope.state(), ScopeState::Draining);
        held.release.0.release();
        assert_eq!(
            task.joined().await.unwrap().unwrap_err().kind(),
            FailureKind::Closed
        );
        assert!(!held.entered.load(Ordering::SeqCst));
        let _skipped_receipt = phase("skipped admitted worker joined", scope.drain())
            .await
            .unwrap();
    })
    .await;
}

struct ConsumerHold(Option<oneshot::Sender<()>>);
impl Drop for ConsumerHold {
    fn drop(&mut self) {
        if let Some(send) = self.0.take() {
            let _gone = send.send(());
        }
    }
}

#[tokio::test]
async fn original_consumer_hold_survives_scope_and_caller_loss_inside_native_work() {
    within(async {
        let budget = ResolutionBudget::default();
        let (dropped, mut drop_notice) = oneshot::channel();
        let consumer = Arc::new(ConsumerHold(Some(dropped)));
        let scope = budget.scope(
            ResolutionQuota::new(SINGLE_SLOT).unwrap(),
            Arc::clone(&consumer),
        );
        let handle = scope.handle();
        let weak_hold = Arc::downgrade(&consumer);
        drop(consumer);
        let (mut held, probes) = Held::probes(Stage::AfterStart, SINGLE_SLOT);
        let mut task = start(&scope, probes.into_iter().next().unwrap());
        held.entered(SINGLE_SLOT).await;
        task.abort();
        assert!(task.joined().await.unwrap_err().is_cancelled());
        drop(scope);
        assert!(drop_notice.try_recv().is_err());
        assert!(weak_hold.upgrade().is_some());
        assert_eq!(
            phase(
                "owner drop closes weak handle",
                handle.resolve(&HostName::new("127.0.0.1").unwrap(), ZERO_PORT)
            )
            .await
            .unwrap_err()
            .kind(),
            FailureKind::Closed
        );
        held.release.0.release();
        phase(
            "original physical consumer hold released after native join",
            drop_notice,
        )
        .await
        .unwrap();
        assert!(weak_hold.upgrade().is_none());
        assert_eq!(budget.0.available_permits(), MAX_NATIVE_RESOLUTIONS);
    })
    .await;
}

#[tokio::test]
async fn root_quota_survives_other_scopes_and_refuses_before_another_spawn() {
    within(async {
        let budget = ResolutionBudget::default();
        let scope = scope(&budget, MAX_NATIVE_RESOLUTIONS);
        let other = self::scope(&budget, SINGLE_SLOT);
        let (mut held, probes) = Held::probes(Stage::AfterStart, MAX_NATIVE_RESOLUTIONS);
        let mut tasks: Vec<_> = probes
            .into_iter()
            .map(|probe| start(&scope, probe))
            .collect();
        held.entered(MAX_NATIVE_RESOLUTIONS).await;
        assert_eq!(budget.0.available_permits(), 0);
        assert_eq!(
            phase(
                "same root finite capacity",
                other
                    .handle()
                    .resolve(&HostName::new("127.0.0.1").unwrap(), ZERO_PORT)
            )
            .await
            .unwrap_err()
            .kind(),
            FailureKind::Full
        );
        let mut drain = Box::pin(scope.drain());
        assert!(drain.as_mut().now_or_never().is_none());
        held.release.0.release();
        for task in &mut tasks {
            let _addresses = task.joined().await.unwrap().unwrap();
        }
        let _root_receipt = phase("root work drained", drain).await.unwrap();
        let _other_receipt = phase("other scope closes normally", other.drain())
            .await
            .unwrap();
        assert_eq!(budget.0.available_permits(), MAX_NATIVE_RESOLUTIONS);
    })
    .await;
}

#[tokio::test]
async fn worker_panic_keeps_a_typed_safe_cause_and_never_mints_normal_drain() {
    within(async {
        let budget = ResolutionBudget::default();
        let scope = scope(&budget, SINGLE_SLOT);
        let (mut held, mut probes) = Held::probes(Stage::AfterStart, SINGLE_SLOT);
        let mut probe = probes.pop().unwrap();
        probe.panic_after = true;
        let mut task = start(&scope, probe);
        held.entered(SINGLE_SLOT).await;
        held.release.0.release();
        let error = task.joined().await.unwrap().unwrap_err();
        assert_eq!(error.kind(), FailureKind::Worker);
        safe_chain(&error);
        assert_eq!(
            phase(
                "failed worker cannot acknowledge normal drain",
                scope.drain()
            )
            .await
            .unwrap_err()
            .kind(),
            FailureKind::Worker
        );
        assert_eq!(scope.state(), ScopeState::Failed);
        assert_eq!(budget.0.available_permits(), MAX_NATIVE_RESOLUTIONS);
    })
    .await;
}

#[tokio::test]
async fn failed_owner_phase_unwind_releases_worker_and_reaps_its_physical_hold() {
    within(async {
        let budget = ResolutionBudget::default();
        let (dropped, drop_notice) = oneshot::channel();
        let scope = budget.scope(
            ResolutionQuota::new(SINGLE_SLOT).unwrap(),
            Arc::new(ConsumerHold(Some(dropped))),
        );
        let (mut held, probes) = Held::probes(Stage::AfterStart, SINGLE_SLOT);
        let task = start(&scope, probes.into_iter().next().unwrap());
        held.entered(SINGLE_SLOT).await;
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _scope = scope;
            let _held = held;
            let _task = task;
            panic!("controlled owner phase failure");
        }));
        assert!(outcome.is_err());
        phase("failed phase releases original physical hold", drop_notice)
            .await
            .unwrap();
        assert_eq!(budget.0.available_permits(), MAX_NATIVE_RESOLUTIONS);
    })
    .await;
}

fn safe_chain(error: &Failure) {
    assert!(!format!("{error:?}").contains(CANARY));
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(error) = source {
        assert!(!format!("{error:?}").contains(CANARY));
        assert!(!error.to_string().contains(CANARY));
        source = error.source();
    }
}

#[test]
fn inputs_results_and_source_chains_have_real_bounds() {
    assert!(HostName::new(&"a".repeat(MAX_HOST_BYTES)).is_ok());
    assert_eq!(
        HostName::new(&"a".repeat(MAX_HOST_BYTES + SINGLE_SLOT))
            .unwrap_err()
            .kind(),
        FailureKind::InvalidHost
    );
    for host in [
        "",
        "user@host.invalid",
        "host.invalid/path",
        "host.invalid?key",
        "host.invalid#secret",
        "host\ninvalid",
        "host invalid",
    ] {
        assert_eq!(
            HostName::new(host).unwrap_err().kind(),
            FailureKind::InvalidHost
        );
    }
    assert_eq!(
        ResolutionQuota::new(0).unwrap_err().kind(),
        FailureKind::InvalidQuota
    );
    assert_eq!(
        ResolutionQuota::new(MAX_NATIVE_RESOLUTIONS + SINGLE_SLOT)
            .unwrap_err()
            .kind(),
        FailureKind::InvalidQuota
    );
    assert!(
        !format!("{:?}", HostName::new("private-host.invalid").unwrap())
            .contains("private-host.invalid")
    );
    let first = "127.0.0.1:0".parse().unwrap();
    let second = "[::1]:0".parse().unwrap();
    let result = worker::collect([second, first, second].into_iter()).unwrap();
    assert_eq!(result.iter().collect::<Vec<_>>(), vec![second, first]);
    assert_eq!(
        worker::collect(std::iter::empty()).unwrap_err().kind(),
        FailureKind::Empty
    );
    assert!(worker::collect(std::iter::repeat_n(first, MAX_RESOLVED_ADDRESSES)).is_ok());
    assert_eq!(
        worker::collect(std::iter::repeat_n(
            first,
            MAX_RESOLVED_ADDRESSES + SINGLE_SLOT
        ))
        .unwrap_err()
        .kind(),
        FailureKind::TooManyAddresses
    );
    safe_chain(&Failure::native(std::io::Error::other(CANARY)));
}
