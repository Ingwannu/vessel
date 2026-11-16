//! Preserve exact inbox-mark acknowledgement publication if the RPC caller leaves.
//! The service keeps authority checks; this only reuses metadata/reopen ordering.

use super::Sessions;
use crate::error::ServiceError;
use std::sync::Arc;
use vessel_proto::inbox::MarkInbox;
use vessel_store::Residence;

impl Sessions {
    /// Keep an admitted mark's committed clock publication despite caller loss.
    pub(crate) async fn mark_inbox(self: &Arc<Self>, mark: MarkInbox) -> Result<(), ServiceError> {
        // Cancelled callers awaiting admission retain no detached task. Once
        // admitted, exactly this owned order survives with commit/publication.
        let order = self.marking_lease().await;
        let sessions = Arc::clone(self);
        self.tasks
            .spawn(async move {
                // Reopen/detach share this order. Once the write starts, the task
                // retains its committed clock publication despite RPC cancellation.
                let _order = order;
                // Read under the order a reopen also takes: a stored session
                // stays stored until this write commits (TRACKER 775).
                let handle = sessions.runtime_handle(mark.key.session);
                let residence = if handle.is_some() {
                    Residence::Loaded
                } else {
                    Residence::Stored
                };
                let committed = sessions
                    .store
                    .mark_inbox_committed(mark, residence)
                    .await?;
                if let Some(handle) = handle {
                    handle.acknowledge_time(committed.updated_at());
                }
                Ok(())
            })
            .await
            .map_err(ServiceError::Task)?
    }
}
