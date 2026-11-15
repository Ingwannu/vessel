//! `session.mark` and `session.delete` (protocol 10, ADR 0026) for live and
//! stored sessions alike: the store keeps the marks, and a session the
//! runtime still has is handed the store's result so its info agrees.
//!
//! [Decision Log]
//! - Purpose: a mark is the same for every client and survives the node;
//!   `session.list`, `session.get`, a subscription's info and the board
//!   must show it at once, for a session that runs as for one that ended.
//! - The keeper: the store. Every mark is applied there first (one row
//!   update), and its answer, the marks it left, is what a live session's
//!   handle takes (`SessionHandle::set_marks`), so the runtime's copy is
//!   never anything the store did not keep. A stored session's answer is
//!   read back from the store.
//! - Order: two marks on one live session could reach the store in one
//!   order and the handle in the other, leaving the handle showing what
//!   the store no longer keeps. Marks on live sessions are therefore
//!   serialized through one lock held from the store's write to the
//!   handle's update; marks are a person's clicks, so one lock for the node
//!   costs nothing, and it is a tokio lock because it is held across the
//!   store's write. That part runs as a task of the node's, so a cancelled
//!   call cannot stop it between the two.
//! - A stored session's mark takes the same lock and looks the session up
//!   again under it: a session started again becomes visible and has its
//!   row reopened under it (`again`), so a mark is either in the store
//!   before the reopening, whose answer carries it, or applied to the
//!   running session after.
//! - `archive` on a live session stops it first and answers once it has
//!   ended, as agent-orchestrator's and Paseo's archive do: an archived
//!   session has ended (`SessionMarks::archived_at`'s promise). The wait
//!   is bounded by the session's stop (a polite signal, then the kill).
//! - `read`'s `through` may not pass the end of the log as published
//!   (`invalid_request`): a client cannot mark read what does not exist
//!   yet, and since `read` only moves forward, a mistake there could never
//!   be taken back. The end is the ring's for a session the runtime has,
//!   which a follower's records never pass.
//! - `session.delete` takes only a session served from the store: one the
//!   runtime still has is live (`conflict`: archive it first), or ended with
//!   its log still being written (`conflict`: a retry works once the store
//!   has it all).

use std::sync::Arc;

use chrono::Utc;
use vessel_proto::{EventSeq, Mark, SessionId, SessionInfo};
use vessel_session::{RemoveError, SessionHandle};

use super::{Found, Sessions};
use crate::error::ServiceError;

impl Sessions {
    /// Set `mark` on a session, live or stored; its info after.
    pub(crate) async fn mark(
        self: &Arc<Self>,
        id: SessionId,
        mark: Mark,
    ) -> Result<SessionInfo, ServiceError> {
        if matches!(mark, Mark::Archive) {
            return self.archive_mark(id).await;
        }
        if let Found::Live { handle, .. } = self.find(id).await? {
            return self.mark_live(handle, mark).await;
        }
        // A session starting again becomes visible and has its row
        // reopened under this lock (`again`): looked up again under it, a
        // stored session stays stored until its mark is written, so the
        // reopening's answer carries the mark.
        let order = self.marking.lock().await;
        let info = match self.find(id).await? {
            Found::Live { handle, .. } => {
                drop(order);
                return self.mark_live(handle, mark).await;
            }
            Found::Stored(info) => info,
        };
        if let Mark::Read { through } = mark {
            within_log(id, through, info.next_seq)?;
        }
        let _kept = self.store.mark(id, mark, Utc::now()).await?;
        drop(order);
        self.store
            .session(id)
            .await?
            .ok_or(ServiceError::NotFound(id))
    }

    /// Non-archive marks for a session the runtime has. Archive holds admission
    /// through shutdown in archive.rs instead of stopping before taking the lock.
    ///
    /// The work runs as a task of its own: a caller that goes away (a
    /// cancelled call) must not leave the store marked and the handle not,
    /// or a session stopped for an archive that never came.
    async fn mark_live(
        self: &Arc<Self>,
        handle: SessionHandle,
        mark: Mark,
    ) -> Result<SessionInfo, ServiceError> {
        let id = handle.id();
        if let Mark::Read { through } = mark {
            within_log(id, through, handle.log_end())?;
        }
        let sessions = Arc::clone(self);
        self.tasks
            .spawn(async move {
                let _order = sessions.marking.lock().await;
                let kept = sessions.store.mark(id, mark, Utc::now()).await?;
                handle.set_marks(kept);
                Ok(handle.info())
            })
            .await
            .map_err(ServiceError::Task)?
    }

    /// Delete an ended session whole: its log, marks, queue and
    /// attachments, files included. A session that has just ended is
    /// waited for until the store serves it (`HAND_OVER_WAIT`).
    pub(crate) async fn delete(self: &Arc<Self>, id: SessionId) -> Result<(), ServiceError> {
        if let Found::Live { handle, .. } = self.find(id).await?
            && handle.has_ended()
        {
            // Persistence can finish independently; do not hold publication
            // admission while waiting for the old run to hand over.
            self.wait_stored(id).await?;
        }
        let sessions = Arc::clone(self);
        self.tasks
            .spawn(async move {
                let _admission = sessions.marking.lock().await;
                sessions.delete_locked(id).await
            })
            .await
            .map_err(ServiceError::Task)?
    }

    /// Resume publication uses this same admission. Re-find under it before
    /// deletion, and keep the admitted commit alive if the RPC waiter leaves.
    async fn delete_locked(&self, id: SessionId) -> Result<(), ServiceError> {
        let found = self.find(id).await?;
        match found {
            Found::Live { handle, .. } if handle.has_ended() => Err(ServiceError::LogWriting(id)),
            Found::Live { .. } => Err(ServiceError::Live(id)),
            Found::Stored(_) => {
                self.store.delete_session(id).await?;
                // A stored session is one the runtime already let go of
                // (`hand_over` removes it before the store serves it); if
                // it ever were not, the runtime must not keep serving a
                // session the store no longer has.
                match self.runtime.remove(id) {
                    Ok(_info) => tracing::debug!(session = %id, "deleted session left the runtime"),
                    Err(RemoveError::NotFound(_)) => {}
                    Err(RemoveError::Running(_)) => {
                        tracing::warn!(session = %id, "a deleted session is still running");
                    }
                }
                Ok(())
            }
        }
    }
}

/// A `read`'s `through` is at most `end`, the end of the log.
fn within_log(session: SessionId, through: EventSeq, end: EventSeq) -> Result<(), ServiceError> {
    if through > end {
        return Err(ServiceError::ReadAhead {
            session,
            through,
            end,
        });
    }
    Ok(())
}
