//! Caller identities travel only as local metadata, never as provider authority.

use super::{AgentInput, Command, SteerOutcome};
use crate::{Attachments, InputError};
use tokio::sync::oneshot;
use vessel_proto::{ContentBlock, timeline::ClientMessageId};

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
        self.call(|reply| Command::Prompt {
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
        let (reply, result) = oneshot::channel();
        self.commands
            .send(Command::Steer {
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
