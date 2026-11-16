//! Caller identities travel only as local metadata, never as provider authority.

use super::{AgentInput, Command, SteerOutcome};
use crate::{Attachments, InputError};
use tokio::sync::oneshot;
use vessel_proto::{
    ContentBlock,
    timeline::{ClientMessageId, PromptProvenance},
};

impl AgentInput {
    /// Start a turn while preserving the exact optional caller identity in its echo.
    ///
    /// # Errors
    /// The ordinary prompt errors; refusal never substitutes an unkeyed prompt.
    pub async fn prompt_identified(
        &self,
        content: Vec<ContentBlock>,
        attachments: Attachments,
        client_message_id: Option<ClientMessageId>,
    ) -> Result<(), InputError> {
        self.prompt_admitted(content, attachments, client_message_id, None)
            .await
    }

    /// The session embedding owner carries this exact input's already-admitted
    /// presentation link. It grants no submit, queue or permission authority.
    /// Legacy/public request adapters must use the ordinary None wrappers.
    ///
    /// # Errors
    /// The ordinary prompt errors; no local echo is produced on known refusal.
    pub async fn prompt_admitted(
        &self,
        content: Vec<ContentBlock>,
        attachments: Attachments,
        client_message_id: Option<ClientMessageId>,
        provenance: Option<PromptProvenance>,
    ) -> Result<(), InputError> {
        self.call(|reply| Command::Prompt {
            provenance,
            content,
            attachments,
            client_message_id,
            context: None,
            reply,
        })
        .await
    }

    /// Steer while preserving identity; `PromptRequired` does not emit or consume an echo.
    ///
    /// # Errors
    /// The ordinary steering errors, including connection loss and refusal.
    pub async fn steer_identified(
        &self,
        content: Vec<ContentBlock>,
        attachments: Attachments,
        client_message_id: Option<ClientMessageId>,
    ) -> Result<SteerOutcome, InputError> {
        self.steer_admitted(content, attachments, client_message_id, None)
            .await
    }

    /// Preserve only this admitted input's presentation origin through actual
    /// steering acceptance. A PromptRequired result emits no accepted echo.
    ///
    /// # Errors
    /// The ordinary steering errors, with no fallback or replacement identity.
    pub async fn steer_admitted(
        &self,
        content: Vec<ContentBlock>,
        attachments: Attachments,
        client_message_id: Option<ClientMessageId>,
        provenance: Option<PromptProvenance>,
    ) -> Result<SteerOutcome, InputError> {
        let (reply, result) = oneshot::channel();
        self.commands
            .send(Command::Steer {
                provenance,
                content,
                attachments,
                client_message_id,
                reply,
            })
            .await
            .map_err(|_closed| InputError::Closed)?;
        result.await.unwrap_or(Err(InputError::Closed))
    }
}
