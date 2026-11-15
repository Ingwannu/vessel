//! Identified prompts share the ordinary serialized delivery and attachment path.
//! Only a committed fresh reservation can authorize one dispatch.

use super::{Found, Sessions};
use crate::error::ServiceError;
use vessel_proto::{
    Creator,
    timeline::{SubmitPrompt, SubmitPromptReply},
};
use vessel_store::ReceiptAdmission;

impl Sessions {
    pub(crate) async fn submit_prompt(
        &self,
        mut request: SubmitPrompt,
        by: Creator,
    ) -> Result<SubmitPromptReply, ServiceError> {
        let found = self.find(request.session).await?;
        // Canonicalize upload references before hashing. This has no provider/queue effect.
        if request
            .content
            .iter()
            .any(|block| matches!(block, vessel_proto::ContentBlock::Attachment { .. }))
        {
            request.content =
                crate::attachments::load::resolve(&self.store, request.session, request.content)
                    .await?;
        }
        // A retained accepted/pending identity still resolves after the process stops.
        if let Some(reply) = self
            .store
            .prompt_receipt(request.clone(), by.clone())
            .await?
        {
            return Ok(reply);
        }
        let Found::Live { handle, .. } = found else {
            return Err(ServiceError::Ended(request.session));
        };
        if handle.info().spec.side_question_of.is_some() {
            return Err(crate::side_questions::SideError::PromptOnly.into());
        }
        handle.validate_prompt(&request.content)?;
        let admission = self.store.admit_prompt(request.clone(), by.clone()).await?;
        let token = match admission {
            ReceiptAdmission::New(token) => token,
            ReceiptAdmission::Existing(reply) => return Ok(reply),
        };
        let message = request.message;
        let delivered = match handle
            .prompt_identified(request.content, request.delivery, by, Some(message.clone()))
            .await
        {
            Ok(delivered) => delivered,
            Err(error) => {
                // The durable Pending reservation is intentionally retained. Even a lost
                // driver acknowledgement cannot turn this key into another dispatch.
                tracing::warn!(session=%request.session, %error, "identified prompt outcome remains uncertain");
                return Ok(SubmitPromptReply::OutcomeUnknown { message });
            }
        };
        match self.store.accept_prompt(token, delivered).await {
            Ok(receipt) => Ok(SubmitPromptReply::Accepted { receipt }),
            Err(error) => {
                tracing::warn!(session=%request.session, %error, "identified prompt acceptance could not be durably acknowledged");
                Ok(SubmitPromptReply::OutcomeUnknown { message })
            }
        }
    }
}
