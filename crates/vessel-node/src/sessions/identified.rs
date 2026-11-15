//! One keyed pipeline for prompt policies and native commands. Only a committed
//! fresh reservation authorizes dispatch; uncertain work never resends automatically.
use super::{Found, Sessions};
use crate::{error::ServiceError, persist};
use std::time::Duration;
use vessel_proto::Creator;
use vessel_proto::timeline::{
    NativeCommandRefusal, SubmissionDelivery, SubmissionRefusal, SubmitPrompt, SubmitPromptReply,
};
use vessel_session::{SessionHandle, SubmissionDispatch};
use vessel_store::{ReceiptAdmission, ReceiptToken};

const RECEIPT_COMMIT_TIMEOUT: Duration = Duration::from_secs(30);

impl Sessions {
    pub(crate) async fn submit_prompt(
        &self,
        mut request: SubmitPrompt,
        by: Creator,
    ) -> Result<SubmitPromptReply, ServiceError> {
        let found = self.find(request.session).await?;
        // Resolve upload references before hashing, without provider/queue effects.
        if request
            .content
            .iter()
            .any(|block| matches!(block, vessel_proto::ContentBlock::Attachment { .. }))
        {
            request.content =
                crate::attachments::load::resolve(&self.store, request.session, request.content)
                    .await?;
        }
        if let Some(reply) = self
            .store
            .prompt_receipt(request.clone(), by.clone())
            .await?
        {
            return Ok(reply);
        }
        let Found::Live {
            handle,
            mut durability,
        } = found
        else {
            return Ok(not_admitted(&request, SubmissionRefusal::SessionEnded));
        };
        if handle.info().spec.side_question_of.is_some() {
            return Err(crate::side_questions::SideError::PromptOnly.into());
        }
        if let Some(reason) = preflight(&handle, &request).await? {
            return Ok(not_admitted(&request, reason));
        }
        let token = match self.store.admit_prompt(request.clone(), by.clone()).await? {
            ReceiptAdmission::New(token) => token,
            ReceiptAdmission::Existing(reply) => return Ok(reply),
        };
        let message = request.message;
        let result = handle
            .submit_identified(
                request.content,
                message.clone(),
                request.delivery,
                by,
                request.native_command,
            )
            .await;
        let accepted = match result {
            Ok(SubmissionDispatch::Accepted(accepted)) => accepted,
            Ok(SubmissionDispatch::Refused(reason)) => {
                return self.refuse_identified(token, message, reason).await;
            }
            Ok(SubmissionDispatch::OutcomeUnknown) | Err(_) => {
                return Ok(SubmitPromptReply::OutcomeUnknown { message });
            }
        };
        // An accepted queue entry or command echo must survive a restart before
        // its receipt is committed. The same watermark also covers prompt echoes.
        let through = handle.info().next_seq;
        let persisted = tokio::time::timeout(
            RECEIPT_COMMIT_TIMEOUT,
            persist::wait_until(request.session, &mut durability, through),
        )
        .await;
        if !matches!(persisted, Ok(Ok(()))) {
            return Ok(SubmitPromptReply::OutcomeUnknown { message });
        }
        match self.store.accept_prompt(token, accepted).await {
            Ok(receipt) => Ok(SubmitPromptReply::Accepted { receipt }),
            Err(error) => {
                tracing::warn!(session=%request.session, %error, "identified acceptance remains uncertain after commit failure");
                Ok(SubmitPromptReply::OutcomeUnknown { message })
            }
        }
    }

    async fn refuse_identified(
        &self,
        token: ReceiptToken,
        message: vessel_proto::timeline::ClientMessageId,
        reason: SubmissionRefusal,
    ) -> Result<SubmitPromptReply, ServiceError> {
        match self.store.refuse_prompt(token, reason).await {
            Ok(reply) => Ok(reply),
            Err(_) => Ok(SubmitPromptReply::OutcomeUnknown { message }),
        }
    }
}

async fn preflight(
    handle: &SessionHandle,
    request: &SubmitPrompt,
) -> Result<Option<SubmissionRefusal>, ServiceError> {
    if let Some(intent) = &request.native_command {
        if request.delivery != SubmissionDelivery::Start
            || !intent.matches_content(&request.content)
        {
            return Ok(Some(SubmissionRefusal::InvalidContent));
        }
        let offers = handle.native_command_offers().await?;
        return Ok(
            (!offers.supports(intent.kind())).then_some(SubmissionRefusal::NativeCommand {
                cause: NativeCommandRefusal::NotOffered,
            }),
        );
    }
    handle.validate_prompt(&request.content)?;
    Ok(None)
}
fn not_admitted(request: &SubmitPrompt, reason: SubmissionRefusal) -> SubmitPromptReply {
    SubmitPromptReply::NotAdmitted {
        message: request.message.clone(),
        reason,
    }
}
