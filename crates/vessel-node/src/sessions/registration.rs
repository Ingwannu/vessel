//! Physical launch evidence is retained independently of a mutable cwd pathname.
use super::{Durability, Sessions};
use std::{
    collections::HashMap,
    sync::{PoisonError, RwLock},
};
use tokio::sync::watch;
#[cfg(target_os = "linux")]
use vessel_proto::checkpoint::CheckpointError;
use vessel_proto::{EventSeq, SessionId, checkpoint::PhysicalRoot};
use vessel_session::SessionHandle;
#[cfg(target_os = "linux")]
use vessel_session::SessionRuntime;

#[derive(Debug, Default)]
pub(super) struct BoundRoots(RwLock<HashMap<SessionId, PhysicalRoot>>);
impl BoundRoots {
    fn remember(&self, id: SessionId, root: Option<PhysicalRoot>) {
        let mut roots = self.0.write().unwrap_or_else(PoisonError::into_inner);
        if let Some(root) = root {
            let _old = roots.insert(id, root);
        } else {
            let _old = roots.remove(&id);
        }
    }
    pub(super) fn forget(&self, id: SessionId) {
        let _old = self
            .0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&id);
    }
    #[cfg(target_os = "linux")]
    pub(super) fn capture(
        &self,
        id: SessionId,
        runtime: &SessionRuntime,
    ) -> Result<PhysicalRoot, CheckpointError> {
        // Copy before consulting runtime handles: never nest the registry locks.
        let roots = self
            .0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let root = roots
            .get(&id)
            .copied()
            .ok_or(CheckpointError::UnsupportedScope)?;
        for other in runtime
            .list()
            .into_iter()
            .filter(|info| info.id != id && info.status.is_live())
        {
            match roots.get(&other.id) {
                Some(found) if *found == root => return Err(CheckpointError::UnsupportedScope),
                Some(_) => {}
                None => return Err(CheckpointError::Busy),
            }
        }
        Ok(root)
    }
}
impl Sessions {
    /// Publish physical startup evidence before node visibility/durability wakeup.
    pub(super) fn track_root(
        &self,
        handle: &SessionHandle,
        stored: EventSeq,
        root: Option<PhysicalRoot>,
    ) -> watch::Sender<Durability> {
        let id = handle.id();
        self.roots.remember(id, root);
        self.agents.started(id, handle.clone());
        self.models.started(id, handle.clone());
        let (durability, watching) = watch::channel(Durability::Through(stored));
        let previous = self
            .durability
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id, watching);
        #[cfg(target_os = "linux")]
        self.registrations.notify_waiters();
        if previous.is_some() {
            tracing::error!(session=%id,"session id created twice");
        }
        durability
    }
    #[cfg(test)]
    pub(super) fn track(
        &self,
        handle: &SessionHandle,
        stored: EventSeq,
    ) -> watch::Sender<Durability> {
        self.track_root(handle, stored, None)
    }
}
