//! Both blocking and durable question responses await the exact committed resolution.

use super::{Found, Sessions};
use crate::{error::ServiceError, persist};
use std::time::Duration;
use vessel_proto::timeline::{QuestionResponseRefusal, RespondQuestion, RespondQuestionReply};
use vessel_proto::{AgentEvent, EventSeq, QuestionId, QuestionOutcome, SessionEvent, SessionId};
use vessel_session::durable_questions::{QuestionDispatch, QuestionResponseState};

/// Bound a lost answer acknowledgement without inventing successful resolution.
const QUESTION_ACK_TIMEOUT: Duration = Duration::from_secs(30);
/// Catch up with bounded pages, never a private whole-log copy.
const QUESTION_ACK_PAGE: u32 = 128;

impl Sessions {
    pub(crate) async fn respond_question(
        &self,
        request: RespondQuestion,
    ) -> Result<RespondQuestionReply, ServiceError> {
        let question = request.question;
        match tokio::time::timeout(QUESTION_ACK_TIMEOUT, self.respond_question_inner(request)).await
        {
            Ok(reply) => reply,
            Err(_elapsed) => Ok(RespondQuestionReply::OutcomeUnknown { question }),
        }
    }

    async fn respond_question_inner(
        &self,
        request: RespondQuestion,
    ) -> Result<RespondQuestionReply, ServiceError> {
        let question = request.question;
        let Found::Live {
            handle,
            mut durability,
        } = self.find(request.session).await?
        else {
            let retained = self
                .store
                .pending_questions(request.session)
                .await?
                .into_iter()
                .find(|pending| pending.id == question);
            return Ok(match retained.map(|question| question.delivery_state) {
                Some(vessel_proto::timeline::QuestionDeliveryState::Answering) => {
                    RespondQuestionReply::Answering { question }
                }
                Some(vessel_proto::timeline::QuestionDeliveryState::OutcomeUnknown) => {
                    RespondQuestionReply::OutcomeUnknown { question }
                }
                Some(vessel_proto::timeline::QuestionDeliveryState::Pending) => {
                    RespondQuestionReply::Refused {
                        question,
                        reason: QuestionResponseRefusal::Unavailable,
                    }
                }
                None => RespondQuestionReply::Vanished { question },
            });
        };
        let from = handle.info().next_seq;
        let outcome = match handle.respond_question(question, request.answer).await? {
            QuestionDispatch::Applied(outcome) => outcome,
            QuestionDispatch::Unchanged(state) => return Ok(unchanged(question, state)),
        };
        let resolution = tokio::time::timeout(QUESTION_ACK_TIMEOUT, async {
            let Some((seq, resolved)) = self
                .question_resolution(request.session, question, from)
                .await?
            else {
                return Ok(None);
            };
            let Some(next) = seq.next() else {
                return Ok(None);
            };
            persist::wait_until(request.session, &mut durability, next).await?;
            Ok::<_, ServiceError>(Some((seq, resolved)))
        })
        .await;
        match resolution {
            Ok(Ok(Some((seq, resolved)))) if resolved == outcome => {
                Ok(RespondQuestionReply::Settled {
                    question,
                    outcome,
                    seq,
                })
            }
            Ok(Ok(Some(_))) => Ok(RespondQuestionReply::Conflict { question }),
            Ok(Ok(None)) | Err(_) => Ok(RespondQuestionReply::OutcomeUnknown { question }),
            Ok(Err(error)) => Err(error),
        }
    }

    async fn question_resolution(
        &self,
        session: SessionId,
        question: QuestionId,
        mut cursor: EventSeq,
    ) -> Result<Option<(EventSeq, QuestionOutcome)>, ServiceError> {
        loop {
            let page = self
                .read_waiting(session, cursor, QUESTION_ACK_PAGE, QUESTION_ACK_TIMEOUT)
                .await?;
            for record in page.records {
                if let SessionEvent::Agent {
                    event: AgentEvent::QuestionResolved { id, outcome },
                } = record.event
                    && id == question
                {
                    return Ok(Some((record.seq, outcome)));
                }
            }
            if page.next == cursor && page.at_end {
                return Ok(None);
            }
            cursor = page.next;
        }
    }
}

fn unchanged(question: QuestionId, state: QuestionResponseState) -> RespondQuestionReply {
    match state {
        QuestionResponseState::Vanished => RespondQuestionReply::Vanished { question },
        QuestionResponseState::Conflict => RespondQuestionReply::Conflict { question },
        QuestionResponseState::Answering => RespondQuestionReply::Answering { question },
        QuestionResponseState::OutcomeUnknown => RespondQuestionReply::OutcomeUnknown { question },
        QuestionResponseState::Refused(reason) => {
            RespondQuestionReply::Refused { question, reason }
        }
    }
}
