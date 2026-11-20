//! How terminals end with what they are beside (protocol 11, ADR 0031): a
//! session's deletion or its removal by retention ends and forgets them; its
//! archiving ends and keeps them (the join of wave 2's first half); its end
//! ends the agents' terminals tied to it, watched from the moment one is
//! registered.

use std::sync::Arc;

use vessel_proto::{Creator, NodeId, SessionId};
use vessel_session::SessionHandle;
use vessel_session::terminals::TerminalHandle;

use super::{Terminals, lock_set};

impl Terminals {
    /// End and forget every terminal beside `session` (its deletion, or
    /// retention dropping it), and wait for their ends.
    pub(crate) async fn end_session(&self, session: SessionId) {
        let handles = self.handles(|info| info.place.session() == Some(session));
        self.end_all(&handles, true).await;
    }

    /// End every live terminal beside `session` and keep them, with their
    /// output (its archiving: a person put the session away, so a service
    /// or a shell beside it would run on unseen, holding a port and the
    /// worktree). Waits for their ends.
    pub(crate) async fn end_beside(&self, session: SessionId) {
        let handles = self.handles(|info| info.is_live() && info.place.session() == Some(session));
        self.end_all(&handles, false).await;
    }

    /// End every live terminal whose folder is in `folder` and keep them,
    /// with their output (a worktree removed with its terminals, ADR 0121).
    /// Waits for their ends.
    pub(crate) async fn end_in(&self, folder: &std::path::Path) {
        let handles = self.handles(|info| info.is_live() && info.folder.starts_with(folder));
        self.end_all(&handles, false).await;
    }

    /// Watch `session`'s end (once, however many terminals): it ends the
    /// terminals its own agent opened (`here` is this node) and every
    /// agent's terminal beside it. Synchronous, so a call that stops
    /// waiting after registering a terminal leaves it watched.
    pub(crate) fn watch_for_agents(self: &Arc<Self>, here: NodeId, session: SessionHandle) {
        let id = session.id();
        if !lock_set(&self.watching).insert(id) {
            return;
        }
        let terminals = Arc::clone(self);
        drop(self.runner.tasks.spawn(async move {
            let _info = session.ended().await;
            terminals.end_agents_of(here, id).await;
            let _was = lock_set(&terminals.watching).remove(&id);
        }));
    }

    /// End (and keep) the agents' terminals `session`'s end takes with it:
    /// those its own agent opened (its credential ended with it) and any
    /// agent's beside it.
    async fn end_agents_of(&self, here: NodeId, session: SessionId) {
        let handles = self.handles(|info| match info.created_by {
            Creator::Agent {
                node,
                session: agent,
            } => (node == here && agent == session) || info.place.session() == Some(session),
            Creator::Owner | Creator::Device { .. } => false,
        });
        self.end_all(&handles, false).await;
    }

    async fn end_all(&self, handles: &[TerminalHandle], forget: bool) {
        for handle in handles {
            handle.kill();
        }
        for handle in handles {
            if tokio::time::timeout(crate::terminals::CLOSE_WAIT, handle.ended())
                .await
                .is_err()
            {
                tracing::warn!(terminal = %handle.id(), "a terminal did not end in time");
            }
            if forget {
                self.forget(handle.id());
            }
        }
    }
}
