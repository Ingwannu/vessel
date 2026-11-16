//! Session-write acknowledgements for a live copy of the canonical durable clock.
//! Legacy convenience methods delegate here and keep their original results.

use super::Store;
use crate::{SessionCommit, StoreError, writer::Write};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use vessel_proto::{EventRecord, EventSeq, KeptMarks, Mark, SessionId, SessionInfo};

impl Store {
    /// Create the row and return its own committed writer time, independent of
    /// imported/creation times. The ordinary create operation has identical effects.
    /// # Errors
    /// Duplicate id, invalid metadata/time, or the existing write/commit failures.
    pub async fn create_session_committed(
        &self,
        info: SessionInfo,
    ) -> Result<SessionCommit<()>, StoreError> {
        self.write(|done| Write::Create {
            info: Box::new(info),
            done,
        })
        .await
    }

    /// Append exactly as append does, retaining THIS batch's acknowledged clock
    /// so a delayed live update cannot claim a later unrelated row snapshot.
    /// # Errors
    /// Unknown session, wrong cursor, invalid/exhausted time, or failed commit.
    pub async fn append_committed(
        &self,
        session: SessionId,
        records: Vec<Arc<EventRecord>>,
    ) -> Result<SessionCommit<()>, StoreError> {
        self.write(|done| Write::Append {
            session,
            records,
            done,
        })
        .await
    }

    /// Apply the ordinary mark and acknowledge its kept marks with the clock
    /// captured in the same savepoint, without a later asynchronous re-read.
    /// # Errors
    /// Unknown session, invalid/exhausted time, or the existing write failures.
    pub async fn mark_committed(
        &self,
        session: SessionId,
        mark: Mark,
        at: DateTime<Utc>,
    ) -> Result<SessionCommit<KeptMarks>, StoreError> {
        self.mark_in_residency(session, mark, at, crate::marks::Residency::Stored)
            .await
    }

    /// Preserve loaded Read/Unread activity time while committing their marks.
    /// Store does not verify residency: only the Node's existing loaded branch,
    /// under its marking lease, should make this ordering-only claim.
    /// # Errors
    /// Unknown session, invalid/exhausted time, or the existing write failures.
    pub async fn mark_loaded_committed(
        &self,
        session: SessionId,
        mark: Mark,
        at: DateTime<Utc>,
    ) -> Result<SessionCommit<KeptMarks>, StoreError> {
        self.mark_in_residency(session, mark, at, crate::marks::Residency::Loaded)
            .await
    }

    async fn mark_in_residency(
        &self,
        session: SessionId,
        mark: Mark,
        at: DateTime<Utc>,
        residency: crate::marks::Residency,
    ) -> Result<SessionCommit<KeptMarks>, StoreError> {
        self.write(|done| Write::Mark {
            session,
            mark,
            at,
            residency,
            done,
        })
        .await
    }

    /// Keep the existing conditional reopen and return its exact durable time
    /// with its marks before another queued mutation can change that pair.
    /// # Errors
    /// Unknown/live session, wrong cursor, invalid/exhausted time, or failed commit.
    pub async fn reopen_committed(
        &self,
        session: SessionId,
        next: EventSeq,
    ) -> Result<SessionCommit<KeptMarks>, StoreError> {
        self.write(|done| Write::Reopen {
            session,
            next,
            done,
        })
        .await
    }
}
