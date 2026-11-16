//! Preserve exact inbox-mark acknowledgement publication if the RPC caller leaves.
//! The service keeps authority checks; this only reuses metadata/reopen ordering.

use super::Sessions;
use crate::error::ServiceError;
use std::sync::Arc;
use vessel_proto::inbox::MarkInbox;

impl Sessions {
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
                let committed = sessions.store.mark_inbox_committed(mark.clone()).await?;
                if let Some(handle) = sessions.runtime_handle(mark.key.session) {
                    handle.acknowledge_time(committed.updated_at());
                }
                Ok(())
            })
            .await
            .map_err(ServiceError::Task)?
    }
}
