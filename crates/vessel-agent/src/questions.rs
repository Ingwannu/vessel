//! Questions the agent waits on (protocol 10's ACP form elicitations):
//! the first answer wins.
//!
//! [Decision Log]
//! - Purpose: the agent's `elicitation/create` stays open until a person
//!   answers, declines, the agent withdraws it, or the turn ends; many
//!   clients may answer, and only the first answer counts.
//! - Choice: the same shape as `permissions`: a node-minted
//!   [`QuestionId`], the agent's responder held here, the recently settled
//!   ids remembered so a late answer hears "already settled". The question
//!   itself is kept too: an answer is checked against its form
//!   ([`Question::check`]) before the agent sees it.
//! - Bounded: at most [`MAX_WAITING`] wait at once; a newer one withdraws
//!   the oldest (answered `cancel`), as the session's projection drops its
//!   oldest. A form is at most 32 fields, so the bound is on memory too.
//! - Withdrawal by the agent (`$/cancel_request`): the SDK marks the
//!   request's [`Responder`] cancelled; the driver watches that marker
//!   (`driver::questions`). Each question holds the sender of a channel its
//!   watcher waits on as well, so settling a question here (dropping it)
//!   also ends its watcher: no task outlives its question.

use std::convert::Infallible;

use agent_client_protocol::v1::Responder;
use agent_client_protocol::v1::schema as acp;
use tokio::sync::oneshot;
use vessel_proto::{Question, QuestionAnswer, QuestionId, QuestionOutcome};

use crate::MAX_WAITING;
use crate::error::InputError;
use crate::mapper;
use crate::recent::Recent;

/// The questions of one session.
#[derive(Debug)]
pub(crate) struct Questions {
    /// Whether the handshake advertised form elicitation: without it, the
    /// agent's questions are refused at once.
    offered: bool,
    /// In arrival order, so withdrawing resolves them in a stable order and
    /// the oldest is the first to go.
    pending: Vec<Pending>,
    /// Bounded: a long session must not keep every id it ever settled.
    settled: Recent<QuestionId>,
}

#[derive(Debug)]
struct Pending {
    question: Question,
    responder: Responder<acp::CreateElicitationResponse>,
    /// Dropped with the question: its watcher's receiver then ends.
    _watched: oneshot::Sender<Infallible>,
}

/// A question held, and what holding it displaced.
pub(crate) struct Opened {
    /// The oldest question, withdrawn (answered `cancel`) to stay within
    /// [`MAX_WAITING`].
    pub(crate) displaced: Option<QuestionId>,
    /// Ends once the question is settled, however it is: its watcher
    /// stops waiting then.
    pub(crate) settled: oneshot::Receiver<Infallible>,
}

impl Questions {
    /// No question waits yet; `offered` is whether the session takes them.
    pub(crate) fn new(offered: bool) -> Self {
        Self {
            offered,
            pending: Vec::new(),
            settled: Recent::default(),
        }
    }

    /// Whether the session takes questions.
    pub(crate) const fn offered(&self) -> bool {
        self.offered
    }

    /// Hold `question` until it is settled.
    pub(crate) fn open(
        &mut self,
        question: Question,
        responder: Responder<acp::CreateElicitationResponse>,
    ) -> Opened {
        let displaced = (self.pending.len() >= MAX_WAITING).then(|| {
            let oldest = self.pending.remove(0);
            self.settle(oldest, &QuestionOutcome::Cancelled)
        });
        let (watched, settled) = oneshot::channel();
        self.pending.push(Pending {
            question,
            responder,
            _watched: watched,
        });
        Opened { displaced, settled }
    }

    /// Answer question `id`: checked against its form, then given to the
    /// agent. Returns how it was settled.
    pub(crate) fn answer(
        &mut self,
        id: QuestionId,
        answer: QuestionAnswer,
    ) -> Result<QuestionOutcome, InputError> {
        let Some(index) = self.index(id) else {
            return Err(if self.settled.contains(&id) {
                InputError::QuestionSettled(id)
            } else {
                InputError::QuestionNotFound(id)
            });
        };
        if let Some(pending) = self.pending.get(index)
            && let Err(reason) = pending.question.check(&answer)
        {
            return Err(InputError::InvalidAnswer { id, reason });
        }
        let outcome = match answer {
            QuestionAnswer::Accept { values } => QuestionOutcome::Answered { values },
            QuestionAnswer::Decline => QuestionOutcome::Declined,
        };
        let pending = self.pending.remove(index);
        let _id = self.settle(pending, &outcome);
        Ok(outcome)
    }

    /// The agent withdrew question `id`; `true` if it was still waiting
    /// (it is answered `cancel`, as ACP has a client answer a cancelled
    /// elicitation).
    pub(crate) fn withdraw(&mut self, id: QuestionId) -> bool {
        let Some(index) = self.index(id) else {
            return false;
        };
        let pending = self.pending.remove(index);
        let _id = self.settle(pending, &QuestionOutcome::Cancelled);
        true
    }

    /// Withdraw every waiting question (answered `cancel`); returns their
    /// ids in arrival order.
    pub(crate) fn cancel_all(&mut self) -> Vec<QuestionId> {
        let pending = std::mem::take(&mut self.pending);
        pending
            .into_iter()
            .map(|pending| self.settle(pending, &QuestionOutcome::Cancelled))
            .collect()
    }

    fn index(&self, id: QuestionId) -> Option<usize> {
        self.pending
            .iter()
            .position(|pending| pending.question.id == id)
    }

    /// Give the agent `outcome` for `pending` and remember it settled.
    fn settle(&mut self, pending: Pending, outcome: &QuestionOutcome) -> QuestionId {
        let id = pending.question.id;
        let response = mapper::elicitation_response(outcome);
        // Fails only when the connection is already gone; the turn ends
        // anyway.
        if let Err(error) = pending.responder.respond(response) {
            tracing::debug!(%error, "answer to a question not delivered");
        }
        let _new = self.settled.insert(id);
        id
    }
}
