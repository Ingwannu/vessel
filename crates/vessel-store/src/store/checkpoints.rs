//! Staged caller facade. The existing writer owns transactions and durable replies.
use super::Store;
use crate::{StoreError, checkpoint::CheckpointWrite, writer::Write};

impl Store {
    pub(crate) async fn checkpoint_write(&self, write: CheckpointWrite) -> Result<(), StoreError> {
        self.write(|done| Write::Checkpoint {
            write: Box::new(write),
            done,
        })
        .await
    }
}
