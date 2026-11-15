//! Retained admission writes and acknowledgements, owned by the existing writer.

use super::apply::isolated;
use super::{Done, after};
use crate::StoreError;
use rusqlite::Transaction;
use std::sync::Arc;
use vessel_proto::SessionId;

#[derive(Debug)]
pub(crate) enum TimelineWrite {
    OfferQuestion {
        session: SessionId,
        source: vessel_proto::ToolCallId,
        question: Box<vessel_proto::Question>,
        done: Done<crate::QuestionAdmission>,
    },
    BeginQuestionAnswer {
        session: SessionId,
        id: vessel_proto::QuestionId,
        answer: vessel_proto::QuestionAnswer,
        target: crate::QuestionAnswerTarget,
        done: Done<crate::QuestionAnswerAdmission>,
    },
    FinishQuestionAnswer {
        token: crate::QuestionAnswerToken,
        done: Done<vessel_proto::QuestionOutcome>,
    },
    RefuseQuestionAnswer {
        token: crate::QuestionAnswerToken,
        reason: crate::QuestionAnswerNotSent,
        done: Done,
    },
    AbandonQuestionAnswer {
        token: crate::QuestionAnswerToken,
        done: Done,
    },
    AdmitPrompt {
        request: Box<vessel_proto::timeline::SubmitPrompt>,
        creator: vessel_proto::Creator,
        done: Done<crate::ReceiptAdmission>,
    },
    RefusePrompt {
        token: crate::ReceiptToken,
        reason: vessel_proto::timeline::SubmissionRefusal,
        done: Done<vessel_proto::timeline::SubmitPromptReply>,
    },
    AcceptPrompt {
        token: crate::ReceiptToken,
        accepted: vessel_proto::timeline::SubmissionAcceptedKind,
        done: Done<vessel_proto::timeline::PromptReceipt>,
    },
    BindPrompt {
        token: crate::ReceiptBindingToken,
        target: vessel_proto::timeline::PromptReceiptBinding,
        done: Done,
    },
}

pub(super) enum TimelineAnswer {
    QuestionOffered(
        Done<crate::QuestionAdmission>,
        Result<crate::QuestionAdmission, StoreError>,
    ),
    QuestionClaimed(
        Done<crate::QuestionAnswerAdmission>,
        Result<crate::QuestionAnswerAdmission, StoreError>,
    ),
    QuestionFinished(
        Done<vessel_proto::QuestionOutcome>,
        Result<vessel_proto::QuestionOutcome, StoreError>,
    ),
    PromptAdmitted(
        Done<crate::ReceiptAdmission>,
        Result<crate::ReceiptAdmission, StoreError>,
    ),
    PromptRefused(
        Done<vessel_proto::timeline::SubmitPromptReply>,
        Result<vessel_proto::timeline::SubmitPromptReply, StoreError>,
    ),
    PromptAccepted(
        Done<vessel_proto::timeline::PromptReceipt>,
        Result<vessel_proto::timeline::PromptReceipt, StoreError>,
    ),
    Done(Done, Result<(), StoreError>),
}

impl TimelineAnswer {
    pub(super) fn send(self, failed: Option<&Arc<rusqlite::Error>>) -> bool {
        match self {
            Self::QuestionOffered(done, result) => done.send(after(failed, result)).is_ok(),
            Self::QuestionClaimed(done, result) => done.send(after(failed, result)).is_ok(),
            Self::QuestionFinished(done, result) => done.send(after(failed, result)).is_ok(),
            Self::PromptAdmitted(done, result) => done.send(after(failed, result)).is_ok(),
            Self::PromptRefused(done, result) => done.send(after(failed, result)).is_ok(),
            Self::PromptAccepted(done, result) => done.send(after(failed, result)).is_ok(),
            Self::Done(done, result) => done.send(after(failed, result)).is_ok(),
        }
    }
}

pub(super) fn apply(transaction: &mut Transaction<'_>, write: TimelineWrite) -> TimelineAnswer {
    match write {
        TimelineWrite::BindPrompt {
            token,
            target,
            done,
        } => TimelineAnswer::Done(
            done,
            isolated(transaction, |connection| {
                crate::timeline_admission::binding::record(connection, token, target)
            }),
        ),
        TimelineWrite::OfferQuestion {
            session,
            source,
            question,
            done,
        } => TimelineAnswer::QuestionOffered(
            done,
            isolated(transaction, |connection| {
                crate::timeline_admission::question::admit_question(
                    connection, session, source, *question,
                )
            }),
        ),
        TimelineWrite::BeginQuestionAnswer {
            session,
            id,
            answer,
            target,
            done,
        } => TimelineAnswer::QuestionClaimed(
            done,
            isolated(transaction, |connection| {
                crate::timeline_admission::question::begin_answer(
                    connection, session, id, answer, target,
                )
            }),
        ),
        TimelineWrite::FinishQuestionAnswer { token, done } => TimelineAnswer::QuestionFinished(
            done,
            isolated(transaction, |connection| {
                crate::timeline_admission::question::finish_answer(connection, token)
            }),
        ),
        TimelineWrite::RefuseQuestionAnswer {
            token,
            reason,
            done,
        } => TimelineAnswer::Done(
            done,
            isolated(transaction, |connection| {
                crate::timeline_admission::question::reject_before_effect(connection, token, reason)
            }),
        ),
        TimelineWrite::AbandonQuestionAnswer { token, done } => TimelineAnswer::Done(
            done,
            isolated(transaction, |connection| {
                crate::timeline_admission::question::abandon_answer(connection, token)
            }),
        ),
        TimelineWrite::AdmitPrompt {
            request,
            creator,
            done,
        } => TimelineAnswer::PromptAdmitted(
            done,
            isolated(transaction, |connection| {
                crate::timeline_admission::admit_input(connection, &request, &creator)
            }),
        ),
        TimelineWrite::RefusePrompt {
            token,
            reason,
            done,
        } => TimelineAnswer::PromptRefused(
            done,
            isolated(transaction, |connection| {
                crate::timeline_admission::refuse_input(connection, token, reason)
            }),
        ),
        TimelineWrite::AcceptPrompt {
            token,
            accepted,
            done,
        } => TimelineAnswer::PromptAccepted(
            done,
            isolated(transaction, |connection| {
                crate::timeline_admission::accept_input(connection, token, accepted)
            }),
        ),
    }
}
