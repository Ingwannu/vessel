//! Reading a session's log a page at a time (`session.read`): a live
//! session's page as of the call, the ring filling what retention pruned,
//! and a read that waits for the next record (protocol 6).

use vessel_proto::{EventPage, EventRecord, EventSeq, SessionId, SessionInfo};
use vessel_session::{FollowError, SessionHandle};

use super::{Found, Sessions};
use crate::error::ServiceError;
use crate::persist;

impl Sessions {
    /// A page of the log. For a live session, first waits until everything
    /// the session had produced when the call arrived is committed, so the
    /// page (and `at_end`) reflects the log as of the call, not the store's
    /// lag; and records retention already pruned from the store but still
    /// in the session's ring come from the ring (see [`with_ring`]).
    pub(crate) async fn read(
        &self,
        id: SessionId,
        from: EventSeq,
        limit: u32,
    ) -> Result<EventPage, ServiceError> {
        match self.find(id).await? {
            Found::Live {
                handle,
                mut durability,
            } => {
                let produced = handle.info().next_seq;
                persist::wait_until(id, &mut durability, produced).await?;
                let page = self.store.read_events(id, from, limit).await?;
                Ok(with_ring(page, &handle, from, limit))
            }
            Found::Stored(_) => Ok(self.store.read_events(id, from, limit).await?),
        }
    }

    /// [`read`](Self::read), but with nothing at `from` yet in a live
    /// session, first wait up to `wait` for a record there or for the
    /// session to end (`session.read` with `wait`, protocol 6).
    ///
    /// [Decision Log]
    /// - Purpose: an agent waits for another node's session to finish a
    ///   turn without a subscription across hops (ADR 0013: a long poll,
    ///   as `fleet.watch`).
    /// - Choice: follow the session's ring from `from` and wait for its
    ///   first record there, or for the live stream to close (the session
    ///   ended); then read as usual, so the page is the same as without
    ///   waiting. Records already at `from`, an ended or stored session,
    ///   and a `from` the read refuses all answer at once.
    /// - Not the info watch: `next_seq` there moves with every record but
    ///   does not notify (that would wake every info watcher on every
    ///   event), so a wait on it slept through a turn's records until a
    ///   status changed or the wait ran out.
    /// - Bound: the caller caps `wait` ([`ReadEvents::MAX_WAIT`]), so a
    ///   waiting call holds one of the connection's call slots for at most
    ///   a minute.
    ///
    /// [`ReadEvents::MAX_WAIT`]: vessel_proto::ReadEvents::MAX_WAIT
    pub(crate) async fn read_waiting(
        &self,
        id: SessionId,
        from: EventSeq,
        limit: u32,
        wait: std::time::Duration,
    ) -> Result<EventPage, ServiceError> {
        if let Found::Live { handle, .. } = self.find(id).await?
            && let Ok(follow) = handle.follow(from)
            && follow.cached.is_empty()
        {
            // A record, a lag (records landed) or the end: read now.
            let mut live = follow.live;
            if tokio::time::timeout(wait, live.recv()).await.is_err() {
                tracing::trace!(session = %id, "a waiting read found nothing new");
            }
        }
        self.read(id, from, limit).await
    }
    pub(crate) async fn get(&self, id: SessionId) -> Result<SessionInfo, ServiceError> {
        Ok(match self.find(id).await? {
            Found::Live { handle, .. } => self.current_info(&handle).await?,
            Found::Stored(info) => info,
        })
    }

    /// Prefer metadata stored after a run ended while its handle awaits handover.
    pub(super) async fn current_info(
        &self,
        handle: &SessionHandle,
    ) -> Result<vessel_proto::SessionInfo, ServiceError> {
        let info = handle.info();
        if !info.status.is_live()
            && let Some(stored) = self.store.session(info.id).await?
            && stored.next_seq > info.next_seq
        {
            return Ok(stored);
        }
        Ok(info)
    }
}

/// `page`, read from the store for a live session, unless retention pruned
/// records at its start that the session's ring still holds: then the page
/// comes from the ring, starting at `from` or at the ring's oldest record.
///
/// [Decision Log]
/// - Problem: a follower that lags catches up with `session.read`. When the
///   retention budget is smaller than the ring (`max_session_bytes` under
///   about 16 MiB with the default ring), the store can have dropped
///   records the ring still has. A store-only page then jumped over them,
///   and the follower reported them missing although the node had them. A
///   test waiting for output in that range waited forever.
/// - Alternatives: have the client resubscribe instead of reading (the
///   subscription path already prefers the ring), but paging exists so a
///   slow client catches up at its own pace without lagging the broadcast
///   again; or forbid budgets under the ring size, which would make
///   retention depend on a runtime setting.
/// - Choice: the read prefers the ring whenever the store's page does not
///   start at `from` and the ring reaches further back. Retention prunes a
///   prefix and the ring holds the newest records, so either source alone
///   is a gap-free suffix; no merge is needed.
/// - Cost: one ring lookup per read of a live session whose store page has
///   a gap, which only happens after pruning.
pub(super) fn with_ring(
    page: EventPage,
    handle: &SessionHandle,
    from: EventSeq,
    limit: u32,
) -> EventPage {
    // An ended run's ring is sealed. Storage-only checkpoint metadata may now
    // follow it; the ring must not truncate the store's later result/history.
    if !handle.info().status.is_live() {
        return page;
    }
    let stored_from = page.records.first().map(|record| record.seq);
    if stored_from == Some(from) {
        return page;
    }
    let follow = match handle.follow(from) {
        Ok(follow) => follow,
        Err(FollowError::Gap { oldest_cached }) => {
            // The ring starts later than `from`: it helps only if it starts
            // earlier than the store's page.
            if stored_from.is_some_and(|stored| stored <= oldest_cached) {
                return page;
            }
            match handle.follow(oldest_cached) {
                Ok(follow) => follow,
                // The ring moved on meanwhile; the store's page stands.
                Err(FollowError::Gap { .. } | FollowError::Ahead { .. }) => return page,
            }
        }
        Err(FollowError::Ahead { .. }) => return page,
    };
    let cached = follow.cached;
    let limit = usize::try_from(limit).unwrap_or(usize::MAX);
    let records: Vec<EventRecord> = cached
        .iter()
        .take(limit)
        .map(|record| EventRecord::clone(record))
        .collect();
    let Some(last) = records.last() else {
        return page;
    };
    EventPage {
        // `None` only for a log that is full; nothing follows its last
        // record, so asking for it again is as good an answer as any.
        next: last.seq.next().unwrap_or(last.seq),
        at_end: cached.len() <= limit,
        records,
    }
}
