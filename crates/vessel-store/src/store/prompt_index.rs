//! Prompt-scan admission belongs to the store's blocking work lifetime.

use std::sync::Arc;
use vessel_proto::prompt_index::{ListSessionPrompts, PromptIndexPage};

use super::Store;
use crate::StoreError;

impl Store {
    /// Read metadata without downloading message bodies or starting a harness.
    /// # Errors
    /// Unknown session, changed revision, exceeded work budget, busy or database failure.
    pub async fn prompt_index(
        &self,
        request: ListSessionPrompts,
    ) -> Result<PromptIndexPage, StoreError> {
        self.prompt_read(move |reader| reader.prompt_index(request))
            .await
    }

    async fn prompt_read<T: Send + 'static>(
        &self,
        read: impl FnOnce(&crate::reader::Reader) -> Result<T, StoreError> + Send + 'static,
    ) -> Result<T, StoreError> {
        let permit = Arc::clone(&self.prompt_reads)
            .try_acquire_owned()
            .map_err(StoreError::PromptIndexBusy)?;
        self.read(move |reader| {
            // A disconnected requester must not release a still-running SQL scan.
            let _place = permit;
            read(reader)
        })
        .await
    }
}

#[cfg(test)]
mod tests;
