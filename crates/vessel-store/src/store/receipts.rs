//! Durable identified-prompt operations, always acknowledged after the writer commits.

use tokio::sync::oneshot;
use vessel_proto::timeline::{PromptReceipt, SubmitPrompt, SubmitPromptReply};
use vessel_proto::{Creator, Delivered};

use super::Store;
use crate::{ReceiptAdmission, ReceiptToken, StoreError, writer::Write};

impl Store {
    /// Look up a retained outcome without reserving a fresh identity or dispatching.
    /// Replay remains available after a session stops, before live-provider preflight.
    ///
    /// # Errors
    /// A stored receipt cannot be decoded or the database cannot be read.
    pub async fn prompt_receipt(
        &self,
        request: SubmitPrompt,
        creator: Creator,
    ) -> Result<Option<SubmitPromptReply>, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| {
                crate::timeline_admission::lookup_input(connection, &request, &creator)
            })
        })
        .await
    }

    /// Reserve replay protection before dispatch, or return an exact retained outcome.
    /// Provider-specific validation and authorization belong to the caller's preflight.
    ///
    /// # Errors
    /// Empty input, unknown session, corrupt accounting, or a failed store commit.
    pub async fn admit_prompt(
        &self,
        request: SubmitPrompt,
        creator: Creator,
    ) -> Result<ReceiptAdmission, StoreError> {
        let (done, committed) = oneshot::channel();
        self.send(Write::AdmitPrompt {
            request: Box::new(request),
            creator,
            done,
        })
        .await?;
        committed.await.map_err(|_dropped| StoreError::Closed)?
    }

    /// Settle one fresh reservation only after provider/queue acceptance was observed.
    /// Dropping the token deliberately leaves an uncertain protected identity.
    ///
    /// # Errors
    /// The reservation disappeared/changed, its result exceeds the envelope, or commit failed.
    pub async fn accept_prompt(
        &self,
        token: ReceiptToken,
        delivered: Delivered,
    ) -> Result<PromptReceipt, StoreError> {
        let (done, committed) = oneshot::channel();
        self.send(Write::AcceptPrompt {
            token,
            delivered,
            done,
        })
        .await?;
        committed.await.map_err(|_dropped| StoreError::Closed)?
    }
}
