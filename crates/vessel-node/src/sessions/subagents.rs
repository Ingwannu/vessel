//! Durable detach publication and the current-parent authorization boundary.
//!
//! The metadata lock linearizes ancestry admission against receipt commit and
//! live publication. Reading authoritative stored metadata additionally refuses
//! a corrupt receipt even when the live actor still has an earlier snapshot.

use chrono::Utc;
use std::sync::Arc;
use vessel_proto::{KeptMarks, SessionInfo, SessionRef, subagents::DetachedSession};
use vessel_session::SessionHandle;
use vessel_store::StoreError;

use super::{Found, Sessions};
use crate::error::ServiceError;

impl Sessions {
    /// Admission is linearized against the entire detach commit/publication.
    pub(crate) async fn check_parentage(
        &self,
        here: vessel_proto::NodeId,
        caller: SessionRef,
        target: vessel_proto::SessionId,
    ) -> Result<(), ServiceError> {
        let _admission = self.marking.lock().await;
        let mut current = target;
        for _step in 0..=crate::agents::MAX_DEPTH {
            let info = self.get(current).await?;
            let Some(parent) = self.current_parent(&info).await? else {
                break;
            };
            if parent == caller {
                return Ok(());
            }
            if parent.node != here {
                break;
            }
            current = parent.session;
        }
        Err(ServiceError::NotOwned(target))
    }

    /// Called only while the whole ancestry admission holds the metadata lock.
    async fn current_parent(&self, live: &SessionInfo) -> Result<Option<SessionRef>, ServiceError> {
        let stored = self.store.session(live.id).await?;
        // A newly visible creation may not have a committed row yet. No detach
        // can commit before that row exists, so its initial audit is authoritative.
        let info = stored.as_ref().unwrap_or(live);
        info.parentage()
            .map(vessel_proto::subagents::Parentage::parent)
            .map_err(|_invalid| StoreError::InvalidParentage(info.id).into())
    }

    pub(crate) async fn detach(
        self: &Arc<Self>,
        id: vessel_proto::SessionId,
    ) -> Result<DetachedSession, ServiceError> {
        let sessions = Arc::clone(self);
        self.tasks
            .spawn(async move {
                let _order = sessions.marking.lock().await;
                // Find again under the publication/resume lock. Publication survives
                // loss of the RPC task, as marks and creation already require.
                let found = sessions.find(id).await?;
                let mut result = sessions.store.detach(id, Utc::now()).await?;
                sessions.agents.detach(id);
                if let Found::Live { handle, .. } = found {
                    handle.set_detach_receipt(Some(result.receipt));
                    result.session = Box::new(handle.info());
                }
                Ok(result)
            })
            .await
            .map_err(ServiceError::Task)?
    }

    /// A resume may have prepared its actor from metadata read before detach.
    /// Re-read under the same metadata lock before opening its publication gate.
    pub(super) async fn restore_metadata(
        &self,
        handle: &SessionHandle,
        kept: KeptMarks,
    ) -> Result<KeptMarks, StoreError> {
        let info = self
            .store
            .session(handle.id())
            .await?
            .ok_or(StoreError::UnknownSession(handle.id()))?;
        handle.set_detach_receipt(info.detach_receipt);
        handle.set_marks(kept);
        if info.detach_receipt.is_some() {
            self.agents.detach(handle.id());
        }
        Ok(kept)
    }
}
