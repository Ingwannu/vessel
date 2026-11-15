//! Runner-issued observations distinguish actual runs even when PIDs repeat.
//! No observation proves current kernel liveness, socket ownership or permission.

use std::sync::Arc;

use tokio::sync::watch;

use crate::ProcessId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Live,
    Revoked,
}

#[derive(Debug)]
struct Run {
    pid: ProcessId,
    state: watch::Sender<State>,
}

/// Read-only observation issued by a process runner for exactly one run.
/// It cannot be decoded from a PID, URL or wire payload. Revocation may precede
/// OS exit (including a refused signal); fresh OS inspection is still required.
#[derive(Debug, Clone)]
pub struct ProcessLifetime(Arc<Run>);

impl ProcessLifetime {
    /// The runner's checked PID; PID equality is not run equality.
    #[must_use]
    pub fn pid(&self) -> ProcessId {
        self.0.pid
    }

    /// Whether both observations originated from the same runner issuance.
    #[must_use]
    pub fn same_run(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    /// Whether the runner has not revoked this observation, not an OS probe.
    #[must_use]
    pub fn is_live(&self) -> bool {
        *self.0.state.borrow() == State::Live
    }

    /// Wait for authority revocation, including when it happened before this call.
    /// This does not report a process exit code or certify descendant cleanup.
    pub async fn ended(&self) {
        let mut state = self.0.state.subscribe();
        let _ended = state.wait_for(|state| *state == State::Revoked).await;
    }
}

/// Revocation authority owned only by actual runners and their waiter threads.
/// Any guard's drop revokes every observer of that run; observers cannot mint it.
#[derive(Debug)]
#[must_use = "dropping the runner guard revokes the process observation"]
pub struct ProcessLifetimeGuard(Arc<Run>);

impl ProcessLifetimeGuard {
    /// Issue a run only at successful process creation, never from a PID snapshot.
    pub fn begin(pid: ProcessId) -> Self {
        Self(Arc::new(Run {
            pid,
            state: watch::Sender::new(State::Live),
        }))
    }

    /// Expose observation without giving a consumer revocation authority.
    #[must_use]
    pub fn observer(&self) -> ProcessLifetime {
        ProcessLifetime(Arc::clone(&self.0))
    }

    /// End authority before signalling or once exit/wait uncertainty is observed.
    pub fn revoke(&self) {
        // No receiver may exist yet: retain revocation for late subscribers too.
        let _previous = self.0.state.send_replace(State::Revoked);
    }

    /// Give an actual waiter a guard whose return or unwind also revokes the run.
    /// This is deliberately not Clone: a temporary copy must not look harmless.
    pub fn end_on_drop(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl Drop for ProcessLifetimeGuard {
    fn drop(&mut self) {
        self.revoke();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    const SAFETY_NET: Duration = Duration::from_secs(30);

    #[tokio::test]
    async fn same_pid_on_a_new_run_never_revives_an_old_observation() {
        let first = ProcessLifetimeGuard::begin(ProcessId::MIN);
        let old = first.observer();
        assert!(old.same_run(&old.clone()));
        assert!(old.is_live());
        drop(first);
        tokio::time::timeout(SAFETY_NET, old.ended()).await.unwrap();
        assert!(!old.is_live());
        let second = ProcessLifetimeGuard::begin(old.pid());
        let new = second.observer();
        assert_eq!(old.pid(), new.pid());
        assert!(!old.same_run(&new));
        assert!(new.is_live());
        assert!(!old.is_live());
    }

    #[tokio::test]
    async fn a_waiter_guard_or_explicit_revocation_wakes_early_and_late_observers() {
        let owner = ProcessLifetimeGuard::begin(ProcessId::MIN);
        let observer = owner.observer();
        let mut waiting = Box::pin(observer.ended());
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(
                waiting.as_mut().poll(cx).is_pending()
            ))
            .await
        );
        drop(owner.end_on_drop());
        tokio::time::timeout(SAFETY_NET, waiting).await.unwrap();
        tokio::time::timeout(SAFETY_NET, observer.ended())
            .await
            .unwrap();
        assert!(!owner.observer().is_live());
        owner.revoke();
        tokio::time::timeout(SAFETY_NET, observer.ended())
            .await
            .unwrap();
    }
}
