//! A session write's own result and clock, captured before later queued writes.

use vessel_proto::SessionUpdatedAt;

/// Returned only after the writer commits. The clock belongs to this result,
/// not a later row read; a delayed consumer must not regress a newer known one.
/// This is data about local persistence, never an execution permission.
#[derive(Debug)]
#[must_use]
pub struct SessionCommit<T> {
    value: T,
    updated_at: Option<SessionUpdatedAt>,
}

impl<T> SessionCommit<T> {
    /// Capture this result before the writer advances to another queued operation.
    pub(crate) const fn new(value: T, updated_at: Option<SessionUpdatedAt>) -> Self {
        Self { value, updated_at }
    }

    /// Keep the original convenience API's result when its caller does not
    /// maintain a live copy of the durable clock.
    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }

    /// The exact acknowledged clock. None preserves legacy Unknown when an
    /// operation such as an empty append performs no mutation.
    #[must_use]
    pub const fn updated_at(&self) -> Option<SessionUpdatedAt> {
        self.updated_at
    }

    /// Publish marks and their acknowledged time without a second database
    /// read that might already observe a later queued operation.
    #[must_use]
    pub fn into_parts(self) -> (T, Option<SessionUpdatedAt>) {
        (self.value, self.updated_at)
    }
}

/// Where a person's marked session is held when its Read/Unread mark reaches
/// the writer: the node decides this, because only it knows which sessions
/// its runtime has.
///
/// [Decision Log]
/// - Purpose: Paseo parity for the session mutation clock (ADR 0063, TRACKER
///   775). Paseo's live mark-read and mark-unread change attention without
///   touching updatedAt (`agent-manager.ts` 2389-2414); its stored paths set
///   it (`session.ts` 7697-7712, `agent-manager.ts` 2420-2433), and a stored
///   mark-read on a record that needs no attention does nothing.
/// - Alternative rejected: reading the row's `live` column inside the store.
///   It flips when the end record persists, while the runtime may still hold
///   the session until hand-over, so it would not match what the node served.
/// - Effect: other marks (pin, archive and their inverses) tick as before.
///   The value is a fact about the caller's lookup, not a permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Residence {
    /// The node's runtime has the session (Paseo's loaded agent): attention
    /// marks leave the clock alone.
    Loaded,
    /// The node serves the session from the store (Paseo's stored record):
    /// an attention mark ticks the clock when it changes attention.
    Stored,
}
