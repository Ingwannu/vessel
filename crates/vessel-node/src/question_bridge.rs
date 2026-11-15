//! The node owns retained async requests; the session actor owns foreground dispatch.
//! This callback avoids a session-to-store dependency or a second mutable question owner.

use vessel_proto::timeline::QuestionDeliveryState;
use vessel_proto::{Question, QuestionAnswer, QuestionId, SessionId, ToolCallId};
use vessel_session::durable_questions::{
    PreparedQuestionAnswer, QuestionAnswerLease, QuestionClaim, QuestionFuture, QuestionNotSent,
    QuestionOffer, QuestionPersistence, QuestionPersistenceError, QuestionTarget,
};
use vessel_store::{
    QuestionAdmission, QuestionAnswerAdmission, QuestionAnswerNotSent, QuestionAnswerTarget,
    QuestionAnswerToken, Store, StoreError,
};

#[derive(Debug)]
pub(crate) struct Questions(Store);

impl Questions {
    pub(crate) const fn new(store: Store) -> Self {
        Self(store)
    }
}

fn unavailable(error: StoreError) -> QuestionPersistenceError {
    tracing::warn!(%error, "durable question operation refused");
    QuestionPersistenceError::Unavailable
}

impl QuestionPersistence for Questions {
    fn offer(
        &self,
        session: SessionId,
        source: ToolCallId,
        question: Question,
    ) -> QuestionFuture<QuestionOffer> {
        let store = self.0.clone();
        Box::pin(async move {
            Ok(
                match store
                    .offer_question(session, source, question)
                    .await
                    .map_err(unavailable)?
                {
                    QuestionAdmission::New(question) | QuestionAdmission::Existing(question) => {
                        QuestionOffer::Available(question)
                    }
                    QuestionAdmission::Settled => QuestionOffer::Settled,
                    QuestionAdmission::Conflict
                    | QuestionAdmission::Capacity(_)
                    | QuestionAdmission::TooLarge
                    | QuestionAdmission::InvalidForm(_) => QuestionOffer::Refused,
                },
            )
        })
    }

    fn claim(
        &self,
        session: SessionId,
        id: QuestionId,
        answer: QuestionAnswer,
        target: QuestionTarget,
    ) -> QuestionFuture<QuestionClaim> {
        let store = self.0.clone();
        Box::pin(async move {
            let stored_target = match target {
                QuestionTarget::Idle => QuestionAnswerTarget::Idle,
                QuestionTarget::Turn { started } => QuestionAnswerTarget::Turn { started },
            };
            Ok(
                match store
                    .begin_question_answer(session, id, answer, stored_target)
                    .await
                    .map_err(unavailable)?
                {
                    QuestionAnswerAdmission::New(token) => {
                        QuestionClaim::Ready(PreparedQuestionAnswer {
                            question: token.question().clone(),
                            answer: token.answer().map_err(unavailable)?.clone(),
                            target,
                            lease: Box::new(Lease { store, token }),
                        })
                    }
                    QuestionAnswerAdmission::AlreadyClaimed(QuestionDeliveryState::Answering) => {
                        QuestionClaim::Answering
                    }
                    QuestionAnswerAdmission::AlreadyClaimed(
                        QuestionDeliveryState::OutcomeUnknown,
                    ) => QuestionClaim::OutcomeUnknown,
                    QuestionAnswerAdmission::AlreadyClaimed(QuestionDeliveryState::Pending) => {
                        return Err(QuestionPersistenceError::Unavailable);
                    }
                    QuestionAnswerAdmission::Settled => QuestionClaim::Settled,
                    QuestionAnswerAdmission::Missing => QuestionClaim::Missing,
                    QuestionAnswerAdmission::Invalid(_) => QuestionClaim::Invalid,
                    QuestionAnswerAdmission::TooLarge => QuestionClaim::TooLarge,
                },
            )
        })
    }

    fn pending(&self, session: SessionId) -> QuestionFuture<Vec<Question>> {
        let store = self.0.clone();
        Box::pin(async move { store.pending_questions(session).await.map_err(unavailable) })
    }
}

#[derive(Debug)]
struct Lease {
    store: Store,
    token: QuestionAnswerToken,
}

impl QuestionAnswerLease for Lease {
    fn accepted(self: Box<Self>) -> QuestionFuture<vessel_proto::QuestionOutcome> {
        let Self { store, token } = *self;
        Box::pin(async move {
            store
                .finish_question_answer(token)
                .await
                .map_err(unavailable)
        })
    }
    fn not_sent(self: Box<Self>, reason: QuestionNotSent) -> QuestionFuture<()> {
        let Self { store, token } = *self;
        let reason = match reason {
            QuestionNotSent::TurnChanged => QuestionAnswerNotSent::TurnChanged,
            QuestionNotSent::Unsupported => QuestionAnswerNotSent::Unsupported,
            QuestionNotSent::RejectedBeforeSend => QuestionAnswerNotSent::RejectedBeforeSend,
        };
        Box::pin(async move {
            store
                .refuse_question_answer(token, reason)
                .await
                .map_err(unavailable)
        })
    }
    fn uncertain(self: Box<Self>) -> QuestionFuture<()> {
        let Self { store, token } = *self;
        Box::pin(async move {
            store
                .abandon_question_answer(token)
                .await
                .map_err(unavailable)
        })
    }
}
