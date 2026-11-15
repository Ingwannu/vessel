//! Incarnation invalidation shares the actual Agents registry lock.
//! Capture/atomic actor queue policy is a separate implementation checkpoint;
//! no liveness read or serialized stamp is upgraded into authority here.

use super::Agents;
use tokio::sync::watch;
use vessel_proto::timeline::ExecutionInvalidation;
use vessel_proto::{ExecutionIncarnationId, SessionId};

#[derive(Debug, Clone, Copy)]
struct Change {
    incarnation: ExecutionIncarnationId,
    cause: Option<ExecutionInvalidation>,
}

#[derive(Debug)]
pub(super) struct Execution {
    changed: watch::Sender<Change>,
}

impl Execution {
    pub(super) fn new() -> Self {
        let (changed, _initial) = watch::channel(Change {
            incarnation: ExecutionIncarnationId::random(),
            cause: None,
        });
        Self { changed }
    }

    fn invalidate(&self, cause: ExecutionInvalidation) -> Change {
        let next = Change {
            incarnation: ExecutionIncarnationId::random(),
            cause: Some(cause),
        };
        let _previous = self.changed.send_replace(next);
        next
    }
}

impl Agents {
    /// Transition owners call after authorization/state validation, before the
    /// transition's first effect. No captured intent may recapture the new UUID.
    pub(crate) fn invalidate_execution(&self, session: SessionId, cause: ExecutionInvalidation) {
        let change = {
            let entries = self.lock();
            entries
                .get(&session)
                .map(|entry| entry.execution.invalidate(cause))
        };
        // Diagnostics run after releasing the registry, never inside an admission callback.
        if let Some(change) = change {
            tracing::debug!(%session, incarnation=%change.incarnation, cause=?change.cause, "agent execution invalidated");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::principal::Principal;
    use vessel_proto::NodeId;

    #[tokio::test]
    async fn every_transition_changes_the_exact_entry_and_publishes_to_existing_receivers() {
        let agents = Agents::new(NodeId::random());
        let session = SessionId::random();
        let other = SessionId::random();
        let _secret = agents.reserve(session, &Principal::Owner, false).unwrap();
        let _other = agents.reserve(other, &Principal::Owner, false).unwrap();
        let (mut watched, other_id) = {
            let entries = agents.lock();
            (
                entries.get(&session).unwrap().execution.changed.subscribe(),
                entries
                    .get(&other)
                    .unwrap()
                    .execution
                    .changed
                    .borrow()
                    .incarnation,
            )
        };
        let original_scopes = agents.scopes(session).unwrap();
        for cause in [
            ExecutionInvalidation::PersonStop,
            ExecutionInvalidation::Resume,
            ExecutionInvalidation::Archive,
            ExecutionInvalidation::Detach,
            ExecutionInvalidation::DriverReplaced,
            ExecutionInvalidation::GrantChanged,
        ] {
            let previous = watched.borrow().incarnation;
            agents.invalidate_execution(session, cause);
            watched.changed().await.unwrap();
            let now = *watched.borrow_and_update();
            assert_ne!(now.incarnation, previous);
            assert_eq!(now.cause, Some(cause));
        }
        assert_eq!(agents.scopes(session).unwrap(), original_scopes);
        assert_eq!(
            agents
                .lock()
                .get(&other)
                .unwrap()
                .execution
                .changed
                .borrow()
                .incarnation,
            other_id
        );
        agents.retire(session);
        watched.changed().await.unwrap();
        assert_eq!(
            watched.borrow_and_update().cause,
            Some(ExecutionInvalidation::Ended)
        );
        assert!(watched.changed().await.is_err());
    }

    #[tokio::test]
    async fn replacing_the_same_session_id_never_reuses_the_old_watch_or_incarnation() {
        let agents = Agents::new(NodeId::random());
        let id = SessionId::random();
        let _first = agents.reserve(id, &Principal::Owner, false).unwrap();
        let mut old = agents
            .lock()
            .get(&id)
            .unwrap()
            .execution
            .changed
            .subscribe();
        let old_id = old.borrow().incarnation;
        agents.forget(id);
        assert!(old.changed().await.is_err());
        let _next = agents.reserve(id, &Principal::Owner, false).unwrap();
        assert_ne!(
            agents
                .lock()
                .get(&id)
                .unwrap()
                .execution
                .changed
                .borrow()
                .incarnation,
            old_id
        );
    }
}
