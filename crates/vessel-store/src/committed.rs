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

    /// The exact acknowledged clock, including a preserved current value for
    /// an empty append or attention no-op. None means the row remains legacy Unknown.
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
