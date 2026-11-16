//! Name the failing lifecycle phase without extending the original total budget.
use std::{future::Future, time::Duration};
use tokio::time::Instant;
use vessel_proto::WorkspaceId;

const TOTAL_WAIT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy)]
enum Phase {
    Setup,
    RecoveryAck,
    WriteAck,
    WatcherWake,
    Read,
    FailedCas,
    Close,
}

// All phases share ONE deadline. Resetting a ten-second timeout per phase
// would silently enlarge the original test budget and conceal contention.
async fn phase<T>(deadline: Instant, name: Phase, work: impl Future<Output = T>) -> T {
    let remaining = deadline.saturating_duration_since(Instant::now());
    assert!(
        !remaining.is_zero(),
        "registry invalidation phase {name:?} began after its total budget"
    );
    let result = tokio::time::timeout_at(deadline, work)
        .await
        .unwrap_or_else(|_elapsed| {
            panic!("registry invalidation phase {name:?} exhausted remaining {remaining:?}")
        });
    assert!(
        Instant::now() <= deadline,
        "registry invalidation phase {name:?} exceeded its total budget"
    );
    result
}

pub(super) async fn check() {
    let deadline = Instant::now()
        .checked_add(TOTAL_WAIT)
        .expect("bounded registry invalidation deadline");
    let (_temp, store) = phase(deadline, Phase::Setup, async {
        let temp = tempfile::tempdir().unwrap();
        let data = crate::DataDir::new(temp.path().join("data"));
        data.prepare().unwrap();
        let store = crate::Store::open(&data, crate::StoreConfig::default())
            .await
            .unwrap()
            .store;
        (temp, store)
    })
    .await;
    let mut changes = store.registry_changes();
    assert!(!changes.has_changed().unwrap());
    phase(
        deadline,
        Phase::RecoveryAck,
        store.recover_registry_reservations(),
    )
    .await
    .unwrap();
    assert!(!changes.has_changed().unwrap());
    let record = super::record(WorkspaceId::random());
    phase(
        deadline,
        Phase::WriteAck,
        store.put_registry_workspace(record.clone(), None),
    )
    .await
    .unwrap();
    phase(deadline, Phase::WatcherWake, changes.changed())
        .await
        .unwrap();
    let fresh = store.registry_changes();
    assert!(
        !fresh.has_changed().unwrap(),
        "a fresh subscriber must not inherit historical invalidations"
    );
    assert_eq!(
        phase(
            deadline,
            Phase::Read,
            store.registry_workspace(record.descriptor.id)
        )
        .await
        .unwrap(),
        Some(record.clone())
    );
    assert!(matches!(
        phase(
            deadline,
            Phase::FailedCas,
            store.put_registry_workspace(record, None)
        )
        .await,
        Err(crate::StoreError::RegistryConflict)
    ));
    assert!(!changes.has_changed().unwrap());
    phase(deadline, Phase::Close, store.close()).await;
}
