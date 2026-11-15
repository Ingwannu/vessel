//! Identified delivery never retries through the unkeyed prompt endpoint.

use crate::NodeClient;
use vessel_proto::timeline::{SubmitPrompt, SubmitPromptReply};
use vessel_rpc::CallError;

impl NodeClient {
    /// Answer a blocking/durable question with a correlated committed acknowledgement.
    ///
    /// # Errors
    /// Unsupported peers, scope/ownership refusal or transport/persistence errors.
    pub async fn respond_question(
        &self,
        request: vessel_proto::timeline::RespondQuestion,
    ) -> Result<vessel_proto::timeline::RespondQuestionReply, CallError> {
        self.call(request).await
    }

    /// Apply a supported offered decision and wait for its committed resolution.
    /// Rich-field refusal leaves the request unchanged; uncertainty never implies acceptance.
    ///
    /// # Errors
    /// Unsupported peers, scope/ownership refusal or transport/persistence errors.
    pub async fn respond_permission(
        &self,
        request: vessel_proto::timeline::RespondPermission,
    ) -> Result<vessel_proto::timeline::RespondPermissionReply, CallError> {
        self.call(request).await
    }

    /// Send the same prepared identity/payload on explicit reconciliation retries.
    /// `OutcomeUnknown` forbids automatic resend with a fresh identity.
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
