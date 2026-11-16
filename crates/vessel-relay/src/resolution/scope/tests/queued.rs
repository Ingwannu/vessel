//! A real Tokio blocking-pool queue, distinct from a held native-start gate.
use super::*;

#[test]
fn closed_admitted_lookup_stays_owned_while_queued_in_the_existing_blocking_pool() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(SINGLE_SLOT)
        .max_blocking_threads(SINGLE_SLOT)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(within(async {
        let budget = ResolutionBudget::default();
        let scope = scope(&budget, SINGLE_SLOT);
        let pool_gate = Gate::new();
        let release_pool = Release(Arc::clone(&pool_gate));
        let (started, entered) = oneshot::channel();
        let mut blocker = Task(Some(tokio::task::spawn_blocking(move || {
            started.send(()).unwrap();
            pool_gate.wait();
        })));
        phase("one existing blocking slot occupied", entered)
            .await
            .unwrap();
        let (mut held, mut probes) = Held::probes(Stage::BeforeStart, SINGLE_SLOT);
        held.release.0.release();
        let host = HostName::new("127.0.0.1").unwrap();
        let handle = scope.handle();
        let mut queued = Box::pin(handle.start(&host, ZERO_PORT, probes.pop()));
        assert!(queued.as_mut().now_or_never().is_none());
        assert_eq!(scope.0.state().active, SINGLE_SLOT);
        assert_eq!(
            budget.0.available_permits(),
            MAX_NATIVE_RESOLUTIONS - SINGLE_SLOT
        );
        assert!(held.ready.try_recv().is_err());
        scope.close();
        assert_eq!(scope.state(), ScopeState::Draining);
        release_pool.0.release();
        blocker.joined().await.unwrap();
        held.entered(SINGLE_SLOT).await;
        assert_eq!(
            phase("actual queued lookup skips native work after close", queued)
                .await
                .unwrap_err()
                .kind(),
            FailureKind::Closed
        );
        assert!(!held.entered.load(Ordering::SeqCst));
        let _receipt = phase("queued blocking join actually drained", scope.drain())
            .await
            .unwrap();
        assert_eq!(budget.0.available_permits(), MAX_NATIVE_RESOLUTIONS);
    }));
}
