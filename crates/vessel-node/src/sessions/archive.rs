//! Archive condition and mark commit share resume/detach admission authority.
//! An actor-admitted stop cannot be cancelled between shutdown and durable marking.
use super::{Found, Sessions};
use crate::error::ServiceError;
use chrono::Utc;
use std::sync::Arc;
use vessel_proto::{Mark, SessionId, SessionInfo, session_archive::ArchiveCondition};
use vessel_session::{ArchiveRefusal, SessionError};

impl Sessions {
    pub(crate) async fn archive_condition(
        self: &Arc<Self>,
        session: SessionId,
        condition: ArchiveCondition,
    ) -> Result<SessionInfo, ServiceError> {
        let sessions = Arc::clone(self);
        self.tasks
            .spawn(async move {
                // This is the same mutex used by resume and detach. It stays held
                // across actor admission and stop, so neither can resurrect/reparent
                // the session before its archive mark is durable.
                let _admission = sessions.marking.lock().await;
                let found = sessions.find(session).await?;
                let current = match &found {
                    Found::Live { handle, .. } => handle.info(),
                    Found::Stored(info) => info.clone(),
                };
                if current.marks.archived_at.is_some() {
                    return Err(ServiceError::Session(SessionError::NotArchived(
                        ArchiveRefusal::AlreadyArchived,
                    )));
                }
                match found {
                    Found::Live { handle, .. } => {
                        let _ended = handle.stop_for_archive(condition).await?;
                        let kept = sessions
                            .store
                            .mark(session, Mark::Archive, Utc::now())
                            .await?;
                        handle.set_marks(kept);
                        Ok(handle.info())
                    }
                    Found::Stored(_) => {
                        let _kept = sessions
                            .store
                            .mark(session, Mark::Archive, Utc::now())
                            .await?;
                        sessions
                            .store
                            .session(session)
                            .await?
                            .ok_or(ServiceError::NotFound(session))
                    }
                }
            })
            .await
            .map_err(ServiceError::Task)?
    }
}
