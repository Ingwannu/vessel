//! Test-only pause immediately before the shared final admission arbiter.
use super::Recovery;
use tokio::sync::oneshot;
pub(super) struct Admission {
    entered: oneshot::Sender<()>,
    release: std::sync::mpsc::Receiver<()>,
}
impl std::fmt::Debug for Admission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Admission").finish_non_exhaustive()
    }
}
impl Recovery {
    pub(crate) fn pause_admission(
        &self,
    ) -> (oneshot::Receiver<()>, std::sync::mpsc::SyncSender<()>) {
        let (entered, waiting) = oneshot::channel();
        let (release, held) = std::sync::mpsc::sync_channel(1);
        *self.admission_gate.lock().unwrap() = Some(Admission {
            entered,
            release: held,
        });
        (waiting, release)
    }
    pub(super) fn before_admission(&self) {
        let gate = self.admission_gate.lock().unwrap().take();
        if let Some(gate) = gate {
            let _gone = gate.entered.send(());
            gate.release.recv().unwrap();
        }
    }
    pub(crate) fn attempt_cancelled(
        &self,
        session: vessel_proto::SessionId,
    ) -> tokio_util::sync::CancellationToken {
        let state = self.lock();
        let Some(attempt) = state.attempts.get(&session) else {
            panic!("no startup attempt owns the test session")
        };
        attempt.cancelled.clone()
    }
    pub(crate) fn boot_done(&self) -> tokio::sync::watch::Receiver<bool> {
        self.finished.subscribe()
    }
    pub(crate) fn finish_boot(&self) {
        let _old = self.finished.send_replace(true);
    }
}

#[test]
fn off_then_on_cannot_rebase_a_claimed_but_not_yet_executed_boot_attempt() {
    use super::{Claim, policy::Policy};
    use std::sync::Arc;
    use vessel_proto::restart::RestartRecovery;
    let policy = Arc::new(Policy::new(RestartRecovery {
        enabled: true,
        ..RestartRecovery::default()
    }));
    let recovery = Arc::new(Recovery::new(Arc::clone(&policy)));
    let original_generation = recovery.boot_generation;
    let Claim::Owned(attempt) = recovery.claim(vessel_proto::SessionId::random()) else {
        panic!("one attempt")
    };
    // The future holding this claimed attempt has not been polled yet.
    policy.apply(RestartRecovery::default());
    policy.cleaned();
    policy.apply(RestartRecovery {
        enabled: true,
        ..RestartRecovery::default()
    });
    policy.cleaned();
    assert_ne!(policy.snapshot().1, original_generation);
    assert_eq!(recovery.boot_generation, original_generation);
    let mut dispatched = 0;
    {
        let mut effect = Some(|_: &RestartRecovery| {
            dispatched += 1;
            Ok(())
        });
        assert_eq!(
            attempt
                .admit(recovery.boot_generation, &mut effect)
                .unwrap_err()
                .code,
            vessel_proto::ErrorCode::Conflict
        );
        assert!(
            effect.is_some(),
            "the unsent one-shot effect was never consumed"
        );
    }
    assert_eq!(dispatched, 0);
}

#[test]
fn attempt_pool_releases_capacity_and_notifies_waiters() {
    use super::{Claim, policy::Policy};
    use std::sync::Arc;
    use vessel_proto::{SessionId, restart::RestartRecovery};
    let recovery = Arc::new(Recovery::new(Arc::new(Policy::new(
        RestartRecovery::default(),
    ))));
    let mut held = Vec::new();
    for _ in 0..Recovery::MAX_ATTEMPTS {
        let Claim::Owned(attempt) = recovery.claim(SessionId::random()) else {
            panic!("capacity below the named attempt bound")
        };
        held.push(attempt);
    }
    let waiting_id = SessionId::random();
    let Claim::Wait(changed) = recovery.claim(waiting_id) else {
        panic!("attempt bound was exceeded")
    };
    drop(held.pop().unwrap());
    assert!(changed.has_changed().unwrap());
    assert!(matches!(recovery.claim(waiting_id), Claim::Owned(_)));
}

#[tokio::test]
async fn operation_pool_refuses_without_waiters_and_returns_released_capacity() {
    use super::{Intervention, policy::Policy};
    use std::sync::Arc;
    use vessel_proto::{ErrorCode, SessionId, restart::RestartRecovery};
    let recovery = Arc::new(Recovery::new(Arc::new(Policy::new(
        RestartRecovery::default(),
    ))));
    let mut held = Vec::new();
    for _ in 0..Recovery::MAX_OPERATIONS {
        held.push(
            recovery
                .before_operation(SessionId::random(), Intervention::Wait)
                .await
                .unwrap(),
        );
    }
    let session = SessionId::random();
    let error = recovery
        .before_operation(session, Intervention::Wait)
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
    assert_eq!(
        error.message,
        format!(
            "the node has {} session operations in flight; retry after one finishes",
            Recovery::MAX_OPERATIONS
        )
    );
    drop(held.pop().unwrap());
    let _admitted = recovery
        .before_operation(session, Intervention::Wait)
        .await
        .unwrap();
}
