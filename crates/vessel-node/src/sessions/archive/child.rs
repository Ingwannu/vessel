//! Finished-child mutation is admitted against the committed current parent.
//! Detach and resume cannot publish between this check and archive completion.
use super::{ExistingArchive, Sessions};
use crate::error::ServiceError;
use std::{future::Future, sync::Arc};
use vessel_proto::{
    SessionInfo,
    session_archive::{
        ArchiveChildSession, ArchiveCondition, ChildArchiveOutcome, ChildArchiveSkip,
        finished_child_candidate,
    },
};
use vessel_session::{ArchiveRefusal, SessionError};

impl Sessions {
    /// The service validates child.node before entering this node-local boundary.
    /// Completion runs under marking and must not reacquire it.
    pub(crate) async fn archive_child_with<C, F>(
        self: &Arc<Self>,
        request: ArchiveChildSession,
        completed: C,
    ) -> Result<ChildArchiveOutcome, ServiceError>
    where
        C: FnOnce(SessionInfo) -> F + Send + 'static,
        F: Future<Output = Result<SessionInfo, ServiceError>> + Send + 'static,
    {
        let sessions = Arc::clone(self);
        self.tasks
            .spawn(async move {
                // The existing metadata lock orders the full parent check, actor
                // IfIdle stop, durable mark and cancellation-safe cleanup together.
                let _admission = sessions.marking.lock().await;
                let skipped = |reason| {
                    Ok(ChildArchiveOutcome::Skipped {
                        child: request.child,
                        reason,
                    })
                };
                if let Some(reason) = sessions.ineligible_child(request).await? {
                    return skipped(reason);
                }
                let archived = match sessions
                    .archive_locked(
                        request.child.session,
                        ArchiveCondition::IfIdle,
                        ExistingArchive::Refuse,
                    )
                    .await
                {
                    Ok(archived) => completed(archived).await?,
                    Err(ServiceError::Session(SessionError::NotArchived(
                        ArchiveRefusal::TurnRunning | ArchiveRefusal::NotReady,
                    ))) => return skipped(ChildArchiveSkip::NotFinished),
                    Err(error) => return Err(error),
                };
                let archived_at = archived
                    .marks
                    .archived_at
                    .ok_or(ServiceError::ArchiveUnmarked(request.child.session))?;
                Ok(ChildArchiveOutcome::Archived {
                    child: request.child,
                    archived_at,
                })
            })
            .await
            .map_err(ServiceError::Task)?
    }

    /// Only the admitted caller above may use the current-parent evidence.
    async fn ineligible_child(
        &self,
        request: ArchiveChildSession,
    ) -> Result<Option<ChildArchiveSkip>, ServiceError> {
        let info = self.get(request.child.session).await?;
        let reason = if self.current_parent(&info).await? != Some(request.parent) {
            Some(ChildArchiveSkip::ParentChanged)
        } else if info.marks.archived_at.is_some() {
            Some(ChildArchiveSkip::AlreadyArchived)
        } else if !finished_child_candidate(&info) {
            Some(ChildArchiveSkip::NotFinished)
        } else {
            None
        };
        Ok(reason)
    }
}
