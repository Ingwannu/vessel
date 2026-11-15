//! Durable async question operations use the same commit-acknowledged writer.

use super::Store;
use crate::{
    QuestionAdmission, QuestionAnswerAdmission, QuestionAnswerNotSent, QuestionAnswerTarget,
    QuestionAnswerToken, StoreError,
    writer::{TimelineWrite, Write},
};
use tokio::sync::oneshot;
use vessel_proto::{Question, QuestionAnswer, QuestionId, QuestionOutcome, SessionId, ToolCallId};

impl Store {
    /// Reserve a completed provider item's nonblocking request before publishing it.
    ///
    /// # Errors
    /// Corrupt retained state, unknown session, or database/commit failure.
    pub async fn offer_question(
        &self,
        session: SessionId,
        source: ToolCallId,
        question: Question,
    ) -> Result<QuestionAdmission, StoreError> {
        let (done, committed) = oneshot::channel();
        self.send(Write::Timeline(TimelineWrite::OfferQuestion {
            session,
            source,
            question: Box::new(question),
            done,
        }))
        .await?;
        committed.await.map_err(|_closed| StoreError::Closed)?
    }

    /// Persist a validated answer/foreground claim before any provider effect.
    ///
    /// # Errors
    /// Corrupt retained state or database/commit failure.
    pub async fn begin_question_answer(
        &self,
        session: SessionId,
        id: QuestionId,
        answer: QuestionAnswer,
        target: QuestionAnswerTarget,
    ) -> Result<QuestionAnswerAdmission, StoreError> {
        let (done, committed) = oneshot::channel();
        self.send(Write::Timeline(TimelineWrite::BeginQuestionAnswer {
            session,
            id,
            answer,
            target,
            done,
        }))
        .await?;
        committed.await.map_err(|_closed| StoreError::Closed)?
    }

    /// Settle a claim only after provider acceptance, retaining its replay tombstone.
    ///
    /// # Errors
    /// The exact claim changed or commit failed.
    pub async fn finish_question_answer(
        &self,
        token: QuestionAnswerToken,
    ) -> Result<QuestionOutcome, StoreError> {
        let (done, committed) = oneshot::channel();
        self.send(Write::Timeline(TimelineWrite::FinishQuestionAnswer {
            token,
            done,
        }))
        .await?;
        committed.await.map_err(|_closed| StoreError::Closed)?
    }

    /// Restore Pending only with a typed proof that no external effect occurred.
    ///
    /// # Errors
    /// The exact claim changed or commit failed.
    pub async fn refuse_question_answer(
        &self,
        token: QuestionAnswerToken,
        reason: QuestionAnswerNotSent,
    ) -> Result<(), StoreError> {
        self.write(|done| {
            Write::Timeline(TimelineWrite::RefuseQuestionAnswer {
                token,
                reason,
                done,
            })
        })
        .await
    }

    /// Retain an uncertain claim without making it actionable again.
    ///
    /// # Errors
    /// The exact claim changed or commit failed.
    pub async fn abandon_question_answer(
        &self,
        token: QuestionAnswerToken,
    ) -> Result<(), StoreError> {
        self.write(|done| Write::Timeline(TimelineWrite::AbandonQuestionAnswer { token, done }))
            .await
    }

    /// Read Pending/Answering/OutcomeUnknown requests, excluding settled tombstones.
    ///
    /// # Errors
    /// Corrupt retained state or database read failure.
    pub async fn pending_questions(&self, session: SessionId) -> Result<Vec<Question>, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| {
                crate::timeline_admission::question::pending(connection, session)
            })
        })
        .await
    }
}
