//! Atomically admitted archive; client-side turn snapshots are never authority.
use crate::NodeClient;
use vessel_proto::{
    SessionId,
    session_archive::{ArchiveCondition, ArchiveSession, ArchivedSession},
};
use vessel_rpc::CallError;

impl NodeClient {
    /// Archive an exact session, refusing an active turn unless Force was requested.
    /// # Errors
    /// Conflict for an active turn or already archived session; Unsupported for
    /// a pending/older method or idle-conditional terminal; otherwise `CallError`.
    pub async fn archive_session(
        &self,
        session: SessionId,
        condition: ArchiveCondition,
    ) -> Result<ArchivedSession, CallError> {
        self.call(ArchiveSession { session, condition }).await
    }
}

impl NodeClient {
    /// Archive only an idle/error child still attached to the exact expected parent.
    /// Route this request to child.node; a different node with that UUID is refused.
    /// # Errors
    /// Pending/older protocol, unsupported TUI idle admission or original RPC failure.
    pub async fn archive_child_session(
        &self,
        request: vessel_proto::session_archive::ArchiveChildSession,
    ) -> Result<vessel_proto::session_archive::ChildArchiveOutcome, CallError> {
        self.call(request).await
    }
}
