//! Observe actual primitive transitions without installing an engine facade.
use super::*;
fn on() -> RestartRecovery {
    RestartRecovery {
        enabled: true,
        ..RestartRecovery::default()
    }
}
#[test]
fn failed_off_save_preserves_current_policy_and_durable_generation() {
    let policy = Policy::new(on());
    let generation = policy.lock().generation;
    policy.begin_write(&RestartRecovery::default());
    assert!(policy.lock().off_saving);
    assert_eq!(policy.current(), on());
    policy.write_failed();
    assert!(!policy.lock().off_saving);
    assert_eq!(policy.lock().generation, generation);
    assert_eq!(policy.current(), on());
    assert!(!policy.needs_sync());
    policy.apply(RestartRecovery::default());
    assert_ne!(policy.lock().generation, generation);
    assert!(policy.needs_cleanup());
}
#[test]
fn a_late_saved_on_and_cleanup_cannot_reopen_the_stopping_primitive() {
    let policy = Policy::new(on());
    policy.seal_effect_admission();
    policy.seal_effect_admission();
    policy.begin_write(&RestartRecovery::default());
    policy.write_failed();
    policy.apply(on());
    assert_eq!(policy.current(), on());
    assert!(policy.needs_sync());
    policy.cleaned();
    assert!(!policy.needs_sync());
    assert_eq!(policy.lock().admission, Admission::Sealed);
    // This is private state evidence, never a claim of engine/helper execution.
}
