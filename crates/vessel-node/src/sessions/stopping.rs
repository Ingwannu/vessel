//! Durable person-stop barriers; raw failure cleanup must not suppress a later run.
use super::{Found, Sessions, persist};
use crate::error::ServiceError;
use std::sync::Arc;
use vessel_proto::SessionId;
use vessel_session::SessionHandle;
impl Sessions {
    /// Stop a session. Killing one that already ended is a no-op.
    pub(crate) async fn kill(self: &Arc<Self>, id: SessionId) -> Result<(), ServiceError> {
        let sessions = Arc::clone(self);
        self.tasks
            .spawn(async move {
                match sessions.find(id).await? {
                    Found::Live { handle, .. } => {
                        sessions
                            .stop(&handle, vessel_session::SessionStop::Kill)
                            .await?;
                    }
                    Found::Stored(info) => {
                        let _same = sessions
                            .store
                            .suppress_ended_restart(id, info.next_seq)
                            .await?;
                    }
                }
                Ok(())
            })
            .await
            .map_err(ServiceError::Task)?
    }

    /// Stop only the run admitted by the recovery arbiter, never a later lookup.
    /// An ended handle uses the existing final-log CAS, not a replacement's row.
    pub(crate) async fn kill_pinned(&self, handle: SessionHandle) -> Result<(), ServiceError> {
        self.stop(&handle, vessel_session::SessionStop::Kill).await
    }

    /// The same ordered barrier serves cancel, kill and archive. If the
    /// final status closed publication first, wait for its log and CAS the
    /// ended row, never a new manually resumed run.
    pub(super) async fn stop(
        &self,
        handle: &SessionHandle,
        action: vessel_session::SessionStop,
    ) -> Result<(), ServiceError> {
        let turn = match action {
            vessel_session::SessionStop::Kill => None,
            vessel_session::SessionStop::Cancel { turn } => turn,
        };
        match handle.stop(action).await {
            Ok(()) => Ok(()),
            Err(vessel_session::SessionError::Journal(
                vessel_session::JournalError::Unavailable,
            )) if handle.has_ended() => {
                // ended is published before the ring append. Await actor
                // completion before choosing the final durable CAS boundary.
                let final_info = handle.ended().await;
                if let Found::Live { mut durability, .. } = self.find(handle.id()).await? {
                    persist::wait_until(handle.id(), &mut durability, final_info.next_seq).await?;
                }
                if turn.is_none() {
                    let _same = self
                        .store
                        .suppress_ended_restart(handle.id(), final_info.next_seq)
                        .await?;
                }
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    }
}
