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
        let permit = Arc::clone(&self.prompt_reads)
            .try_acquire_owned()
            .map_err(StoreError::PromptIndexBusy)?;
        self.read(move |reader| {
            // A disconnected requester must not release a still-running SQL scan.
            let _place = permit;
            reader.prompt_index(request)
        })
        .await
    }
}
