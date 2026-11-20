//! The session's side of what the agent asks: a tool request published as
//! a permission request, a plan's approval or a question; a person's answer
//! sent back; and withdrawals, each settled in the session's events.
//!
//! Every request reaching here is answered exactly once (ADR 0057's
//! floor): by the person, by a withdrawal (the turn cancelled or ended, the
//! connection gone, the bound passed), or at once with a denial when it
//! cannot be shown, saying why.

use vessel_proto::{
    AgentEvent, PermissionId, PermissionOutcome, QuestionAnswer, QuestionId, QuestionOutcome,
};

use super::super::asks::{self, Asked, Opened, Settled, Shown};
use super::super::wire::{CanUseTool, HostAnswer, Outgoing};
use super::Session;
use crate::error::InputError;

/// Claude Code's question tool.
const QUESTION_TOOL: &str = "AskUserQuestion";
/// Claude Code's plan approval tool.
const PLAN_TOOL: &str = "ExitPlanMode";

impl Session {
    /// A tool request (`can_use_tool` `request_id`): its tool call shown if
    /// the agent has not shown it yet, then asked of a person.
    pub(super) async fn ask(&mut self, request_id: String, asked: CanUseTool) {
        if let Some(call) =
            self.translator
                .announce(&asked.tool_use_id, &asked.tool_name, &asked.input)
        {
            self.emit(call).await;
        }
        let opened = match asked.tool_name.as_str() {
            QUESTION_TOOL if !self.questions => {
                let why = "Nobody can answer a question in this session, so Vessel declined it.";
                self.unshown(request_id, &asked, why).await;
                return;
            }
            QUESTION_TOOL => match self.asks.question(request_id.clone(), &asked) {
                Ok(opened) => opened,
                Err(why) => {
                    let why = format!("Vessel could not show this question: {why}.");
                    self.unshown(request_id, &asked, &why).await;
                    return;
                }
            },
            PLAN_TOOL => self.asks.plan(request_id, &asked),
            _ => self.asks.tool(request_id, &asked),
        };
        self.opened(opened).await;
    }

    async fn opened(&mut self, opened: Opened) {
        let Opened { shown, displaced } = opened;
        if let Some((asked, settled)) = displaced {
            self.settle(asked, settled).await;
        }
        let event = match shown {
            Shown::Permission(request) => AgentEvent::PermissionRequested(request),
            Shown::Question(question) => AgentEvent::QuestionAsked(question),
        };
        self.emit(event).await;
    }

    /// A request that cannot be shown: denied at once, saying `why`, with a
    /// line in the session's log.
    async fn unshown(&mut self, request_id: String, asked: &CanUseTool, why: &str) {
        let settled = Settled {
            request_id,
            permission: asks::deny(why, &asked.tool_use_id),
        };
        self.respond(settled).await;
        tracing::info!(tool = %asked.tool_name, why, "Claude Code's request declined unasked");
        self.emit(AgentEvent::HarnessLog {
            line: format!(
                "Vessel declined Claude Code's request to use {}: {why}",
                asked.tool_name
            ),
        })
        .await;
    }

    /// A person's answer to permission request `id`.
    pub(super) async fn answer_permission(
        &mut self,
        id: PermissionId,
        outcome: PermissionOutcome,
    ) -> Result<(), InputError> {
        let settled = self.asks.answer(id, &outcome)?;
        self.respond(settled).await;
        self.emit(AgentEvent::PermissionResolved { id, outcome })
            .await;
        Ok(())
    }

    /// A person's answer to question `id`.
    pub(super) async fn answer_question(
        &mut self,
        id: QuestionId,
        answer: QuestionAnswer,
    ) -> Result<(), InputError> {
        let (outcome, settled) = self.asks.reply(id, answer)?;
        self.respond(settled).await;
        self.emit(AgentEvent::QuestionResolved { id, outcome })
            .await;
        Ok(())
    }

    /// The agent withdrew its request `request_id`: settled as cancelled,
    /// with no answer (the agent no longer waits for one).
    pub(super) async fn agent_withdrew(&mut self, request_id: &str) {
        if let Some(asked) = self.asks.withdrawn(request_id) {
            self.resolved_cancelled(asked).await;
        }
    }

    /// Withdraw everything still asked: each denied to the agent and
    /// settled as cancelled.
    pub(super) async fn withdraw_all(&mut self) {
        for (asked, settled) in self.asks.withdraw_all() {
            self.settle(asked, settled).await;
        }
    }

    async fn settle(&mut self, asked: Asked, settled: Settled) {
        self.respond(settled).await;
        self.resolved_cancelled(asked).await;
    }

    async fn resolved_cancelled(&self, asked: Asked) {
        let event = match asked {
            Asked::Permission(id) => AgentEvent::PermissionResolved {
                id,
                outcome: PermissionOutcome::Cancelled,
            },
            Asked::Question(id) => AgentEvent::QuestionResolved {
                id,
                outcome: QuestionOutcome::Cancelled,
            },
        };
        self.emit(event).await;
    }

    /// Answer the agent's request as `settled` says. An answer that does
    /// not write as JSON is sent as a refusal, so the request is still
    /// answered.
    async fn respond(&mut self, settled: Settled) {
        let Settled {
            request_id,
            permission,
        } = settled;
        let answer = match serde_json::to_value(permission) {
            Ok(response) => HostAnswer::Success {
                request_id,
                response,
            },
            Err(error) => HostAnswer::Error {
                request_id,
                error: error.to_string(),
            },
        };
        let _sent = self
            .out
            .send(&Outgoing::ControlResponse { response: answer })
            .await;
    }
}
