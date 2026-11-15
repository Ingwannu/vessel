//! Bounded ephemeral turn receipts, held by opener identity and exact session.
//! No polling can infer which concurrent/queued prompt started a turn.

use crate::transfers::Holder;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};
use tokio::sync::{Semaphore, oneshot, watch};
use tokio::time::Instant;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_proto::mcp::{PromptReceiptId, PromptReceiptState};
use vessel_proto::{ErrorCode, EventSeq, SessionId, WireError};

const MAX_RECEIPTS: usize = 256;
const MAX_PER_HOLDER: usize = 32;
const MAX_READERS: usize = 32;
pub(crate) const RECEIPT_LIFETIME: Duration = Duration::from_secs(30 * 60);
const READ_WAIT: Duration = Duration::from_secs(30);

#[derive(Debug)]
struct Entry {
    holder: Holder,
    session: SessionId,
    expires: Instant,
    state: watch::Receiver<PromptReceiptState>,
}
#[derive(Debug)]
pub(crate) struct PromptReceipts {
    entries: Mutex<HashMap<PromptReceiptId, Entry>>,
    readers: Semaphore,
}
impl Default for PromptReceipts {
    fn default() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            readers: Semaphore::new(MAX_READERS),
        }
    }
}
impl PromptReceipts {
    fn entries(&self) -> std::sync::MutexGuard<'_, HashMap<PromptReceiptId, Entry>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
    pub(crate) fn reserve(
        self: &Arc<Self>,
        holder: Holder,
        session: SessionId,
    ) -> Result<Reservation, WireError> {
        let now = Instant::now();
        let mut entries = self.entries();
        entries.retain(|_id, entry| entry.expires > now);
        if entries.len() >= MAX_RECEIPTS
            || entries
                .values()
                .filter(|entry| entry.holder == holder)
                .count()
                >= MAX_PER_HOLDER
        {
            return Err(WireError::new(
                ErrorCode::Conflict,
                "too many exact-turn receipts remain open",
            ));
        }
        let id = PromptReceiptId::random();
        let (sender, state) = watch::channel(PromptReceiptState::Pending);
        let _old = entries.insert(
            id,
            Entry {
                holder,
                session,
                expires: now + RECEIPT_LIFETIME,
                state,
            },
        );
        Ok(Reservation {
            id,
            sender,
            receipts: Arc::clone(self),
            retained: false,
        })
    }
    fn owned(
        &self,
        holder: Holder,
        session: SessionId,
        id: PromptReceiptId,
    ) -> Result<watch::Receiver<PromptReceiptState>, WireError> {
        self.entries()
            .get(&id)
            .filter(|entry| {
                entry.holder == holder && entry.session == session && entry.expires > Instant::now()
            })
            .map(|entry| entry.state.clone())
            .ok_or_else(missing)
    }
    pub(crate) async fn read(
        &self,
        holder: Holder,
        session: SessionId,
        id: PromptReceiptId,
    ) -> Result<PromptReceiptState, WireError> {
        let _permit = self.readers.try_acquire().map_err(|_full| {
            WireError::new(ErrorCode::Conflict, "too many receipt reads already wait")
        })?;
        let mut state = self.owned(holder, session, id)?;
        let current = *state.borrow_and_update();
        if current != PromptReceiptState::Pending {
            return Ok(current);
        }
        let _changed = tokio::time::timeout(READ_WAIT, state.changed()).await;
        // Expiry and opener are rechecked after waiting; close/expiry is not authority.
        let _current = self.owned(holder, session, id)?;
        let current = *state.borrow();
        Ok(current)
    }
    pub(crate) fn close(
        &self,
        holder: Holder,
        session: SessionId,
        id: PromptReceiptId,
    ) -> Result<(), WireError> {
        let _state = self.owned(holder, session, id)?;
        let _gone = self.entries().remove(&id);
        Ok(())
    }
}
fn missing() -> WireError {
    WireError::new(
        ErrorCode::NotFound,
        "no exact-turn receipt for this caller and session",
    )
}

#[derive(Debug)]
pub(crate) struct Reservation {
    pub(crate) id: PromptReceiptId,
    sender: watch::Sender<PromptReceiptState>,
    receipts: Arc<PromptReceipts>,
    retained: bool,
}
impl Reservation {
    pub(crate) fn follow(
        mut self,
        turn: oneshot::Receiver<EventSeq>,
        tasks: &TaskTracker,
        stop: CancellationToken,
    ) {
        self.retained = true;
        drop(tasks.spawn(async move {
            let outcome = tokio::select! {
                () = stop.cancelled() => None,
                () = self.sender.closed() => None,
                result = tokio::time::timeout(RECEIPT_LIFETIME, turn) => result.ok().and_then(Result::ok),
            };
            let state = outcome.map_or(PromptReceiptState::Removed, |turn| PromptReceiptState::Started { turn });
            let _old = self.sender.send_replace(state);
        }));
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        if !self.retained {
            let _gone = self.receipts.entries().remove(&self.id);
        }
    }
}

#[cfg(test)]
mod tests;
