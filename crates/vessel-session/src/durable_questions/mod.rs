//! Async request persistence is supplied by the node; this crate never depends on a store.
//! The actor owns dispatch order. A retained lease permits exactly one answer effect.

mod actor;
pub(crate) mod answer;
mod prompt;
pub use answer::{QuestionDispatch, QuestionResponseState};

use std::{fmt::Debug, future::Future, pin::Pin};
use vessel_proto::{
    EventSeq, Question, QuestionAnswer, QuestionId, QuestionOutcome, SessionId, ToolCallId,
};

/// Node-owned persistence never borrows actor state across an await.
pub type QuestionFuture<T> =
    Pin<Box<dyn Future<Output = Result<T, QuestionPersistenceError>> + Send>>;

/// Persistence refusal never means provider acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum QuestionPersistenceError {
    /// The retained-state owner could not read/commit safely.
    #[error("durable question persistence unavailable")]
    Unavailable,
}

/// Exact actor foreground state captured before durable answer admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionTarget {
    /// Only this foreground turn may receive the answer.
    Turn {
        /// Its canonical start record.
        started: EventSeq,
    },
    /// An accepted answer can start one follow-up while the actor remains idle.
    Idle,
}

/// Only a known lack of external effects permits another answer attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionNotSent {
    /// Foreground changed before dispatch.
    TurnChanged,
    /// Provider lacks the required exact-turn delivery capability.
    Unsupported,
    /// Input validation refused before external work.
    RejectedBeforeSend,
}

/// A completed provider item is not actionable until its retained admission commits.
#[derive(Debug)]
pub enum QuestionOffer {
    /// New or replayed request with authoritative delivery state.
    Available(Question),
    /// Already settled; replay protection remains retained.
    Settled,
    /// Capacity, source conflict or malformed metadata refused admission.
    Refused,
}

/// A fresh admitted answer owns a lease; other states must never dispatch.
#[derive(Debug)]
pub enum QuestionClaim {
    /// Exact stored form and answer, with an opaque one-effect lease.
    Ready(PreparedQuestionAnswer),
    /// One admitted answer is already in flight.
    Answering,
    /// An effect may have happened; automatic resend is forbidden.
    OutcomeUnknown,
    /// A prior response settled the request.
    Settled,
    /// No retained request has this identity.
    Missing,
    /// The answer does not fit the authoritative form.
    Invalid,
    /// The answer cannot fit the pre-reserved result envelope.
    TooLarge,
}

/// The provider dispatches only this exact admitted answer and captured target.
#[derive(Debug)]
pub struct PreparedQuestionAnswer {
    /// Authoritative form, including generic custom companions.
    pub question: Question,
    /// Validated answer retained before dispatch.
    pub answer: QuestionAnswer,
    /// Foreground bound at admission.
    pub target: QuestionTarget,
    /// Consuming completion/refusal cannot be forged by another actor request.
    pub lease: Box<dyn QuestionAnswerLease>,
}

/// A node-owned opaque claim. Dropping it never releases replay protection.
pub trait QuestionAnswerLease: Send + Debug {
    /// Commit a real observed acceptance before publishing its resolution.
    fn accepted(self: Box<Self>) -> QuestionFuture<QuestionOutcome>;
    /// Restore Pending only after a typed known-not-sent result.
    fn not_sent(self: Box<Self>, reason: QuestionNotSent) -> QuestionFuture<()>;
    /// Preserve uncertainty after a possibly executed effect or lost acknowledgement.
    fn uncertain(self: Box<Self>) -> QuestionFuture<()>;
}

/// One retained-state owner for every session. Implemented by the node's store bridge.
pub trait QuestionPersistence: Send + Sync + Debug {
    /// Reserve before creating an actionable request, deduplicated by stable provider item.
    fn offer(
        &self,
        session: SessionId,
        source: ToolCallId,
        question: Question,
    ) -> QuestionFuture<QuestionOffer>;
    /// Validate and commit an exact answer/foreground claim before dispatch.
    fn claim(
        &self,
        session: SessionId,
        id: QuestionId,
        answer: QuestionAnswer,
        target: QuestionTarget,
    ) -> QuestionFuture<QuestionClaim>;
    /// Restore Pending/Answering/OutcomeUnknown requests independently of provider history.
    fn pending(&self, session: SessionId) -> QuestionFuture<Vec<Question>>;
}
