//! Keyed delivery policies reuse the existing actor, attachment loader and pump.
//! Wire attachment/retained outcome storage remain with the node receipt owner.
use super::{TurnEnd, TurnTicket};
use crate::moving::Control;
use crate::structured::StructuredActor;
use crate::{SessionError, SessionHandle};
use tokio::sync::oneshot;
use vessel_proto::native_commands::NativeCommandIntent;
use vessel_proto::timeline::{
    ClientMessageId, PromptDisposition, SubmissionAcceptedKind, SubmissionDelivery,
    SubmissionRefusal,
};
use vessel_proto::{ContentBlock, Creator, Delivered, EventSeq, QueueEntryId};

/// An exact owning-actor ticket fact, separate from immutable delivery acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmissionTarget {
    /// The actual canonical start record of the turn this submission started/joined.
    Turn {
        /// Actor-observed original record, including a turn that already completed.
        started: EventSeq,
        /// This exact accepted input's canonical echo, distinct for a steer.
        echo: EventSeq,
    },
    /// An explicit queue mutation removed this entry before it was sent.
    Withdrawn {
        /// Actual removed entry; channel loss alone never constructs this value.
        entry: QueueEntryId,
        /// Exclusive watermark after the canonical queue removal was published.
        through: EventSeq,
    },
}

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
    turn: Option<TurnTicket>,
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
        self.submit_identified_ticket(content, message, delivery, by, native_command, None)
            .await
    }

    /// Admit the same keyed delivery and retain the existing actor's exact target
    /// ticket. Lost/closed tickets are uncertainty, never a no-send withdrawal.
    ///
    /// # Errors
    /// As `submit_identified`; receipt persistence remains the node owner's duty.
    pub async fn submit_identified_tracked(
        &self,
        content: Vec<ContentBlock>,
        message: ClientMessageId,
        delivery: SubmissionDelivery,
        by: Creator,
        native_command: Option<NativeCommandIntent>,
    ) -> Result<(SubmissionDispatch, oneshot::Receiver<SubmissionTarget>), SessionError> {
        let (ticket, target) = oneshot::channel();
        let ticket = TurnTicket::receipt(message.clone(), ticket);
        let result = self
            .submit_identified_ticket(content, message, delivery, by, native_command, Some(ticket))
            .await?;
        Ok((result, target))
    }

    async fn submit_identified_ticket(
        &self,
        content: Vec<ContentBlock>,
        message: ClientMessageId,
        delivery: SubmissionDelivery,
        by: Creator,
        native_command: Option<NativeCommandIntent>,
        turn: Option<TurnTicket>,
    ) -> Result<SubmissionDispatch, SessionError> {
        let route = self.structured("an identified submission")?;
        self.order(route, |reply| {
            Control::Submit(Submit {
                content,
                message,
                delivery,
                by,
                native_command,
                turn,
                reply,
            })
        })
        .await
    }
}
impl StructuredActor {
    pub(crate) async fn submit_identified(&mut self, mut order: Submit) {
        // Register the ORIGINAL bounded owner before any provider/cancel/queue
        // await. OOB commands deliberately have no model-turn ticket.
        if order.native_command.is_none()
            && let Some(ticket) = order.turn.take()
            && self.desk.tickets.prepare(ticket).is_err()
        {
            let _observer_gone = order.reply.send(Ok(SubmissionDispatch::Refused(
                SubmissionRefusal::Unavailable,
            )));
            return;
        }
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
        if let SubmissionDispatch::Accepted(SubmissionAcceptedKind::Prompt { disposition }) =
            &result
        {
            let delivered = match disposition {
                PromptDisposition::Started | PromptDisposition::InterruptedAndStarted { .. } => {
                    Delivered::Started
                }
                PromptDisposition::Steered => Delivered::Steered,
                PromptDisposition::Queued { entry } => Delivered::Queued { entry: *entry },
            };
            self.desk
                .tickets
                .finish_prepared(delivered, self.desk.started);
        } else if matches!(result, SubmissionDispatch::OutcomeUnknown) {
            self.desk.tickets.uncertain();
        } else {
            let _no_effect_or_command = self.desk.tickets.take_prepared();
        }
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
        match self.send_as(content, legacy, by, Some(message), None).await {
            Ok(delivered) => accepted(disposition(delivered)),
            Err(error) => known_or_uncertain(&error),
        }
    }

    async fn submit_interrupt(
        &mut self,
        content: Vec<ContentBlock>,
        message: ClientMessageId,
    ) -> SubmissionDispatch {
        // Loading/validating all attachments precedes cancellation. No control
        // order or queue advance can overtake this owner while it pumps outputs.
        let Ok(attachments) = self.load_attachments(&content).await else {
            return SubmissionDispatch::Refused(SubmissionRefusal::AttachmentsUnavailable);
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
                known_or_uncertain(&error)
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
pub(super) fn known_or_uncertain(error: &SessionError) -> SubmissionDispatch {
    let reason = match error {
        SessionError::Input(vessel_agent::InputError::TurnInProgress) => {
            SubmissionRefusal::TurnRunning
        }
        SessionError::Input(vessel_agent::InputError::CannotSteer) => {
            SubmissionRefusal::Unavailable
        }
        SessionError::Delivery(crate::DeliveryError::ReceiptCapacity) => {
            SubmissionRefusal::Unavailable
        }
        SessionError::Input(
            vessel_agent::InputError::InvalidContent(_) | vessel_agent::InputError::ContextLimit,
        ) => SubmissionRefusal::InvalidContent,
        SessionError::Attachment(_) => SubmissionRefusal::AttachmentsUnavailable,
        SessionError::Queue(_) => SubmissionRefusal::QueueFull,
        SessionError::Launch(_)
        | SessionError::Spawn(_)
        | SessionError::McpFile(_)
        | SessionError::StartAgent(_)
        | SessionError::NoWorkingDirectory
        | SessionError::Input(_)
        | SessionError::Delivery(_)
        | SessionError::Terminal(_)
        | SessionError::NoOptions
        | SessionError::Probe(_)
        | SessionError::Unsupported { .. }
        | SessionError::NotFound(_)
        | SessionError::DuplicateId(_)
        | SessionError::Moving
        | SessionError::Move(_)
        | SessionError::Ended
        | SessionError::Parked
        | SessionError::NotParked(_)
        | SessionError::WakeFailed(_)
        | SessionError::NotRewound(_)
        | SessionError::ForkRefused(_)
        | SessionError::History(_)
        | SessionError::RewindFailed(_)
        | SessionError::ConversationTaken(_)
        | SessionError::NotResumable(_)
        | SessionError::ShuttingDown
        | SessionError::Internal(_) => return SubmissionDispatch::OutcomeUnknown,
    };
    SubmissionDispatch::Refused(reason)
}
