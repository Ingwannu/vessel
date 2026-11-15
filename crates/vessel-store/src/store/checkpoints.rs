//! Typed checkpoint caller facade. The existing writer owns transactions and durable replies.
use super::Store;
use crate::{StoreError, checkpoint::CheckpointWrite, writer::Write};

impl Store {
    /// Commit one checkpoint command through the existing writer with FULL sync.
    /// The node owns authorization, physical admission and actual I/O completion.
    ///
    /// # Errors
    /// Stale/foreign state, quota/admission refusal, encoding or commit failure.
    pub async fn checkpoint_write(&self, write: CheckpointWrite) -> Result<(), StoreError> {
        self.write(|done| Write::Checkpoint {
            write: Box::new(write),
            done,
        })
        .await
    }
}
