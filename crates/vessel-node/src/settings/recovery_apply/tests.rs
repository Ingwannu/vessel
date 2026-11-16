//! Actual Store settlement and debt; engine admission cases stay on its branch.
use super::*;
use crate::settings::writer_tests::{WRITE_TEST_BUDGET, phase};
use vessel_store::{DataDir, StoreConfig};

struct Fixture {
    _directory: tempfile::TempDir,
    policy: Arc<Policy>,
    apply: RecoveryApply,
}
impl Fixture {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let data = DataDir::new(directory.path().join("data"));
        data.prepare().unwrap();
        let store = Store::open(&data, StoreConfig::default())
            .await
            .unwrap()
            .store;
        let policy = Arc::new(Policy::new(on()));
        let apply = RecoveryApply::new(Arc::clone(&policy), store);
        Self {
            _directory: directory,
            policy,
            apply,
        }
    }
}
fn on() -> RestartRecovery {
    RestartRecovery {
        enabled: true,
        ..RestartRecovery::default()
    }
}

#[tokio::test]
async fn effective_current_tracks_saved_value_and_actual_on_store_settlement() {
    vessel_testkit::within(WRITE_TEST_BUDGET, async {
        let f = phase("open real Store fixture", Fixture::new()).await;
        assert_eq!(f.apply.current(), on());
        f.apply.saved(on());
        assert!(f.policy.needs_sync());
        phase("settle On SQL", f.apply.settle_saved(&on()))
            .await
            .unwrap();
        assert!(!f.policy.needs_sync());
        assert_eq!(f.apply.current(), on());
        phase("close real Store", f.apply.store.close()).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn real_cleanup_failure_retains_debt_and_blocks_following_settlement() {
    vessel_testkit::within(WRITE_TEST_BUDGET, async {
        let f = phase("open real Store fixture", Fixture::new()).await;
        phase("close real Store", f.apply.store.close()).await;
        f.apply.saved(RestartRecovery::default());
        assert!(matches!(
            phase(
                "settle saved Off SQL",
                f.apply.settle_saved(&RestartRecovery::default())
            )
            .await,
            Err(RecoveryApplyError::Cleanup(StoreError::Closed))
        ));
        assert!(f.policy.needs_cleanup());
        assert!(matches!(
            phase("settle Off cleanup debt", f.apply.settle_before()).await,
            Err(RecoveryApplyError::Cleanup(StoreError::Closed))
        ));
        assert!(f.policy.needs_cleanup());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn real_on_apply_failure_stays_paused_and_same_value_does_not_succeed() {
    vessel_testkit::within(WRITE_TEST_BUDGET, async {
        let f = phase("open real Store fixture", Fixture::new()).await;
        phase("close real Store", f.apply.store.close()).await;
        f.apply.saved(on());
        for () in [(), ()] {
            assert!(matches!(
                phase("settle On SQL", f.apply.settle_saved(&on())).await,
                Err(RecoveryApplyError::Apply(StoreError::Closed))
            ));
            assert!(f.policy.needs_sync());
        }
    })
    .await
    .unwrap();
}
