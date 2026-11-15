//! `session.resume` on the node's side (protocol 10, ADR 0026): which
//! ended session may start again, its row reopened before it publishes,
//! the answer once its agent is ready, and how a subscription the store
//! served ends when the session starts again meanwhile.
//!
//! [Decision Log]
//! - Purpose: the same session continues (its id, log, title, choices,
//!   marks and held queue) under a new run of its agent; readers find it
//!   in one place throughout, and retention never prunes it mid-start.
//! - Who may: a session the store serves; one the runtime still has is
//!   live (`conflict`). One that has just ended, its log on its way to the
//!   store, is waited for, at most `HAND_OVER_WAIT`, as `session.delete`
//!   waits (2026-11-15: an agent archiving then resuming met `conflict` in
//!   that gap); past it, `conflict` as before (a retry works). Structured,
//!   its agent able to resume and
//!   a conversation to resume (`unsupported` otherwise), read from its
//!   stored row before any process starts. Its folder still there
//!   (`conflict` otherwise, `FolderGone`): a worktree goes when the
//!   session it was made for is deleted, and a person may remove one, or
//!   a folder, while a session that ran in it has ended. One start again of a session at
//!   a time ([`Claim`]), so two calls never reserve its agent's place
//!   twice nor start it twice.
//! - Order: the runtime starts it gated (`vessel_session::Gate`, nothing
//!   published); under the marks' lock it becomes visible (its durability
//!   entry: the store holds its log up to its `next_seq`), the store
//!   reopens its row and the handle takes the store's marks; then the gate
//!   opens and persistence starts. A row the store will not reopen
//!   (deleted or pruned meanwhile) drops the gate: the session ends
//!   without a record, and its hand-over frees its places.
//! - The marks' lock makes a mark land either in the store before the
//!   reopening, whose answer then carries it, or on the running session
//!   after it; never in the store for a row its handle does not know of
//!   (`marks`, which looks again under the lock).
//! - The call answers once the agent is ready, its status leaving
//!   `starting`: bounded by the agent's handshake timeout, past which the
//!   session stops and ends `failed`. `running`, or `spawn_failed` with
//!   the reason its log ends with; the session then stays ended.
//! - A subscription the store served (review item 2): once it has sent
//!   everything, it ends `lagged` if the runtime has the session again
//!   (the client resubscribes from there and follows the new run),
//!   reads on if the stored log grew (a run started and ended meanwhile),
//!   and ends `session_ended` otherwise. One lookup per end, so it neither
//!   misses the new run nor loops: a row a failed store left live does
//!   not grow, and ends as before.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use vessel_proto::{EventSeq, SessionId, SessionInfo, SessionStatus};
use vessel_session::{Created, Ended, Relaunch, Reopening, SessionError, SessionHandle};
use vessel_store::EndedSession;

use super::side_questions::HAND_OVER_WAIT;
use super::{Found, Sessions};
use crate::error::ServiceError;

/// The sessions being started again, each claimed by one call.
#[derive(Debug, Default)]
pub(crate) struct Claims(Arc<Mutex<HashSet<SessionId>>>);

impl Claims {
    fn lock(&self) -> MutexGuard<'_, HashSet<SessionId>> {
        // Each change is one insert or remove: a poisoned set is whole.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// One call's claim on starting a session again; let go when dropped.
#[derive(Debug)]
pub(crate) struct Claim {
    claims: Arc<Mutex<HashSet<SessionId>>>,
    id: SessionId,
}

impl Drop for Claim {
    fn drop(&mut self) {
        let _was_claimed = self
            .claims
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.id);
    }
}

/// How a subscription the store served ends, once it has sent every
/// record before its cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StoredEnd {
    /// Still ended: `session_ended`.
    Ended,
    /// Started again meanwhile, and the runtime has it: `lagged`, and the
    /// client's resubscription follows the new run.
    Reopened,
    /// Its stored log grew meanwhile (a run started and ended): read on.
    Grew,
}

impl Sessions {
    /// Claim `id` for a start again.
    ///
    /// # Errors
    /// Another call is starting it again.
    pub(crate) fn claim(&self, id: SessionId) -> Result<Claim, ServiceError> {
        if !self.claims.lock().insert(id) {
            return Err(ServiceError::StartingAgain(id));
        }
        Ok(Claim {
            claims: Arc::clone(&self.claims.0),
            id,
        })
    }

    /// The ended session `id` as the store keeps it, if it may start again.
    ///
    /// # Errors
    /// Unknown; live, or its log still being written (`conflict`); a
    /// terminal session, an agent that cannot resume, or no conversation
    /// to resume (`unsupported`).
    pub(crate) async fn ended(&self, id: SessionId) -> Result<EndedSession, ServiceError> {
        self.handed_over(id).await?;
        match self.find(id).await? {
            Found::Live { handle, .. } if handle.has_ended() => {
                return Err(ServiceError::LogWriting(id));
            }
            Found::Live { .. } => return Err(ServiceError::AlreadyLive(id)),
            // Handed over, but not all the way yet: its last run's places
            // may still be taken.
            Found::Stored(_) if self.visible(id) => return Err(ServiceError::LogWriting(id)),
            Found::Stored(_) => {}
        }
        let ended = self
            .store
            .ended_session(id)
            .await?
            .ok_or(ServiceError::NotFound(id))?;
        let _conversation = vessel_session::resumable(&ended.info).map_err(SessionError::from)?;
        // Its folder first: an agent started in a folder that is gone fails
        // at its spawn with the operating system's words, or starts and
        // loads a conversation about files it cannot find.
        if let Some(folder) = &ended.info.spec.cwd
            && !is_folder(folder).await
        {
            return Err(ServiceError::FolderGone {
                session: id,
                folder: folder.clone(),
            });
        }
        Ok(ended)
    }

    /// Start `ended` again with `launch`, holding `claim` until it is
    /// visible; its handle, publishing. Runs as its own task, as a
    /// creation does: once the agent is started, reopening its row and
    /// starting its persistence must finish even if the call is cancelled.
    pub(crate) async fn start_again(
        self: &Arc<Self>,
        ended: EndedSession,
        launch: Relaunch,
        claim: Claim,
    ) -> Result<SessionHandle, ServiceError> {
        let sessions = Arc::clone(self);
        self.tasks
            .spawn(async move {
                let started = sessions.reopen(ended, launch).await;
                drop(claim);
                started
            })
            .await
            .map_err(ServiceError::Task)?
    }

    async fn reopen(
        self: &Arc<Self>,
        ended: EndedSession,
        launch: Relaunch,
    ) -> Result<SessionHandle, ServiceError> {
        let id = ended.info.id;
        let next = ended.info.next_seq;
        let generation = self.store.restart_generation(id).await?;
        let reopening = self.runtime.start_again(continued(ended), launch).await?;
        self.adopt_again(
            reopening,
            next,
            generation,
            vessel_store::ReopenMode::Manual,
        )
        .await
    }

    pub(crate) async fn prepare_again(
        &self,
        ended: EndedSession,
        launch: Relaunch,
    ) -> Result<vessel_session::PreparedAgain<'_>, ServiceError> {
        Ok(self.runtime.prepare_again(continued(ended), launch).await?)
    }

    /// After synchronous spawn admission this must settle, including when
    /// the attempt was cancelled. Its known run is never abandoned mid-write.
    pub(crate) async fn adopt_again(
        self: &Arc<Self>,
        reopening: Reopening,
        next: EventSeq,
        generation: vessel_proto::restart::RestartGeneration,
        mode: vessel_store::ReopenMode,
    ) -> Result<SessionHandle, ServiceError> {
        let Reopening {
            created: Created { handle, durable },
            gate,
        } = reopening;
        let id = handle.id();
        handle
            .initialise_restart_generation(generation)
            .map_err(SessionError::from)?;
        let (durability, reopened) = {
            let _order = self.marking.lock().await;
            let durability = self.track(&handle, next);
            let reopened = match self.store.reopen_exact(id, next, generation, mode).await {
                Ok(kept) => self.restore_metadata(&handle, kept).await,
                Err(error) => Err(error),
            };
            (durability, reopened)
        };
        if let Err(error) = reopened {
            tracing::info!(session = %id, %error, "its row was not reopened; not starting it again");
            // Nothing is published: its persistence finds no record, and
            // hands it over at once.
            drop(gate);
            self.persist(handle, durable, durability);
            return Err(error.into());
        }
        gate.open();
        self.persist(handle.clone(), durable, durability);
        Ok(handle)
    }

    /// Whether `id` has a durability entry: the runtime has it, or is
    /// handing it over to the store.
    /// Wait, at most [`HAND_OVER_WAIT`], while `id` has just ended and its
    /// hand-over is under way: the runtime still has it, or the store
    /// serves it while its last run's entry is still held. `hand_over`
    /// publishes the end after both are gone, so each end rechecks. A live
    /// session or one fully handed over returns at once; past the wait the
    /// caller's own checks answer `conflict`, as before.
    async fn handed_over(&self, id: SessionId) -> Result<(), ServiceError> {
        let mut ends = self.ends();
        let settled = async {
            loop {
                match self.find(id).await? {
                    Found::Live { handle, .. } if !handle.has_ended() => return Ok(()),
                    Found::Stored(_) if !self.visible(id) => return Ok(()),
                    Found::Live { .. } | Found::Stored(_) => {}
                }
                match ends.recv().await {
                    Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        return Err(ServiceError::Draining);
                    }
                }
            }
        };
        tokio::time::timeout(HAND_OVER_WAIT, settled)
            .await
            .unwrap_or(Ok(()))
    }

    fn visible(&self, id: SessionId) -> bool {
        self.durability
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(&id)
    }

    /// How a subscription the store served ends, having sent every
    /// record before `cursor`.
    pub(crate) async fn stored_end(&self, id: SessionId, cursor: EventSeq) -> StoredEnd {
        match self.find(id).await {
            Ok(Found::Live { .. }) => StoredEnd::Reopened,
            Ok(Found::Stored(info)) if info.next_seq > cursor => StoredEnd::Grew,
            Ok(Found::Stored(_)) => StoredEnd::Ended,
            Err(error) => {
                tracing::debug!(session = %id, %error, "a stored session is gone; its subscription ends");
                StoredEnd::Ended
            }
        }
    }
}

/// Once `handle`'s agent is ready, its info; why not otherwise.
///
/// # Errors
/// [`ServiceError::DidNotStart`]: the session ended first (its handshake
/// or `session/load` failed, or it was stopped).
pub(crate) async fn ready(handle: &SessionHandle) -> Result<SessionInfo, ServiceError> {
    let mut watching = handle.watch_info();
    // `starting` until the agent is ready or the run ends; the agent's
    // handshake timeout bounds it. The watch closes once the session ended.
    let status = match watching
        .wait_for(|info| info.status != SessionStatus::Starting)
        .await
    {
        Ok(info) => info.status.clone(),
        Err(_ended) => handle.info().status,
    };
    let why = match status {
        SessionStatus::Running => return Ok(handle.info()),
        SessionStatus::Failed { message, .. } => message,
        SessionStatus::Starting | SessionStatus::Exited { .. } | SessionStatus::Interrupted => {
            "it was stopped before its agent was ready".to_owned()
        }
    };
    Err(ServiceError::DidNotStart {
        session: handle.id(),
        why,
    })
}

/// Whether `path` is a folder now (a link to one counts, as for a spawn).
async fn is_folder(path: &std::path::Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .is_ok_and(|metadata| metadata.is_dir())
}

/// What the runtime continues from: the store's record of the session,
/// its choices the spec's when it never reported any.
fn continued(ended: EndedSession) -> Ended {
    let EndedSession {
        info,
        choices,
        queue,
        usage,
        attention,
        moved_through,
    } = ended;
    let choices = choices.unwrap_or_else(|| info.spec.mode.options().to_vec());
    Ended {
        info,
        choices,
        queue,
        usage,
        attention,
        moved_through,
    }
}
