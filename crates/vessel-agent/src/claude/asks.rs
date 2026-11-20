//! What Claude Code asks a person: its tool requests (`can_use_tool`) as
//! permission requests, a plan's approval as one, `AskUserQuestion` as a
//! question; each held under a node-minted id until a client answers, the
//! agent withdraws it, the turn ends or the connection does (the plan's
//! Claude Code slice 2).
//!
//! [Decision Log]
//! - Purpose: the person decides, with the choices Claude Code itself
//!   offers (`choices`), as Paseo shows them; the session sees the same
//!   permission requests and questions as from every other agent.
//! - Every request is answered exactly once (ADR 0057's floor): by the
//!   person's choice, or with a denial saying it was withdrawn when the
//!   turn is cancelled, the turn or the connection ends first, or more
//!   than [`MAX_WAITING`] are open (the oldest gives way). A request the
//!   agent withdraws itself (`control_cancel_request`) is settled here
//!   without an answer, since the agent no longer waits for one.
//! - Why not Codex's `Approvals` and `Questions`: they hold Codex's request
//!   values and the JSON each answer becomes; these hold Claude Code's
//!   request ids and typed answers (`Permission`). The ids, the bound and
//!   the "already settled" memory are the same.

mod choices;
mod question;

use vessel_proto::{
    PermissionId, PermissionOptionId, PermissionOutcome, PermissionRequest, Question,
    QuestionAnswer, QuestionId, QuestionOutcome, ToolCallId,
};

use super::wire::{CanUseTool, Permission};
use crate::MAX_WAITING;
use crate::error::InputError;
use crate::recent::Recent;

/// The denial for `tool_use_id`, saying `message`, for a request answered
/// without asking anyone.
pub(super) use choices::deny;

/// An answer to send: the agent's request `request_id`, answered with
/// `permission`.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Settled {
    pub(super) request_id: String,
    pub(super) permission: Permission,
}

/// Something asked, by the id the session's events name it with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Asked {
    Permission(PermissionId),
    Question(QuestionId),
}

/// What to publish for a request held.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Shown {
    Permission(PermissionRequest),
    Question(Question),
}

/// A request held: what to publish, and an older one withdrawn to stay
/// within [`MAX_WAITING`].
#[derive(Debug)]
pub(super) struct Opened {
    pub(super) shown: Shown,
    pub(super) displaced: Option<(Asked, Settled)>,
}

#[derive(Debug)]
enum Held {
    /// The permission request's id, and each option's answer.
    Permission {
        id: PermissionId,
        choices: Vec<(PermissionOptionId, Permission)>,
    },
    Question(question::Form),
}

#[derive(Debug)]
struct Pending {
    request_id: String,
    tool_use_id: String,
    held: Held,
}

impl Pending {
    fn asked(&self) -> Asked {
        match &self.held {
            Held::Permission { id, .. } => Asked::Permission(*id),
            Held::Question(form) => Asked::Question(form.question.id),
        }
    }

    /// Its withdrawal: the denial the agent is sent.
    fn withdrawn(self) -> (Asked, Settled) {
        let asked = self.asked();
        let settled = Settled {
            permission: choices::deny(choices::WITHDRAWN, &self.tool_use_id),
            request_id: self.request_id,
        };
        (asked, settled)
    }
}

/// The requests waiting for a person.
#[derive(Debug, Default)]
pub(super) struct Asks {
    pending: Vec<Pending>,
    permissions: Recent<PermissionId>,
    questions: Recent<QuestionId>,
}

impl Asks {
    /// A tool request, asked with its choices.
    pub(super) fn tool(&mut self, request_id: String, asked: &CanUseTool) -> Opened {
        let offered = choices::tool(
            &asked.input,
            &asked.permission_suggestions,
            &asked.tool_use_id,
        );
        self.permission(request_id, asked, offered)
    }

    /// A plan's approval (`ExitPlanMode`).
    pub(super) fn plan(&mut self, request_id: String, asked: &CanUseTool) -> Opened {
        let offered = choices::plan(&asked.input, &asked.tool_use_id);
        self.permission(request_id, asked, offered)
    }

    /// An `AskUserQuestion` call, as a question; `Err` with why it cannot
    /// be shown.
    pub(super) fn question(
        &mut self,
        request_id: String,
        asked: &CanUseTool,
    ) -> Result<Opened, &'static str> {
        let form = question::form(&asked.input, &asked.tool_use_id)?;
        let shown = Shown::Question(form.question.clone());
        Ok(self.hold(
            Pending {
                request_id,
                tool_use_id: asked.tool_use_id.clone(),
                held: Held::Question(form),
            },
            shown,
        ))
    }

    fn permission(
        &mut self,
        request_id: String,
        asked: &CanUseTool,
        offered: Vec<choices::Choice>,
    ) -> Opened {
        let id = PermissionId::random();
        let mut options = Vec::with_capacity(offered.len());
        let mut answers = Vec::with_capacity(offered.len());
        for (option, answer) in offered {
            answers.push((option.id.clone(), answer));
            options.push(option);
        }
        let shown = Shown::Permission(PermissionRequest {
            id,
            tool_call: ToolCallId::new(asked.tool_use_id.as_str()),
            options,
        });
        self.hold(
            Pending {
                request_id,
                tool_use_id: asked.tool_use_id.clone(),
                held: Held::Permission {
                    id,
                    choices: answers,
                },
            },
            shown,
        )
    }

    fn hold(&mut self, pending: Pending, shown: Shown) -> Opened {
        let displaced = if self.pending.len() >= MAX_WAITING {
            let oldest = self.pending.remove(0);
            self.remember(oldest.asked());
            Some(oldest.withdrawn())
        } else {
            None
        };
        self.pending.push(pending);
        Opened { shown, displaced }
    }

    fn remember(&mut self, asked: Asked) {
        let _new = match asked {
            Asked::Permission(id) => self.permissions.insert(id),
            Asked::Question(id) => self.questions.insert(id),
        };
    }

    /// Answer permission request `id` as `outcome` says (a cancelled
    /// outcome withdraws it).
    ///
    /// # Errors
    /// [`InputError::PermissionSettled`], [`InputError::PermissionNotFound`]
    /// or [`InputError::UnknownOption`]; nothing changes then.
    pub(super) fn answer(
        &mut self,
        id: PermissionId,
        outcome: &PermissionOutcome,
    ) -> Result<Settled, InputError> {
        let found = self.pending.iter().position(
            |pending| matches!(&pending.held, Held::Permission { id: held, .. } if *held == id),
        );
        let Some(index) = found else {
            return Err(if self.permissions.contains(&id) {
                InputError::PermissionSettled(id)
            } else {
                InputError::PermissionNotFound(id)
            });
        };
        let chosen = match outcome {
            PermissionOutcome::Selected { option } => {
                let answer = self
                    .pending
                    .get(index)
                    .and_then(|pending| match &pending.held {
                        Held::Permission { choices, .. } => choices
                            .iter()
                            .find(|(offered, _)| offered == option)
                            .map(|(_, answer)| answer.clone()),
                        Held::Question(_) => None,
                    });
                let Some(answer) = answer else {
                    return Err(InputError::UnknownOption {
                        id,
                        option: option.clone(),
                    });
                };
                Some(answer)
            }
            PermissionOutcome::Cancelled => None,
        };
        let pending = self.pending.remove(index);
        let _new = self.permissions.insert(id);
        Ok(match chosen {
            Some(permission) => Settled {
                request_id: pending.request_id,
                permission,
            },
            None => pending.withdrawn().1,
        })
    }

    /// Answer question `id`: checked against its form, then given to the
    /// agent (the tool's input with its answers, or a denial for a
    /// decline).
    ///
    /// # Errors
    /// [`InputError::QuestionSettled`], [`InputError::QuestionNotFound`]
    /// or [`InputError::InvalidAnswer`]; nothing changes then.
    pub(super) fn reply(
        &mut self,
        id: QuestionId,
        answer: QuestionAnswer,
    ) -> Result<(QuestionOutcome, Settled), InputError> {
        let found = self.pending.iter().position(
            |pending| matches!(&pending.held, Held::Question(form) if form.question.id == id),
        );
        let Some(index) = found else {
            return Err(if self.questions.contains(&id) {
                InputError::QuestionSettled(id)
            } else {
                InputError::QuestionNotFound(id)
            });
        };
        if let Some(Pending {
            held: Held::Question(form),
            ..
        }) = self.pending.get(index)
            && let Err(reason) = form.question.check(&answer)
        {
            return Err(InputError::InvalidAnswer { id, reason });
        }
        let pending = self.pending.remove(index);
        let _new = self.questions.insert(id);
        let (outcome, permission) = match (answer, &pending.held) {
            (QuestionAnswer::Accept { values }, Held::Question(form)) => {
                let input = form.answered(&values);
                let permission = Permission::Allow {
                    updated_input: input,
                    updated_permissions: Vec::new(),
                    tool_use_id: pending.tool_use_id.clone(),
                };
                (QuestionOutcome::Answered { values }, permission)
            }
            (QuestionAnswer::Decline, Held::Question(_)) => (
                QuestionOutcome::Declined,
                choices::deny(question::DECLINED, &pending.tool_use_id),
            ),
            // Not reached: `index` was found among the questions. Should it
            // be, the request is still answered, as withdrawn.
            (QuestionAnswer::Accept { .. } | QuestionAnswer::Decline, Held::Permission { .. }) => (
                QuestionOutcome::Cancelled,
                choices::deny(choices::WITHDRAWN, &pending.tool_use_id),
            ),
        };
        Ok((
            outcome,
            Settled {
                request_id: pending.request_id,
                permission,
            },
        ))
    }

    /// The agent withdrew its request `request_id`
    /// (`control_cancel_request`): what was asked, if it was still open.
    pub(super) fn withdrawn(&mut self, request_id: &str) -> Option<Asked> {
        let index = self
            .pending
            .iter()
            .position(|pending| pending.request_id == request_id)?;
        let asked = self.pending.remove(index).asked();
        self.remember(asked);
        Some(asked)
    }

    /// Withdraw everything still open: each, and the denial the agent is
    /// sent.
    pub(super) fn withdraw_all(&mut self) -> Vec<(Asked, Settled)> {
        let withdrawn: Vec<_> = self.pending.drain(..).map(Pending::withdrawn).collect();
        for (asked, _) in &withdrawn {
            self.remember(*asked);
        }
        withdrawn
    }
}

#[cfg(test)]
mod tests;
