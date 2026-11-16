//! Incarnation invalidation shares the actual Agents registry lock.
//! Capture/atomic actor queue policy is a separate implementation checkpoint;
//! no liveness read or serialized stamp is upgraded into authority here.

use super::Agents;
use tokio::sync::watch;
use vessel_proto::timeline::ExecutionInvalidation;
use vessel_proto::{ExecutionIncarnationId, SessionId};

// Staged until the finish-admission actor capsule joins this runtime capture.
#[cfg(test)]
mod capture;
#[cfg(test)]
pub(crate) use capture::{CaptureRefusal, CapturedParentExecution, ExecutionObservation};

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

    pub(super) fn invalidate(&self, cause: ExecutionInvalidation) {
        let next = Change {
            incarnation: ExecutionIncarnationId::random(),
            cause: Some(cause),
        };
        let _previous = self.changed.send_replace(next);
    }
}

impl Agents {
    /// Transition owners call after authorization/state validation, before the
    /// transition's first effect. No captured intent may recapture the new UUID.
    pub(crate) fn invalidate_execution(&self, session: SessionId, cause: ExecutionInvalidation) {
        if cause == ExecutionInvalidation::Ended {
            self.retire_execution(session);
            return;
        }
        let change = {
            let entries = self.lock();
            entries.get(&session).map(|entry| {
                entry.execution.invalidate(cause);
                *entry.execution.changed.borrow()
            })
        };
        // Diagnostics run after releasing the registry, never inside an admission callback.
        if let Some(change) = change {
            tracing::debug!(%session, incarnation=%change.incarnation, cause=?change.cause, "agent execution invalidated");
        }
    }
    fn retire_execution(&self, session: SessionId) {
        let entry = {
            let mut entries = self.lock();
            let Some(entry) = entries.remove(&session) else {
                return;
            };
            // The removal/final change has one current-incarnation order:
            // no capture or same-id replacement can interleave between them.
            entry.execution.invalidate(ExecutionInvalidation::Ended);
            entry
        };
        for closing in entry.connections.values() {
            closing.cancel();
        }
        if !entry.connections.is_empty() {
            tracing::info!(%session, connections=entry.connections.len(), "the session ended; closed its agent's connections");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::principal::Principal;
    use vessel_proto::NodeId;

    #[test]
    fn every_transition_changes_the_exact_entry_and_publishes_to_existing_receivers() {
        let agents = Agents::new(NodeId::random());
        let session = SessionId::random();
        let other = SessionId::random();
        let _secret = agents.reserve(session, &Principal::Owner, false).unwrap();
        let _other = agents.reserve(other, &Principal::Owner, false).unwrap();
        let (mut watched, other_id) = {
            let entries = agents.lock();
            let watched = entries.get(&session).unwrap().execution.changed.subscribe();
            let other_id = entries
                .get(&other)
                .unwrap()
                .execution
                .changed
                .borrow()
                .incarnation;
            (watched, other_id)
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
            assert!(watched.has_changed().unwrap());
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
        assert_eq!(
            watched.borrow_and_update().cause,
            Some(ExecutionInvalidation::Ended)
        );
        assert!(watched.has_changed().is_err());
    }

    #[test]
    fn replacing_the_same_session_id_never_reuses_the_old_watch_or_incarnation() {
        let agents = Agents::new(NodeId::random());
        let id = SessionId::random();
        let _first = agents.reserve(id, &Principal::Owner, false).unwrap();
        let old = agents
            .lock()
            .get(&id)
            .unwrap()
            .execution
            .changed
            .subscribe();
        let old_id = old.borrow().incarnation;
        agents.forget(id);
        assert!(old.has_changed().is_err());
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
    #[test]
    fn profile_replacement_invalidates_with_or_without_an_mcp_credential() {
        for with_credential in [false, true] {
            let agents = Agents::new(NodeId::random());
            let id = SessionId::random();
            let _secret = agents
                .reserve(id, &Principal::Owner, with_credential)
                .unwrap();
            let mut watched = agents
                .lock()
                .get(&id)
                .unwrap()
                .execution
                .changed
                .subscribe();
            let before = watched.borrow().incarnation;
            let _next = agents.begin_move(id).unwrap();
            assert!(watched.has_changed().unwrap());
            let changed = *watched.borrow_and_update();
            assert_ne!(changed.incarnation, before);
            assert_eq!(changed.cause, Some(ExecutionInvalidation::DriverReplaced));
            if with_credential {
                assert!(matches!(
                    agents.begin_move(id),
                    Err(crate::error::ServiceError::MoveRunning(_))
                ));
                assert!(!watched.has_changed().unwrap());
            }
            // A failed replacement may restore its old credentials, never its
            // old captured execution. Credential settlement cannot resurrect it.
            agents.end_move(id, false);
            assert_eq!(watched.borrow().incarnation, changed.incarnation);
            assert!(!watched.has_changed().unwrap());
        }
    }
}
