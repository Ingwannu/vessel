//! Persistence of live sessions: one task per session drains its durable
//! channel into the store and publishes how far the log is committed.
//!
//! The task takes every record already waiting (up to [`BATCH`]) and
//! appends them in one call, so a burst of output costs one commit, not
//! one per record. Records stay `Arc`s all the way into the store. A typed
//! checkpoint ends the batch and is acknowledged only after its preceding
//! records and projection change commit together; it consumes no `EventSeq`.
//!
//! If the store fails, the session cannot keep its gap-free log, so the
//! task kills it and marks its durability [`Durability::Broken`]; readers
//! waiting for those records get an error instead of a silent hole.
//!
//! The task reports how it ended ([`Persisted`]); `sessions` then hands the
//! session over to the store and removes it from the runtime.

#[cfg(test)]
pub(crate) mod reply_gate;

use std::sync::Arc;

use tokio::sync::oneshot;
use tokio::sync::{mpsc, watch};
use vessel_proto::{ErrorCode, WireError, restart::RestartCheckpoint};
use vessel_proto::{EventRecord, EventSeq, SessionId};
use vessel_session::{JournalEntry, JournalError, SessionHandle};
use vessel_store::Store;

/// Most records appended in one store call.
const BATCH: usize = 256;

/// How much of a live session's log is committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Durability {
    /// Every record before this sequence number is committed.
    Through(EventSeq),
    /// Persisting failed; the session was killed and later records are not
    /// in the store.
    Broken,
}

/// How persisting a session's log ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Persisted {
    /// Every record is committed, through the final status: the durable
    /// channel was drained to its end, which means the session ended.
    Complete,
    /// The store failed; the session was killed and may still be ending.
    Incomplete,
}

/// The log of `session` could not be persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the log of session {0} could not be persisted")]
pub(crate) struct Unpersisted(pub(crate) SessionId);

/// Drain `durable` into the store until the session ends.
pub(crate) async fn run(
    store: Store,
    handle: SessionHandle,
    mut durable: mpsc::Receiver<JournalEntry>,
    durability: watch::Sender<Durability>,
) -> Persisted {
    let session = handle.id();
    while let Some(batch) = Batch::receive(&mut durable).await {
        let next = batch.records.last().map(|record| record.seq.next());
        if next == Some(None) {
            tracing::error!(%session, "record sequence exhausted; stopping persistence");
            durability.send_modify(|state| *state = Durability::Broken);
            handle.kill();
            return Persisted::Incomplete;
        }
        let result = match &batch.barrier {
            Some((change, _reply)) => {
                store
                    .checkpoint_restart(session, batch.records, *change)
                    .await
            }
            None => store.append(session, batch.records).await.map(|()| true),
        };
        match result {
            Ok(changed) => {
                if let Some(Some(next)) = next {
                    durability.send_modify(|state| *state = Durability::Through(next));
                }
                if let Some((_change, reply)) = batch.barrier {
                    #[cfg(test)]
                    let reply = match reply_gate::hold(session, _change, reply, changed) {
                        Ok(()) => continue,
                        Err(reply) => reply,
                    };
                    drop(reply.send(Ok(changed)));
                }
            }
            Err(error) => {
                tracing::error!(%session, %error, "cannot persist session records or recovery barrier; killing the session");
                durability.send_modify(|state| *state = Durability::Broken);
                if let Some((_change, reply)) = batch.barrier {
                    drop(reply.send(Err(JournalError::Commit(WireError::new(
                        ErrorCode::Internal,
                        "the session records or recovery checkpoint could not be committed",
                    )))));
                }
                handle.kill();
                return Persisted::Incomplete;
            }
        }
    }
    tracing::debug!(%session, "session log fully persisted");
    Persisted::Complete
}

type Barrier = (
    RestartCheckpoint,
    oneshot::Sender<Result<bool, JournalError>>,
);
struct Batch {
    records: Vec<Arc<EventRecord>>,
    barrier: Option<Barrier>,
}
impl Batch {
    async fn receive(receiver: &mut mpsc::Receiver<JournalEntry>) -> Option<Self> {
        let first = receiver.recv().await?;
        let mut batch = Self {
            records: Vec::with_capacity(BATCH),
            barrier: None,
        };
        batch.take(first);
        // Drain only what is already ready and stop at the first barrier.
        // Public records after it belong to the next store operation.
        while batch.barrier.is_none() && batch.records.len() < BATCH {
            match receiver.try_recv() {
                Ok(item) => batch.take(item),
                Err(mpsc::error::TryRecvError::Empty | mpsc::error::TryRecvError::Disconnected) => {
                    break;
                }
            }
        }
        Some(batch)
    }
    fn take(&mut self, item: JournalEntry) {
        match item {
            JournalEntry::Event(record) => self.records.push(record),
            JournalEntry::Checkpoint { change, reply } => self.barrier = Some((change, reply)),
        }
    }
}

/// Wait until every record before `target` is committed.
pub(crate) async fn wait_until(
    session: SessionId,
    durability: &mut watch::Receiver<Durability>,
    target: EventSeq,
) -> Result<(), Unpersisted> {
    let reached = |durability: &Durability| match durability {
        Durability::Through(next) => *next >= target,
        Durability::Broken => true,
    };
    let waited = durability.wait_for(reached).await.map(|current| *current);
    // An error means the persist task is gone; its last word stands.
    let last = waited.unwrap_or_else(|_finished| *durability.borrow());
    match last {
        Durability::Through(next) if next >= target => Ok(()),
        Durability::Through(_) | Durability::Broken => Err(Unpersisted(session)),
    }
}
