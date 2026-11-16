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
        if cause == ExecutionInvalidation::Ended {
            self.retire_execution(session);
            return;
        }
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
    fn retire_execution(&self, session: SessionId) {
        let entry = {
            let mut entries = self.lock();
            let Some(entry) = entries.remove(&session) else {
                return;
            };
            // The removal/final change has one current-incarnation order:
            // no capture or same-id replacement can interleave between them.
            let _change = entry.execution.invalidate(ExecutionInvalidation::Ended);
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

/// Diagnostic subscription to the existing execution watch; never admission.
#[cfg(test)]
pub(crate) struct ExecutionTestWatch {
    changed: watch::Receiver<Change>,
}

#[cfg(test)]
impl ExecutionTestWatch {
    pub(crate) fn current(&self) -> (ExecutionIncarnationId, Option<ExecutionInvalidation>) {
        let change = *self.changed.borrow();
        (change.incarnation, change.cause)
    }

    pub(crate) fn has_changed(&self) -> Result<bool, watch::error::RecvError> {
        self.changed.has_changed()
    }

    pub(crate) fn mark_seen(&mut self) {
        let _seen = self.changed.borrow_and_update();
    }
}

#[cfg(test)]
impl Agents {
    pub(crate) fn watch_execution_for_test(&self, id: SessionId) -> Option<ExecutionTestWatch> {
        self.lock().get(&id).map(|entry| ExecutionTestWatch {
            changed: entry.execution.changed.subscribe(),
        })
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
}
