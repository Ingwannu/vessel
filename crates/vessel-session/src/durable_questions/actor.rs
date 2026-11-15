//! The actor publishes actionable async requests only after node-owned admission.

use super::QuestionOffer;
use crate::structured::StructuredActor;
use vessel_proto::{AgentEvent, Question, SessionEvent};

impl StructuredActor {
    pub(crate) async fn offer_durable_question(&mut self, question: Question) {
        let Some(owner) = self.questions.clone() else {
            self.question_unavailable().await;
            return;
        };
        let Some(source) = question.tool_call.clone() else {
            self.question_unavailable().await;
            return;
        };
        match owner.offer(self.life.out.session(), source, question).await {
            Ok(QuestionOffer::Available(question)) => self.publish_question(question).await,
            Ok(QuestionOffer::Settled) => {}
            Ok(QuestionOffer::Refused) | Err(_) => self.question_unavailable().await,
        }
    }

    pub(crate) async fn restore_durable_questions(&mut self) {
        let Some(owner) = self.questions.clone() else {
            return;
        };
        match owner.pending(self.life.out.session()).await {
            Ok(questions) => {
                for question in questions {
                    self.publish_question(question).await;
                }
            }
            Err(_) => self.question_unavailable().await,
        }
    }

    pub(super) async fn publish_question(&mut self, question: Question) {
        self.life
            .publish(
                &mut self.agent,
                SessionEvent::Agent {
                    event: AgentEvent::QuestionAsked(question),
                },
            )
            .await;
    }

    async fn question_unavailable(&mut self) {
        self.life
            .publish(
                &mut self.agent,
                SessionEvent::Agent {
                    event: AgentEvent::Error {
                        message: "A nonblocking question could not be retained safely".into(),
                    },
                },
            )
            .await;
    }
}
