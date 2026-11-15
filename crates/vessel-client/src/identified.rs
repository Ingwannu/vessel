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

    /// Read-or-admit one prepared identity/payload. This is never observation:
    /// an absent key can dispatch work. Uncertainty does not authorize resend.
    ///
    /// # Errors
    /// Unsupported/unassigned peers, access refusal, or transport/store failures.
    pub async fn submit_prompt(
        &self,
        request: SubmitPrompt,
    ) -> Result<SubmitPromptReply, CallError> {
        self.call(request).await
    }

    /// Observe one original request without creating a reservation or dispatch.
    /// Missing or uncertain results never authorize resend or a fresh identity.
    ///
    /// # Errors
    /// Unsupported/unassigned peers, ordinary access refusal or read failures.
    pub async fn observe_prompt_receipt(
        &self,
        request: vessel_proto::timeline::ObservePromptReceipt,
    ) -> Result<vessel_proto::timeline::ObservePromptReceiptReply, CallError> {
        self.call(request).await
    }
}
