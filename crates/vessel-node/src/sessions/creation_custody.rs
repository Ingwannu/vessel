//! Bound-only custody after actual spawn; this grants no launch permission.
//! Original parent observation moves into the existing child persistence owner.
use super::*;
use crate::agents::{CapturedParentExecution, ExecutionObservation};

pub(crate) struct RegisteredBoundChild {
    pub(crate) handle: SessionHandle,
    // A historical observation, not permission or proof of physical release.
    pub(crate) parent_at_registration: ExecutionObservation,
}

impl Sessions {
    /// The tracked creation driver must already own actual spawn and its count,
    /// placement/start and Begin. This lower join takes only an actual Created;
    /// it never starts a process or authorizes an unadmitted command.
    pub(crate) async fn keep_bound_child(
        self: &Arc<Self>,
        created: Created,
        parent: CapturedParentExecution,
    ) -> Result<RegisteredBoundChild, ServiceError> {
        let Created { handle, durable } = created;
        let id = handle.id();
        let durability = self.track(&handle, EventSeq::ZERO);
        if let Err(error) = self.store.create_session(handle.info()).await {
            durability.send_modify(|state| *state = Durability::Broken);
            handle.kill();
            self.forget(id);
            drop(durable);
            let sessions = Arc::clone(self);
            drop(self.tasks.spawn(async move {
                // Failure cannot release custody merely because its reply is
                // gone. Actual ended/hand-over remains owned, including watch.
                sessions.hand_over(handle, Persisted::Incomplete).await;
                drop(parent);
            }));
            return Err(error.into());
        }
        let parent_at_registration = parent.observation();
        self.persist_bound(handle.clone(), durable, durability, parent);
        Ok(RegisteredBoundChild {
            handle,
            parent_at_registration,
        })
    }

    fn persist_bound(
        self: &Arc<Self>,
        handle: SessionHandle,
        durable: mpsc::Receiver<Arc<EventRecord>>,
        durability: watch::Sender<Durability>,
        mut parent: CapturedParentExecution,
    ) {
        let sessions = Arc::clone(self);
        let store = self.store.clone();
        drop(self.tasks.spawn(async move {
            let persistence = persist::run(store, handle.clone(), durable, durability);
            tokio::pin!(persistence);
            // The original watch is moved, never cloned/recaptured. A change
            // before this task's first poll remains pending in that receiver.
            // Watch/grant loss fails closed; Current is only observation here.
            let invalidation = async {
                match parent.observation() {
                    ExecutionObservation::Current => parent.invalidated().await,
                    changed @ (ExecutionObservation::Invalidated(_)
                    | ExecutionObservation::Unavailable) => changed,
                }
            };
            let persisted = tokio::select! {
                biased;
                _revoked = invalidation => {
                    handle.kill();
                    persistence.await
                }
                persisted = &mut persistence => persisted,
            };
            sessions.hand_over(handle, persisted).await;
        }));
    }
}

#[cfg(test)]
mod tests;
