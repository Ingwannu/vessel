//! A session's worktree goes when the session goes (protocol 10, where
//! track A's deletion and retention meet track B's tidying, ADR 0027).
//!
//! [Decision Log]
//! - Purpose: a worktree made for a session (`WorktreeInfo.created_for`)
//!   is removed when that session is deleted or retention drops it, by the
//!   rule a person's `worktree.remove { if_lost: keep }` follows
//!   (`Workspaces::retire`): nothing is lost, and worktrees whose sessions
//!   are gone do not pile up.
//! - Not when a session merely ends or is archived: `session.resume`
//!   starts it again in the same folder (ADR 0026), so its folder must
//!   still be there.
//! - When: `session.delete` tidies within the call and answers what became
//!   of the worktree (`SessionDeleted`). A sweep tidies what no call
//!   answers for: sessions retention dropped (the store's writer prunes
//!   them and says how many, `Store::pruned`), a node stopped between a
//!   deletion and its tidying, and a worktree kept because another session
//!   was in it, whose own session is gone. The sweep runs over every
//!   worktree made for a session when the node starts and after each
//!   prune, and over the kept ones after each deletion (which may have let
//!   one go).
//! - One tidying at a time, under a tokio mutex held across the deletion
//!   and the removal (so across `.await`s): a delete and a sweep reaching
//!   the same worktree at once would inspect and remove it twice, and the
//!   delete would answer "no worktree" for one the sweep had just removed.
//!   It also bounds the git work tidying does to one removal. A person's
//!   `worktree.remove` does not take it: the worktree's own claim keeps
//!   those two apart, as the rule always did.
//! - Gone means neither the runtime nor the store has the session. A
//!   worktree names its session only after the session was created (the
//!   runtime has it from its start, the store from its first row), so a
//!   session being created is never taken for gone.
//! - The sweep tries only worktrees tidying never kept, or kept because a
//!   session was in it (which costs no git run and no teardown until that
//!   session has gone): one kept for the work it holds, or one git could
//!   not inspect (whose teardown would run again at every try), stays
//!   until a person decides (`worktree.remove` with `snapshot` or
//!   `discard`), as ADR 0027 says.
//! - Cost: a delete waits for the worktree's teardown, when it has an
//!   approved one (at most 5 minutes, ADR 0027), as `worktree.remove`
//!   does, and for a sweep already running.

use std::sync::Arc;

use tokio::sync::{Mutex, watch};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::SessionId;
use vessel_proto::conversation::{SessionDeleted, TidiedWorktree};

use crate::error::ServiceError;
use crate::sessions::Sessions;
use crate::workspace::{TidyScope, Workspaces};

/// Deleting sessions with their worktrees, and the sweep.
#[derive(Debug)]
pub(crate) struct Tidying {
    sessions: Arc<Sessions>,
    workspaces: Arc<Workspaces>,
    /// One tidying at a time (Decision Log).
    one: Mutex<()>,
}

impl Tidying {
    pub(crate) fn new(sessions: Arc<Sessions>, workspaces: Arc<Workspaces>) -> Arc<Self> {
        Arc::new(Self {
            sessions,
            workspaces,
            one: Mutex::new(()),
        })
    }

    /// `session.delete`: the ended session `id` deleted whole, then the
    /// worktree made for it tidied.
    ///
    /// # Errors
    /// As [`Sessions::delete`]; nothing is tidied then.
    pub(crate) async fn delete(&self, id: SessionId) -> Result<SessionDeleted, ServiceError> {
        let _one = self.one.lock().await;
        self.sessions.delete(id).await?;
        let worktree = self.retire(id).await;
        self.sweep_locked(TidyScope::Kept).await;
        Ok(SessionDeleted { worktree })
    }

    /// Tidy every worktree whose session is gone; what is kept says why
    /// in its `kept`.
    pub(crate) async fn sweep(&self) {
        let _one = self.one.lock().await;
        self.sweep_locked(TidyScope::Every).await;
    }

    /// [`sweep`](Self::sweep) over `scope`, the lock held by the caller.
    async fn sweep_locked(&self, scope: TidyScope) {
        for session in self.workspaces.tidy_candidates(scope) {
            if !self.sessions.gone(session).await {
                continue;
            }
            if let Some(tidied) = self.retire(session).await {
                tracing::info!(
                    %session,
                    worktree = %tidied.worktree,
                    outcome = ?tidied.outcome,
                    "tidied the worktree of a session that is gone"
                );
            }
        }
    }

    /// Sweep now, then after each prune the store reports, until `stop`.
    pub(crate) fn watch(
        self: Arc<Self>,
        mut pruned: watch::Receiver<u64>,
        tasks: &TaskTracker,
        stop: CancellationToken,
    ) {
        drop(tasks.spawn(async move {
            loop {
                self.sweep().await;
                tokio::select! {
                    () = stop.cancelled() => break,
                    changed = pruned.changed() => {
                        if changed.is_err() {
                            // The store closed: nothing prunes any more.
                            break;
                        }
                    }
                }
            }
        }));
    }

    /// The worktree made for `session`, removed by the rule, with the
    /// live sessions' folders as they are now.
    async fn retire(&self, session: SessionId) -> Option<TidiedWorktree> {
        let live = self.sessions.live_folders();
        self.workspaces.retire(session, &live).await
    }
}
