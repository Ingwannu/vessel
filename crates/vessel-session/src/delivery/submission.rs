//! Keyed delivery policies reuse the existing actor, attachment loader and pump.
//! Wire attachment/retained outcome storage remain with the node receipt owner.
use super::TurnEnd;
use crate::moving::Control;
use crate::structured::StructuredActor;
use crate::{SessionError, SessionHandle};
use tokio::sync::oneshot;
use vessel_proto::native_commands::NativeCommandIntent;
use vessel_proto::timeline::{
    ClientMessageId, PromptDisposition, SubmissionAcceptedKind, SubmissionDelivery,
    SubmissionRefusal,
};
use vessel_proto::{ContentBlock, Creator, Delivered};

/// Actual actor acceptance, never inferred from a requested delivery policy.
#[derive(Debug)]
pub enum SubmissionDispatch {
    /// The driver/queue accepted this one delivery.
    Accepted(SubmissionAcceptedKind),
    /// Validation/state refusal before cancellation or dispatch.
    Refused(SubmissionRefusal),
    /// Cancellation or sending may already have happened; never automatically retry.
    OutcomeUnknown,
}
#[derive(Debug)]
pub(crate) struct Submit {
    content: Vec<ContentBlock>,
    message: ClientMessageId,
    delivery: SubmissionDelivery,
    by: Creator,
    native_command: Option<NativeCommandIntent>,
    reply: oneshot::Sender<Result<SubmissionDispatch, SessionError>>,
}

impl SessionHandle {
    /// The node receipt owner uses this once after its durable Pending reservation.
    /// No caller-side capability read or fallback can race another actor command.
    /// # Errors
    /// Moving/ended/terminal sessions refuse before the actor accepts this order;
    /// a lost actor reply must remain an uncertain retained receipt at the node.
    pub async fn submit_identified(
        &self,
        content: Vec<ContentBlock>,
        message: ClientMessageId,
        delivery: SubmissionDelivery,
        by: Creator,
        native_command: Option<NativeCommandIntent>,
    ) -> Result<SubmissionDispatch, SessionError> {
        let route = self.structured("an identified submission")?;
        self.order(route, |reply| {
            Control::Submit(Submit {
                content,
                message,
                delivery,
                by,
                native_command,
                reply,
            })
        })
        .await
    }
}
impl StructuredActor {
    pub(crate) async fn submit_identified(&mut self, order: Submit) {
        let result = if let Some(intent) = order.native_command {
            self.submit_native_command(order.content, order.message, order.delivery, intent)
                .await
        } else if order.content.is_empty()
            || vessel_agent::validate_prompt(
                self.launch.driver,
                &order.content,
                self.desk.abilities(),
            )
            .is_err()
        {
            SubmissionDispatch::Refused(SubmissionRefusal::InvalidContent)
        } else {
            self.submit_prompt_policy(order.content, order.message, order.delivery, order.by)
                .await
        };
        let _closed = order.reply.send(Ok(result));
    }

    async fn submit_prompt_policy(
        &mut self,
        content: Vec<ContentBlock>,
        message: ClientMessageId,
        delivery: SubmissionDelivery,
        by: Creator,
    ) -> SubmissionDispatch {
        let legacy = match delivery {
            SubmissionDelivery::Start => vessel_proto::Delivery::Start,
            SubmissionDelivery::Queue => vessel_proto::Delivery::Queue,
            SubmissionDelivery::Steer => vessel_proto::Delivery::Steer,
            SubmissionDelivery::Interrupt => return self.submit_interrupt(content, message).await,
            SubmissionDelivery::SteerOrInterrupt => {
                if self.desk.running() && !self.desk.abilities().steer {
                    return self.submit_interrupt(content, message).await;
                }
                vessel_proto::Delivery::Steer
            }
        };
        match self.send_as(content, legacy, by, Some(message)).await {
            Ok(delivered) => accepted(disposition(delivered)),
            Err(error) => known_or_uncertain(error),
        }
    }

    async fn submit_interrupt(
        &mut self,
        content: Vec<ContentBlock>,
        message: ClientMessageId,
    ) -> SubmissionDispatch {
        // Loading/validating all attachments precedes cancellation. No control
        // order or queue advance can overtake this owner while it pumps outputs.
        let attachments = match self.load_attachments(&content).await {
            Ok(attachments) => attachments,
            Err(_) => {
                return SubmissionDispatch::Refused(SubmissionRefusal::AttachmentsUnavailable);
            }
        };
        let mut interrupted = None;
        let mut cancel_attempted = false;
        if self.desk.running() {
            let Some(held) = self.desk.turn() else {
                return SubmissionDispatch::Refused(SubmissionRefusal::Unavailable);
            };
            cancel_attempted = true;
            self.desk.last_end = None;
            self.desk.interrupting = true;
            let over = self.cancel_and_wait().await;
            self.desk.interrupting = false;
            if !matches!(over, Ok(true)) {
                return SubmissionDispatch::OutcomeUnknown;
            }
            if self.desk.last_end == Some(TurnEnd::Cancelled) {
                interrupted = Some(held);
            }
        }
        if let Err(error) = self.start_loaded(content, attachments, Some(message)).await {
            // Even a precise prompt refusal cannot undo an already sent cancel.
            return if cancel_attempted {
                SubmissionDispatch::OutcomeUnknown
            } else {
                known_or_uncertain(error)
            };
        }
        accepted(
            interrupted.map_or(PromptDisposition::Started, |interrupted| {
                PromptDisposition::InterruptedAndStarted { interrupted }
            }),
        )
    }
}
fn accepted(disposition: PromptDisposition) -> SubmissionDispatch {
    SubmissionDispatch::Accepted(SubmissionAcceptedKind::Prompt { disposition })
}
fn disposition(delivered: Delivered) -> PromptDisposition {
    match delivered {
        Delivered::Started => PromptDisposition::Started,
        Delivered::Steered => PromptDisposition::Steered,
        Delivered::Queued { entry } => PromptDisposition::Queued { entry },
    }
}
fn known_or_uncertain(error: SessionError) -> SubmissionDispatch {
    let reason = match error {
        SessionError::Input(vessel_agent::InputError::TurnInProgress) => {
            SubmissionRefusal::TurnRunning
        }
        SessionError::Input(vessel_agent::InputError::CannotSteer) => {
            SubmissionRefusal::Unavailable
        }
        SessionError::Input(
            vessel_agent::InputError::InvalidContent(_) | vessel_agent::InputError::ContextLimit,
        ) => SubmissionRefusal::InvalidContent,
        SessionError::Attachment(_) => SubmissionRefusal::AttachmentsUnavailable,
        SessionError::Queue(_) => SubmissionRefusal::QueueFull,
        _ => return SubmissionDispatch::OutcomeUnknown,
    };
    SubmissionDispatch::Refused(reason)
}
