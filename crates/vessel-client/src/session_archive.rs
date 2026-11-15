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
