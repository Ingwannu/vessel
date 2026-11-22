//! Session-write acknowledgements for a live copy of the canonical durable clock.
//! Legacy convenience methods delegate here and keep their original results.

use super::Store;
use crate::{
    ReopenMode, Residence, RestartEligibility, SessionCommit, StoreError,
    writer::{SessionMark, Write},
};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use vessel_proto::restart::RestartGeneration;
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
        self.create_session_with_restart(info, RestartEligibility::Never)
            .await
    }

    /// A new session and its eligibility commit together before its actor drains.
    /// # Errors
    /// As with the ordinary create, or if the eligibility projection failed.
    pub async fn create_session_with_restart(
        &self,
        info: SessionInfo,
        eligibility: RestartEligibility,
    ) -> Result<SessionCommit<()>, StoreError> {
        self.write(|done| Write::Create {
            info: Box::new(info),
            eligibility,
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
        self.reopen_guarded(session, next, None, ReopenMode::Manual)
            .await
    }

    /// Reopen the precise observed run. Startup also requires an unarchived,
    /// interrupted row with an active boot claim.
    /// # Errors
    /// As with the ordinary reopen, or if the stop generation or claim changed.
    pub async fn reopen_exact(
        &self,
        session: SessionId,
        next: EventSeq,
        generation: RestartGeneration,
        mode: ReopenMode,
    ) -> Result<SessionCommit<KeptMarks>, StoreError> {
        self.reopen_guarded(session, next, Some(generation), mode)
            .await
    }

    async fn reopen_guarded(
        &self,
        session: SessionId,
        next: EventSeq,
        generation: Option<RestartGeneration>,
        mode: ReopenMode,
    ) -> Result<SessionCommit<KeptMarks>, StoreError> {
        self.write(|done| Write::Reopen {
            session,
            next,
            generation,
            mode,
            done,
        })
        .await
    }
}
