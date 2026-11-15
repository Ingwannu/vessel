//! Archive condition and mark commit share resume/detach admission authority.
//! An actor-admitted stop cannot be cancelled between shutdown and durable marking.
use super::{Found, Sessions};
use crate::error::ServiceError;
use chrono::Utc;
use std::sync::Arc;
use vessel_proto::{Mark, SessionId, SessionInfo, session_archive::ArchiveCondition};
use vessel_session::{ArchiveRefusal, SessionError};

enum ExistingArchive {
    Keep,
    Refuse,
}

impl Sessions {
    pub(crate) async fn archive_condition(
        self: &Arc<Self>,
        session: SessionId,
        condition: ArchiveCondition,
    ) -> Result<SessionInfo, ServiceError> {
        self.archive_requested(session, condition, ExistingArchive::Refuse)
            .await
    }
    /// The legacy mark keeps its idempotent timestamp while sharing admission.
    pub(super) async fn archive_mark(
        self: &Arc<Self>,
        session: SessionId,
    ) -> Result<SessionInfo, ServiceError> {
        self.archive_requested(session, ArchiveCondition::Force, ExistingArchive::Keep)
            .await
    }
    async fn archive_requested(
        self: &Arc<Self>,
        session: SessionId,
        condition: ArchiveCondition,
        existing: ExistingArchive,
    ) -> Result<SessionInfo, ServiceError> {
        let sessions = Arc::clone(self);
        self.tasks
            .spawn(async move {
                // Resume and detach use this same admission mutex. Neither may
                // publish between the actor's stop and its durable archive mark.
                let _admission = sessions.marking.lock().await;
                sessions.archive_locked(session, condition, existing).await
            })
            .await
            .map_err(ServiceError::Task)?
    }
    /// Only archive_requested calls this, while holding this node's marking lock.
    async fn archive_locked(
        &self,
        session: SessionId,
        condition: ArchiveCondition,
        existing: ExistingArchive,
    ) -> Result<SessionInfo, ServiceError> {
        let found = self.find(session).await?;
        let already = match &found {
            Found::Live { handle, .. } => handle.info().marks.archived_at.is_some(),
            Found::Stored(info) => info.marks.archived_at.is_some(),
        };
        if already {
            return match existing {
                ExistingArchive::Keep => Ok(match found {
                    Found::Live { handle, .. } => handle.info(),
                    Found::Stored(info) => info,
                }),
                ExistingArchive::Refuse => Err(ServiceError::Session(SessionError::NotArchived(
                    ArchiveRefusal::AlreadyArchived,
                ))),
            };
        }
        match found {
            Found::Live { handle, .. } => {
                let _ended = handle.stop_for_archive(condition).await?;
                let kept = self.store.mark(session, Mark::Archive, Utc::now()).await?;
                handle.set_marks(kept);
                Ok(handle.info())
            }
            Found::Stored(_) => {
                let _kept = self.store.mark(session, Mark::Archive, Utc::now()).await?;
                self.store
                    .session(session)
                    .await?
                    .ok_or(ServiceError::NotFound(session))
            }
        }
    }
}
