//! Session-write acknowledgements for a live copy of the canonical durable clock.
//! Legacy convenience methods delegate here and keep their original results.

use super::Store;
use crate::{
    Residence, SessionCommit, StoreError,
    writer::{SessionMark, Write},
};
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
    /// `residence` decides whether a Read or Unread advances that clock.
    /// # Errors
    /// Unknown session, invalid/exhausted time, or the existing write failures.
    pub async fn mark_committed(
        &self,
        session: SessionId,
        mark: Mark,
        at: DateTime<Utc>,
        residence: Residence,
    ) -> Result<SessionCommit<KeptMarks>, StoreError> {
        self.commit_mark(session, mark, at, residence, false).await
    }

    /// A person's Archive also releases an exact scheduled Keep receipt,
    /// in the same transaction as the mark.
    /// # Errors
    /// Unknown session or failure to commit the mark and release.
    pub async fn archive_kept_session(
        &self,
        session: SessionId,
        at: DateTime<Utc>,
        residence: Residence,
    ) -> Result<SessionCommit<KeptMarks>, StoreError> {
        self.commit_mark(session, Mark::Archive, at, residence, true)
            .await
    }

    async fn commit_mark(
        &self,
        session: SessionId,
        mark: Mark,
        at: DateTime<Utc>,
        residence: Residence,
        release_keep: bool,
    ) -> Result<SessionCommit<KeptMarks>, StoreError> {
        self.write(|done| Write::Mark {
            write: SessionMark {
                session,
                mark,
                release_keep,
                at,
                residence,
            },
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
