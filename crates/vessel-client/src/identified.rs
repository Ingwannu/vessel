//! Identified delivery never retries through the unkeyed prompt endpoint.

use crate::NodeClient;
use vessel_proto::timeline::{SubmitPrompt, SubmitPromptReply};
use vessel_rpc::CallError;

impl NodeClient {
    /// Send the same prepared identity/payload on explicit reconciliation retries.
    /// OutcomeUnknown forbids automatic resend with a fresh identity.
    ///
    /// # Errors
    /// Unsupported/unassigned peers, access refusal, or transport/store failures.
    pub async fn submit_prompt(
        &self,
        request: SubmitPrompt,
    ) -> Result<SubmitPromptReply, CallError> {
        self.call(request).await
    }
}
