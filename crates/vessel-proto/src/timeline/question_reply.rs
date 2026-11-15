//! Question answers acknowledge exact durable resolution instead of inferring tool completion.

use crate::{EventSeq, QuestionAnswer, QuestionId, QuestionOutcome, SessionId};
use serde::{Deserialize, Serialize};

/// An exact typed answer, for blocking or durable session-lifetime requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RespondQuestion {
    /// Authorized owning session.
    pub session: SessionId,
    /// Canonical request identity.
    pub question: QuestionId,
    /// Existing authoritative field-keyed answer shape.
    pub answer: QuestionAnswer,
}

/// A refusal known to precede any answer effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum QuestionResponseRefusal {
    /// The typed values do not fit the stored form.
    InvalidAnswer,
    /// The answer exceeds its retained-state envelope.
    TooLarge,
    /// Provider cannot take the answer in this foreground state.
    Unsupported,
    /// The captured foreground turn changed before any effect.
    TurnChanged,
    /// Persistence or a live owning session is unavailable; the request is retained.
    Unavailable,
}

/// The request remains non-actionable after a possibly executed answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RespondQuestionReply {
    /// The exact accepted outcome and resolution record are committed.
    Settled {
        /// Resolved request identity.
        question: QuestionId,
        /// Actual accepted field values or decline.
        outcome: QuestionOutcome,
        /// Committed resolution record.
        seq: EventSeq,
    },
    /// Request is absent from both durable and live owners.
    Vanished {
        /// Request identity.
        question: QuestionId,
    },
    /// Another response or withdrawal already settled it.
    Conflict {
        /// Request identity.
        question: QuestionId,
    },
    /// One retained answer claim is in flight; another answer cannot dispatch.
    Answering {
        /// Request identity.
        question: QuestionId,
    },
    /// An effect may have happened; automatic resend is forbidden.
    OutcomeUnknown {
        /// Request identity.
        question: QuestionId,
    },
    /// No answer effect was attempted; corrected input may be submitted explicitly.
    Refused {
        /// Retained/live request left unchanged.
        question: QuestionId,
        /// Exact refusal category.
        reason: QuestionResponseRefusal,
    },
}
