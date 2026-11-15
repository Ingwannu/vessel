//! Durable claims serialize against the exact actor foreground before provider dispatch.

use super::{PreparedQuestionAnswer, QuestionClaim, QuestionNotSent, QuestionTarget};
use crate::{SessionError, structured::StructuredActor};
use tokio::sync::oneshot;
use vessel_agent::{InputError, SteerOutcome};
use vessel_proto::timeline::{QuestionDeliveryState, QuestionResponseRefusal};
use vessel_proto::{
    AgentEvent, ContentBlock, QuestionAnswer, QuestionId, QuestionOutcome, SessionEvent,
};

/// Driver acceptance is followed by an exact committed-resolution wait in the node.
#[derive(Debug)]
pub enum QuestionDispatch {
    /// The observed real outcome, already published by the actor.
    Applied(QuestionOutcome),
    /// An authoritative typed refusal/state, with no new dispatch.
    Unchanged(QuestionResponseState),
}

/// Non-success states never fabricate a resolution event.
#[derive(Debug)]
pub enum QuestionResponseState {
    /// No such durable/live request.
    Vanished,
    /// A competing answer or withdrawal settled it.
    Conflict,
    /// A prior claim is in flight.
    Answering,
    /// A possible external effect must be reconciled.
    OutcomeUnknown,
    /// Input/state validation refused before effects.
    Refused(QuestionResponseRefusal),
}

#[derive(Debug)]
pub(crate) struct QuestionOrder {
    pub(crate) question: QuestionId,
    pub(crate) answer: QuestionAnswer,
    pub(crate) reply: oneshot::Sender<Result<QuestionDispatch, SessionError>>,
}

enum Effect {
    Accepted,
    NotSent(QuestionNotSent),
    Unknown,
}

impl StructuredActor {
    pub(crate) async fn answer_durable_question(&mut self, order: QuestionOrder) {
        let result = self
            .answer_question_claim(order.question, order.answer)
            .await;
        let _closed = order.reply.send(result);
    }

    async fn answer_question_claim(
        &mut self,
        id: QuestionId,
        answer: QuestionAnswer,
    ) -> Result<QuestionDispatch, SessionError> {
        let target = match self.desk.turn() {
            Some(started) => QuestionTarget::Turn { started },
            None if self.desk.running() => {
                return Ok(unchanged(QuestionResponseState::Refused(
                    QuestionResponseRefusal::TurnChanged,
                )));
            }
            None => QuestionTarget::Idle,
        };
        let Some(owner) = self.questions.clone() else {
            return self.answer_blocking_question(id, answer).await;
        };
        let Ok(claim) = owner
            .claim(self.life.out.session(), id, answer.clone(), target)
            .await
        else {
            return Ok(unchanged(QuestionResponseState::Refused(
                QuestionResponseRefusal::Unavailable,
            )));
        };
        let prepared = match claim {
            QuestionClaim::Ready(prepared) => prepared,
            QuestionClaim::Missing => return self.answer_blocking_question(id, answer).await,
            QuestionClaim::Answering => return Ok(unchanged(QuestionResponseState::Answering)),
            QuestionClaim::OutcomeUnknown => {
                return Ok(unchanged(QuestionResponseState::OutcomeUnknown));
            }
            QuestionClaim::Settled => return Ok(unchanged(QuestionResponseState::Conflict)),
            QuestionClaim::Invalid => {
                return Ok(unchanged(QuestionResponseState::Refused(
                    QuestionResponseRefusal::InvalidAnswer,
                )));
            }
            QuestionClaim::TooLarge => {
                return Ok(unchanged(QuestionResponseState::Refused(
                    QuestionResponseRefusal::TooLarge,
                )));
            }
        };
        self.apply_question_claim(prepared).await
    }

    async fn answer_blocking_question(
        &mut self,
        id: QuestionId,
        answer: QuestionAnswer,
    ) -> Result<QuestionDispatch, SessionError> {
        let outcome = match &answer {
            QuestionAnswer::Accept { values } => QuestionOutcome::Answered {
                values: values.clone(),
            },
            QuestionAnswer::Decline => QuestionOutcome::Declined,
        };
        let input = self.input.clone();
        let result = self
            .pump(async move { input.answer_question(id, answer).await })
            .await;
        match result {
            Ok(Ok(())) => Ok(QuestionDispatch::Applied(outcome)),
            Ok(Err(InputError::QuestionNotFound(_))) => {
                Ok(unchanged(QuestionResponseState::Vanished))
            }
            Ok(Err(InputError::QuestionSettled(_))) => {
                Ok(unchanged(QuestionResponseState::Conflict))
            }
            Ok(Err(InputError::InvalidAnswer { .. })) => Ok(unchanged(
                QuestionResponseState::Refused(QuestionResponseRefusal::InvalidAnswer),
            )),
            Ok(Err(error)) => Err(error.into()),
            Err(_stopped) => Ok(unchanged(QuestionResponseState::OutcomeUnknown)),
        }
    }

    async fn apply_question_claim(
        &mut self,
        prepared: PreparedQuestionAnswer,
    ) -> Result<QuestionDispatch, SessionError> {
        let PreparedQuestionAnswer {
            mut question,
            answer,
            target,
            lease,
        } = prepared;
        question.delivery_state = QuestionDeliveryState::Answering;
        self.publish_question(question.clone()).await;
        let effect = if let Some(prompt) = super::prompt::answer_prompt(&question, &answer) {
            self.dispatch_question_prompt(prompt, target).await
        } else {
            Effect::Accepted
        };
        match effect {
            Effect::Accepted => match lease.accepted().await {
                Ok(outcome) => {
                    self.life
                        .publish(
                            &mut self.agent,
                            SessionEvent::Agent {
                                event: AgentEvent::QuestionResolved {
                                    id: question.id,
                                    outcome: outcome.clone(),
                                },
                            },
                        )
                        .await;
                    Ok(QuestionDispatch::Applied(outcome))
                }
                Err(_) => Ok(unchanged(QuestionResponseState::OutcomeUnknown)),
            },
            Effect::NotSent(reason) => {
                let refusal = match reason {
                    QuestionNotSent::TurnChanged => QuestionResponseRefusal::TurnChanged,
                    QuestionNotSent::Unsupported => QuestionResponseRefusal::Unsupported,
                    QuestionNotSent::RejectedBeforeSend => QuestionResponseRefusal::InvalidAnswer,
                };
                if lease.not_sent(reason).await.is_err() {
                    return Ok(unchanged(QuestionResponseState::OutcomeUnknown));
                }
                question.delivery_state = QuestionDeliveryState::Pending;
                self.publish_question(question).await;
                Ok(unchanged(QuestionResponseState::Refused(refusal)))
            }
            Effect::Unknown => {
                if lease.uncertain().await.is_ok() {
                    question.delivery_state = QuestionDeliveryState::OutcomeUnknown;
                    self.publish_question(question).await;
                }
                Ok(unchanged(QuestionResponseState::OutcomeUnknown))
            }
        }
    }

    async fn dispatch_question_prompt(&mut self, prompt: String, target: QuestionTarget) -> Effect {
        let content = vec![ContentBlock::Text { text: prompt }];
        match target {
            QuestionTarget::Turn { started } => {
                if self.desk.turn() != Some(started) {
                    return Effect::NotSent(QuestionNotSent::TurnChanged);
                }
                let input = self.input.clone();
                match self
                    .pump(async move {
                        input
                            .steer_identified(content, vessel_agent::Attachments::new(), None)
                            .await
                    })
                    .await
                {
                    Ok(Ok(SteerOutcome::Injected)) => Effect::Accepted,
                    Ok(Ok(SteerOutcome::PromptRequired)) => {
                        Effect::NotSent(QuestionNotSent::TurnChanged)
                    }
                    Ok(Err(InputError::CannotSteer)) => {
                        Effect::NotSent(QuestionNotSent::Unsupported)
                    }
                    Ok(Err(
                        InputError::InvalidContent(_)
                        | InputError::ContextLimit
                        | InputError::SteerRefused { .. },
                    )) => Effect::NotSent(QuestionNotSent::RejectedBeforeSend),
                    Ok(Err(_)) | Err(_) => Effect::Unknown,
                }
            }
            QuestionTarget::Idle => {
                if self.desk.running() {
                    return Effect::NotSent(QuestionNotSent::TurnChanged);
                }
                match self.start(content, None).await {
                    Ok(()) => Effect::Accepted,
                    Err(SessionError::Input(InputError::TurnInProgress)) => {
                        Effect::NotSent(QuestionNotSent::TurnChanged)
                    }
                    Err(SessionError::Input(
                        InputError::InvalidContent(_) | InputError::ContextLimit,
                    )) => Effect::NotSent(QuestionNotSent::RejectedBeforeSend),
                    Err(_error) => Effect::Unknown,
                }
            }
        }
    }
}

fn unchanged(state: QuestionResponseState) -> QuestionDispatch {
    QuestionDispatch::Unchanged(state)
}
