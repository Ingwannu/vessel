//! Conditional archive preserves the node's admitted id and committed timestamp.
use super::{Connection, NodeService, wire};
use vessel_proto::{
    ErrorCode, WireError,
    session_archive::{ArchiveSession, ArchivedSession},
};

impl NodeService {
    pub(crate) async fn archive_session(
        &self,
        _: &Connection,
        request: ArchiveSession,
    ) -> Result<ArchivedSession, WireError> {
        let archived = self
            .sessions
            .archive_condition(request.session, request.condition)
            .await
            .map_err(|error| wire(&error))?;
        self.unwatch_session(request.session);
        self.terminals.end_beside(request.session).await;
        let Some(archived_at) = archived.marks.archived_at else {
            return Err(WireError::new(
                ErrorCode::Internal,
                "archive completed without a durable timestamp",
            ));
        };
        Ok(ArchivedSession {
            session: archived.id,
            archived_at,
        })
    }
}
