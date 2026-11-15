//! Policy fences and actual Store failures, without a second settings writer.
use super::*;
use vessel_proto::ErrorCode;
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
async fn failed_off_save_restores_admission_but_shutdown_seal_never_reopens() {
    let f = Fixture::new().await;
    let (_, generation) = f.policy.snapshot();
    assert!(f.policy.check(generation).unwrap());
    f.apply.begin_write(&RestartRecovery::default());
    assert!(!f.policy.check(generation).unwrap());
    f.apply.write_failed();
    assert!(f.policy.check(generation).unwrap());
    f.apply.seal_effect_admission();
    f.apply.saved(on());
    f.apply.settle_saved(&on()).await.unwrap();
    f.apply.write_failed();
    let (_, generation) = f.policy.snapshot();
    let mut effect = Some(
        |_: &RestartRecovery| -> Result<(), vessel_proto::WireError> {
            panic!("a late saved On must not reopen final effect admission")
        },
    );
    assert_eq!(
        f.policy.admit(generation, &mut effect).unwrap_err().code,
        ErrorCode::Unavailable
    );
    assert!(effect.is_some());
    f.apply.store.close().await;
}

#[tokio::test]
async fn real_cleanup_failure_retains_debt_and_blocks_following_settlement() {
    let f = Fixture::new().await;
    f.apply.store.close().await;
    f.apply.saved(RestartRecovery::default());
    assert!(matches!(
        f.apply.settle_saved(&RestartRecovery::default()).await,
        Err(RecoveryApplyError::Cleanup(StoreError::Closed))
    ));
    assert!(f.policy.needs_cleanup());
    assert!(matches!(
        f.apply.settle_before().await,
        Err(RecoveryApplyError::Cleanup(StoreError::Closed))
    ));
    assert!(f.policy.needs_cleanup());
}

#[tokio::test]
async fn real_on_apply_failure_stays_paused_and_same_value_does_not_succeed() {
    let f = Fixture::new().await;
    f.apply.store.close().await;
    f.apply.saved(on());
    let (_, generation) = f.policy.snapshot();
    assert!(!f.policy.check(generation).unwrap());
    for _ in [(), ()] {
        assert!(matches!(
            f.apply.settle_saved(&on()).await,
            Err(RecoveryApplyError::Apply(StoreError::Closed))
        ));
        assert!(f.policy.needs_sync());
        assert!(!f.policy.check(generation).unwrap());
    }
}
