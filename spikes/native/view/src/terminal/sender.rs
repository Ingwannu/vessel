//! Ordered terminal input. The RPC transport needs one identified call in flight.
//!
//! Paseo's current stream controller/client send terminal input in transport order;
//! Vessel serializes RPC calls so concurrent handlers cannot reorder it. The queue
//! chunks against the protocol's own byte limit, never retries an uncertain write,
//! and settles only the opaque receipt of the exact dispatch (T2 plan).
mod pending;
use pending::{Characters, Pending, Remainder};
use std::{collections::VecDeque, sync::Arc};

/// Most Unicode scalar values waiting while one call is in flight.
pub const MAX_PENDING: usize = 1024 * 1024;

/// One bounded ordered call; routing and authenticated lease admission stay outside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outgoing {
    /// UTF8 keystrokes/paste, at most the existing node call's byte ceiling.
    Keys(String),
    /// An admitted view size, ordered among the keys.
    Resize {
        /// Columns.
        cols: u16,
        /// Rows.
        rows: u16,
    },
}

/// Admission leaves the caller's borrowed input intact on every refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taken {
    /// The complete input was queued, or empty input required no operation.
    Queued,
    /// First quota refusal in one overflow episode; show the notice once.
    DroppedFirst,
    /// Another explicit quota refusal in the same episode.
    Dropped,
    /// This owner was closed; nothing was acknowledged or retained.
    Closed,
    /// Allocation refused before changing the queue.
    Unavailable,
}

/// Retaining a receipt pins its allocation identity, preventing A→B→A reuse.
#[derive(Debug, Clone)]
pub struct SendReceipt(Arc<()>);
impl PartialEq for SendReceipt {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for SendReceipt {}

/// Only this exact in-flight operation may release the next ordered call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendSettlement {
    /// The matching call settled; failure still never retries its bytes.
    Settled,
    /// Late, duplicate, retired or other-owner receipt; no state changed.
    Stale,
}
/// The caller retains the receipt with its actual asynchronous operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendDispatch {
    receipt: SendReceipt,
    outgoing: Outgoing,
}
impl SendDispatch {
    /// Move the payload once and keep the receipt for the corresponding completion.
    #[must_use]
    pub fn into_parts(self) -> (SendReceipt, Outgoing) {
        (self.receipt, self.outgoing)
    }
}
/// A bounded preparation failure never removes or acknowledges pending input.
#[derive(Debug, thiserror::Error)]
pub enum SenderError {
    /// The complete next chunk could not reserve its bounded allocation.
    #[error("terminal input allocation is unavailable")]
    Allocation(#[source] std::collections::TryReserveError),
    /// The private queue's consumed UTF8 prefix is not valid.
    #[error("terminal input queue is unavailable")]
    InvalidQueue,
}

/// One queue per retained owner; it grants no node or input authority.
#[derive(Debug, Default)]
pub struct TerminalSender {
    items: VecDeque<Pending>,
    chars: Characters,
    busy: Option<SendReceipt>,
    closed: bool,
    overflowing: bool,
}
impl TerminalSender {
    /// Admit complete keys; a partially consumed blob is never extended indefinitely.
    pub fn keys(&mut self, text: &str) -> Taken {
        if self.closed {
            return Taken::Closed;
        }
        if text.is_empty() {
            return Taken::Queued;
        }
        let count = Characters::of(text);
        if self.chars.get().saturating_add(count.get()) > MAX_PENDING {
            if self.overflowing {
                return Taken::Dropped;
            }
            self.overflowing = true;
            return Taken::DroppedFirst;
        }
        if let Some(last) = self.items.back_mut().filter(|item| item.can_append()) {
            if last.append(text).is_err() {
                return Taken::Unavailable;
            }
        } else {
            let Ok(item) = Pending::keys(text) else {
                return Taken::Unavailable;
            };
            if self.items.try_reserve(1).is_err() {
                return Taken::Unavailable;
            }
            self.items.push_back(item);
        }
        self.chars.add(count);
        self.overflowing = false;
        Taken::Queued
    }
    /// Adjacent sizes coalesce, but never cross a queued key/paste boundary.
    #[must_use]
    pub fn resize(&mut self, size: super::GridSize) -> Taken {
        if self.closed {
            return Taken::Closed;
        }
        if let Some(Pending::Resize { cols, rows }) = self.items.back_mut() {
            *cols = size.cols();
            *rows = size.rows();
        } else {
            if self.items.try_reserve(1).is_err() {
                return Taken::Unavailable;
            }
            self.items.push_back(Pending::Resize {
                cols: size.cols(),
                rows: size.rows(),
            });
        }
        Taken::Queued
    }
    /// Prepare one bounded next call, without releasing its in-flight ownership.
    ///
    /// # Errors
    /// Refuses allocation/private-prefix failure without consuming queued input.
    pub fn take(&mut self) -> Result<Option<SendDispatch>, SenderError> {
        if self.busy.is_some() || self.closed {
            return Ok(None);
        }
        let Some(item) = self.items.front_mut() else {
            return Ok(None);
        };
        let batch = item.take()?;
        self.chars.remove(batch.chars);
        if batch.remainder == Remainder::Empty {
            self.items.pop_front();
        }
        let receipt = SendReceipt(Arc::new(()));
        self.busy = Some(receipt.clone());
        Ok(Some(SendDispatch {
            receipt,
            outgoing: batch.outgoing,
        }))
    }
    /// Settle the exact call once; the owner records delivery failure/uncertainty separately.
    pub fn answered(&mut self, receipt: &SendReceipt) -> SendSettlement {
        if self.busy.as_ref() != Some(receipt) {
            return SendSettlement::Stale;
        }
        self.busy = None;
        SendSettlement::Settled
    }
    /// Explicitly close/discard unsent input; return the retired in-flight identity.
    /// Sent work remains owned by its caller and must never be auto-retried.
    #[must_use]
    pub fn close(&mut self) -> Option<SendReceipt> {
        self.closed = true;
        self.items.clear();
        self.chars = Characters::default();
        self.busy.take()
    }
    /// Whether one exact call still owns the serial lane.
    #[must_use]
    pub const fn busy(&self) -> bool {
        self.busy.is_some()
    }
}
#[cfg(test)]
mod tests;
