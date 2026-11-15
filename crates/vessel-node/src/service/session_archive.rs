//! Conditional archive preserves the node's admitted id and committed timestamp.
use super::{Connection, NodeService, wire};
use crate::{
    error::ServiceError,
    forge::{Command, Forges},
    terminals::Terminals,
};
use std::sync::Arc;
use vessel_proto::{
    ErrorCode, SessionInfo, WireError,
    session_archive::{ArchiveChildSession, ArchiveSession, ArchivedSession, ChildArchiveOutcome},
};

impl NodeService {
    pub(crate) async fn archive_child_session(
        &self,
        connection: &Connection,
        request: ArchiveChildSession,
    ) -> Result<ChildArchiveOutcome, WireError> {
        admit_child(connection.principal(), self.lifecycle.info().id, request)?;
        let completion = self.archive_completion();
        self.sessions
            .archive_child_with(request, move |info| completion.finish(info))
            .await
            .map_err(|error| wire(&error))
    }
    pub(super) fn archive_completion(&self) -> ArchiveCompletion {
        ArchiveCompletion {
            terminals: Arc::clone(&self.terminals),
            forges: Arc::clone(&self.forges),
        }
    }
    pub(crate) async fn archive_session(
        &self,
        _: &Connection,
        request: ArchiveSession,
    ) -> Result<ArchivedSession, WireError> {
        let completion = self.archive_completion();
        let archived = self
            .sessions
            .archive_condition_with(request.session, request.condition, move |info| {
                completion.finish(info)
            })
            .await
            .map_err(|error| wire(&error))?;
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

fn admit_child(
    principal: &crate::principal::Principal,
    here: vessel_proto::NodeId,
    request: ArchiveChildSession,
) -> Result<(), WireError> {
    principal.permit(vessel_proto::Method::ArchiveChildSession)?;
    // This person's bulk action cannot extend delegated agent authority across
    // the separate ancestry guard and mutation admissions.
    if principal.agent().is_some() {
        return Err(WireError::new(
            ErrorCode::Forbidden,
            "only a person may archive finished children",
        ));
    }
    if request.child.node != here {
        return Err(WireError::new(
            ErrorCode::InvalidRequest,
            "the child belongs to another node",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;

/// Service-owned handles stay outside Sessions while its task retains the work.
pub(super) struct ArchiveCompletion {
    terminals: Arc<Terminals>,
    forges: Arc<Forges>,
}
impl ArchiveCompletion {
    pub(super) async fn finish(self, archived: SessionInfo) -> Result<SessionInfo, ServiceError> {
        self.forges.tell(Command::Forget(archived.id));
        self.terminals.end_beside(archived.id).await;
        Ok(archived)
    }
}
