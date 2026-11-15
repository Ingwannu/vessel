//! One keyed pipeline for prompt policies and native commands. Only a committed
//! fresh reservation authorizes dispatch; uncertain work never resends automatically.

mod receipt_observation;
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
        // Replay binds the original submitted payload, not attachment records
        // that may have been pruned since acceptance. Lookup still follows the
        // ordinary authorized session lookup and never loads or sends anything.
        if let Some(reply) = self
            .store
            .prompt_receipt(request.clone(), by.clone())
            .await?
        {
            return Ok(reply);
        }
        let original = request.clone();
        let Found::Live { handle, durability } = found else {
            return Ok(not_admitted(&request, SubmissionRefusal::SessionEnded));
        };
        if handle.info().spec.side_question_of.is_some() {
            return Err(crate::side_questions::SideError::PromptOnly.into());
        }
        // Only fresh work resolves caller references to trusted store metadata.
        // That resolved content reaches the provider, while replay/conflict stays
        // bound to the exact retained client payload, including every reference.
        if request
            .content
            .iter()
            .any(|block| matches!(block, vessel_proto::ContentBlock::Attachment { .. }))
        {
            request.content =
                crate::attachments::load::resolve(&self.store, request.session, request.content)
                    .await?;
        }
        if let Some(reason) = preflight(&handle, &request).await? {
            return Ok(not_admitted(&request, reason));
        }
        let token = match self.store.admit_prompt(original, by.clone()).await? {
            ReceiptAdmission::New(token) => token,
            ReceiptAdmission::Existing(reply) => return Ok(reply),
        };
        self.dispatch_identified(request, by, handle, durability, token)
            .await
    }

    async fn dispatch_identified(
        &self,
        request: SubmitPrompt,
        by: Creator,
        handle: SessionHandle,
        mut durability: tokio::sync::watch::Receiver<persist::Durability>,
        token: ReceiptToken,
    ) -> Result<SubmitPromptReply, ServiceError> {
        let message = request.message;
        let result = handle
            .submit_identified_tracked(
                request.content,
                message.clone(),
                request.delivery,
                by,
                request.native_command,
            )
            .await;
        let (accepted, target) = match result {
            Ok((SubmissionDispatch::Accepted(accepted), target)) => (accepted, target),
            Ok((SubmissionDispatch::Refused(reason), _target)) => {
                return self.refuse_identified(token, message, reason).await;
            }
            Err(vessel_session::SessionError::Moving) => {
                // This error is produced only by SessionHandle::order before
                // enqueuing the command, never by a possibly sent provider RPC.
                return self
                    .refuse_identified(token, message, SubmissionRefusal::Moving)
                    .await;
            }
            Ok((SubmissionDispatch::OutcomeUnknown, _target)) | Err(_) => {
                return Ok(SubmitPromptReply::OutcomeUnknown { message });
            }
        };
        // An accepted queue entry or command echo must survive a restart before
        // its receipt is committed. The same watermark also covers prompt echoes.
        let Some(barrier) =
            receipt_observation::target_barrier(accepted, target, handle.info().next_seq).await
        else {
            return Ok(SubmitPromptReply::OutcomeUnknown { message });
        };
        let persisted = tokio::time::timeout(
            RECEIPT_COMMIT_TIMEOUT,
            persist::wait_until(request.session, &mut durability, barrier.through),
        )
        .await;
        if !matches!(persisted, Ok(Ok(()))) {
            return Ok(SubmitPromptReply::OutcomeUnknown { message });
        }
        let binding = token.binding_token();
        match self.store.accept_prompt(token, accepted).await {
            Ok(receipt) => {
                if let Some(target) = barrier.target {
                    self.retain_receipt_target(request.session, binding, target, durability);
                }
                Ok(SubmitPromptReply::Accepted { receipt })
            }
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
