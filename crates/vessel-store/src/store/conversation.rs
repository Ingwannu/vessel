//! The store's side of protocol 10's session calls (ADR 0026): a stored
//! session's conversation, its marks, deleting it, and reopening it to
//! start again.

use chrono::{DateTime, Utc};
use vessel_proto::{Conversation, EventSeq, KeptMarks, Mark, SessionId};

use super::Store;
use crate::SessionCommit;
use crate::again::EndedSession;
use crate::error::StoreError;
use crate::writer::Write;

impl Store {
    /// A stored session's conversation as its log left it (protocol 10's
    /// `session.conversation` for an ended session); `None` for an unknown
    /// session.
    ///
    /// # Errors
    /// The read failed, or a stored value is corrupt.
    pub async fn conversation(&self, id: SessionId) -> Result<Option<Conversation>, StoreError> {
        self.read(move |reader| reader.conversation(id)).await
    }

    /// Set a mark on a session (protocol 10's `session.mark`, ADR 0026),
    /// made at `at`; returns its marks after. `read` keeps the furthest
    /// `through` and clears `unread`; `archive` keeps the first time.
    /// Nothing checks that an archived session has ended: the node stops
    /// a session before archiving it. This is the path of a session served
    /// from the store ([`crate::Residence::Stored`]); a session the node's
    /// runtime has uses [`Self::mark_committed`] with `Loaded`.
    ///
    /// # Errors
    /// [`StoreError::UnknownSession`], or the write failed.
    pub async fn mark(
        &self,
        session: SessionId,
        mark: Mark,
        at: DateTime<Utc>,
    ) -> Result<KeptMarks, StoreError> {
        self.mark_committed(session, mark, at, crate::Residence::Stored)
            .await
            .map(SessionCommit::into_value)
    }

    /// Delete an ended session (protocol 10's `session.delete`): its log,
    /// its marks, its queue, its attachment rows, and its folder of
    /// attachment files, which is gone when this returns.
    ///
    /// # Errors
    /// [`StoreError::UnknownSession`], [`StoreError::SessionLive`] while
    /// the store still shows it live, or the write failed.
    pub async fn delete_session(&self, session: SessionId) -> Result<(), StoreError> {
        self.write(|done| Write::Delete { session, done }).await
    }

    /// A stored session with what a new run of it continues from (protocol
    /// 10's `session.resume`, ADR 0026): its info, choices, queue, last
    /// context report and attention point, read as one state of its row.
    /// `None` for an unknown session.
    ///
    /// # Errors
    /// The read failed, or a stored value is corrupt.
    pub async fn ended_session(&self, id: SessionId) -> Result<Option<EndedSession>, StoreError> {
        self.read(move |reader| reader.ended_session(id)).await
    }

    /// Reopen an ended session's row for a new run whose first record is
    /// `next` (the `next_seq` the caller read): live, no end, out of the
    /// archive, before the run publishes anything, so retention never
    /// prunes a session that is starting again and its log continues after
    /// its final status. Returns its marks after.
    ///
    /// # Errors
    /// [`StoreError::UnknownSession`] once deleted or pruned,
    /// [`StoreError::SessionLive`] while the row is still live,
    /// [`StoreError::OutOfOrder`] when its log no longer ends at `next`,
    /// or the write failed. A refusal changes nothing.
    pub async fn reopen(
        &self,
        session: SessionId,
        next: EventSeq,
    ) -> Result<KeptMarks, StoreError> {
        self.reopen_committed(session, next)
            .await
            .map(SessionCommit::into_value)
    }
}
