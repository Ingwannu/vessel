//! Questions (protocol 10): the agent's `elicitation/create`, asked of a
//! person, answered by the first client, withdrawn by the agent or by the
//! end of its turn.
//!
//! [Decision Log]
//! - Purpose: Claude's `AskUserQuestion`, Codex's and `OpenCode`'s questions
//!   reach a person as a typed question (`QuestionAsked`) and come back as
//!   the agent's answer (`QuestionResolved`), as permission requests do.
//! - What is refused at once, without a question: any mode Vessel did not
//!   advertise (URL mode always, which would have a person open a link the
//!   agent chose from a page on another machine; form mode when this
//!   session does not take questions), with an error the agent sees as
//!   "invalid params", MCP's answer to a mode a client did not declare. A
//!   form Vessel cannot show whole is declined (`mapper::Unfit`). Each says
//!   why in the session's log (`vessel:` lines), never with the URL, which
//!   can carry a sign-in secret.
//! - The agent withdrawing a question (`$/cancel_request`, which Claude's
//!   adapter sends when a hook answered or a subagent closed): the SDK sets
//!   the request's cancellation marker; a task per question waits on it,
//!   or on the question being settled, whichever comes first, and reports
//!   into `inbound`. So the withdrawal is handled by the loop, in order
//!   with everything else, and a question answered meanwhile is not
//!   reported twice.
//! - Alternatives: watching `$/cancel_request` notifications by request id
//!   (the SDK hands markers to responders already, and ids are its own);
//!   polling `is_cancelled` (a loop on a timer, against AGENTS.md).
//! - Cost: one small task per waiting question (at most
//!   [`MAX_WAITING`](crate::MAX_WAITING)), ended when it is settled.

use agent_client_protocol::v1::schema as acp;
use agent_client_protocol::v1::{ConnectionTo, Responder};
use agent_client_protocol::{Agent, Error, ErrorCode};
use vessel_proto::{AgentEvent, QuestionAnswer, QuestionId, QuestionOutcome};

use super::{Driver, Inbound, forward};
use crate::error::InputError;
use crate::mapper::{self, Refusal};
use crate::questions::Opened;

impl Driver {
    /// Ask a person what the agent asks, or refuse it at once.
    pub(super) async fn question(
        &mut self,
        cx: &ConnectionTo<Agent>,
        request: acp::CreateElicitationRequest,
        responder: Responder<acp::CreateElicitationResponse>,
    ) {
        let asked = mapper::question(QuestionId::random(), request);
        let question = match asked {
            Err(Refusal::NotAForm { mode }) => {
                self.refuse(responder, &mode, "Vessel offers only forms")
                    .await;
                return;
            }
            // A form, shown or not: this session did not advertise forms.
            Ok(_) | Err(Refusal::Unfit(_)) if !self.questions.offered() => {
                self.refuse(responder, "form", "this session does not take questions")
                    .await;
                return;
            }
            Ok(question) => question,
            Err(Refusal::Unfit(why)) => {
                let response = mapper::elicitation_response(&QuestionOutcome::Declined);
                if let Err(error) = responder.respond(response) {
                    tracing::debug!(%error, "declining a question failed");
                }
                tracing::warn!(%why, "a question Vessel cannot show was declined");
                self.log(format!(
                    "vessel: the agent asked a question Vessel cannot show ({why}); declined"
                ))
                .await;
                return;
            }
        };
        let id = question.id;
        let cancellation = responder.cancellation();
        let Opened { displaced, settled } = self.questions.open(question.clone(), responder);
        if let Some(oldest) = displaced {
            self.resolved(oldest, QuestionOutcome::Cancelled).await;
        }
        self.emit(AgentEvent::QuestionAsked(question)).await;
        let inbound = self.inbound.clone();
        let watch = async move {
            tokio::select! {
                () = cancellation.cancelled() => {
                    forward(&inbound, Inbound::Withdrawn(id)).await;
                }
                // Settled first (answered, declined or withdrawn here): the
                // sender went with the question.
                _settled = settled => {}
            }
            Ok(())
        };
        if let Err(error) = cx.spawn(watch) {
            // The connection is closing; what waits is settled with it.
            tracing::debug!(%error, "a question's withdrawal is not watched");
        }
        if self.cancelling {
            // Asked before the agent saw the turn's cancel: ACP has the
            // client answer it `cancel`, and the turn cannot end until it
            // is. The log shows it asked and withdrawn.
            self.withdraw_questions().await;
        }
    }

    /// A person's answer to question `id`.
    pub(super) async fn answer_question(
        &mut self,
        id: QuestionId,
        answer: QuestionAnswer,
    ) -> Result<(), InputError> {
        let outcome = self.questions.answer(id, answer)?;
        self.resolved(id, outcome).await;
        Ok(())
    }

    /// The agent withdrew question `id` (if it still waits).
    pub(super) async fn withdrawn(&mut self, id: QuestionId) {
        if self.questions.withdraw(id) {
            self.resolved(id, QuestionOutcome::Cancelled).await;
        }
    }

    /// Withdraw every question still waiting (the turn was cancelled or
    /// ended, or the connection closed).
    pub(super) async fn withdraw_questions(&mut self) {
        for id in self.questions.cancel_all() {
            self.resolved(id, QuestionOutcome::Cancelled).await;
        }
    }

    async fn resolved(&mut self, id: QuestionId, outcome: QuestionOutcome) {
        self.emit(AgentEvent::QuestionResolved { id, outcome })
            .await;
    }

    /// Refuse an elicitation in a mode Vessel did not advertise.
    async fn refuse(
        &mut self,
        responder: Responder<acp::CreateElicitationResponse>,
        mode: &str,
        why: &str,
    ) {
        let error = Error::new(
            ErrorCode::INVALID_PARAMS,
            format!("the client did not advertise {mode} elicitation: {why}"),
        );
        if let Err(error) = responder.respond_with_error(error) {
            tracing::debug!(%error, "refusing a question failed");
        }
        tracing::warn!(
            mode,
            "an elicitation in a mode Vessel did not advertise was refused"
        );
        self.log(format!(
            "vessel: the agent asked for input in a way Vessel does not offer ({mode} elicitation; {why}); refused"
        ))
        .await;
    }

    /// A `vessel:` line in the session's log.
    async fn log(&mut self, line: String) {
        self.emit(AgentEvent::HarnessLog { line }).await;
    }
}
