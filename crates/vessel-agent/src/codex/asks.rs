//! What Codex asks a person: approvals as permission requests, its own
//! questions and its tool servers' forms as questions, each held until a
//! client answers, Codex withdraws it, the turn ends or the connection
//! does (the plan's Codex slice 2).
//!
//! [Decision Log]
//! - Purpose: the person decides, with the choices Codex itself offers;
//!   the session sees the same permission requests and questions as over
//!   ACP, under node-minted ids many clients can answer.
//! - Choices: a command's `availableDecisions` (each kept as Codex gave it,
//!   so a decision carrying a rule goes back with that rule; one this build
//!   does not know is left out, never guessed), a file change's four
//!   decisions, more permissions for the turn or the session or none.
//!   Anything settled without an answer (a cancel, the turn's end, the
//!   connection's, more than [`MAX_WAITING`] open) is answered with Codex's
//!   own refusal: `cancel` for a command or a file change, no permissions,
//!   no answers, an elicitation's `cancel`.
//! - Questions: a question with options is a choice (one of them); one
//!   without, or one that also takes free text (`isOther`), is text. A
//!   secret question (`isSecret`) is not shown: an answer is kept in the
//!   session's timeline, where a secret must never be. A tool server's
//!   form (an MCP elicitation in form mode) is read as ACP's (both are
//!   MCP's schema) by the ACP driver's own mapper, so one reader holds the
//!   bounds and the checks.
//! - Why not the ACP driver's `Permissions` and `Questions`: they hold
//!   ACP's responders; these hold Codex's request ids and the response
//!   each answer becomes. The ids, the bound and the "already settled"
//!   memory are the same.

use serde_json::{Value, json};
use vessel_proto::{
    PermissionId, PermissionOption, PermissionOptionId, PermissionOptionKind, PermissionOutcome,
    PermissionRequest, RawJson, ToolCall, ToolCallId, ToolCallStatus, ToolKind,
};

use super::protocol::{CommandApproval, PermissionsApproval};
use crate::MAX_WAITING;
use crate::error::InputError;
use crate::recent::Recent;

mod form;
mod questions;
pub(super) use questions::{Opened, Questions, elicitation_response, no_answers};

mod choices;
use choices::{command_choice, option};

/// A request of Codex's, settled: answer `request` with `response`.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Settled {
    pub(super) request: Value,
    pub(super) response: Value,
}

/// One approval waiting for a person.
#[derive(Debug)]
struct Approval {
    id: PermissionId,
    request: Value,
    /// Each option, and the response it becomes.
    choices: Vec<(PermissionOptionId, Value)>,
    /// The response when it is withdrawn.
    withdrawn: Value,
}

/// The approvals waiting for a person.
#[derive(Debug, Default)]
pub(super) struct Approvals {
    pending: Vec<Approval>,
    settled: Recent<PermissionId>,
}

/// An approval to ask: the permission request's choices and responses.
#[derive(Debug)]
pub(super) struct Asking {
    options: Vec<(PermissionOption, Value)>,
    withdrawn: Value,
}

impl Asking {
    /// A command's approval, with the decisions Codex offers.
    pub(super) fn command(asked: &CommandApproval) -> Self {
        const PLAIN: [&str; 4] = ["accept", "acceptForSession", "decline", "cancel"];
        let offered = asked
            .available_decisions
            .clone()
            .unwrap_or_else(|| PLAIN.iter().map(|decision| json!(decision)).collect());
        let options = offered
            .into_iter()
            .filter_map(|decision| {
                let (id, name, kind) = command_choice(&decision)?;
                let chosen = option(id, name, kind);
                Some((chosen, json!({ "decision": decision })))
            })
            .collect();
        Self {
            options,
            withdrawn: json!({ "decision": "cancel" }),
        }
    }

    /// A file change's approval.
    pub(super) fn file_change() -> Self {
        let choice =
            |id: &str, name: &str, kind| (option(id, name, kind), json!({ "decision": id }));
        Self {
            options: vec![
                choice("accept", "Apply", PermissionOptionKind::AllowOnce),
                choice(
                    "acceptForSession",
                    "Apply, and do not ask again for these files this session",
                    PermissionOptionKind::AllowAlways,
                ),
                choice("decline", "Do not apply", PermissionOptionKind::RejectOnce),
                choice(
                    "cancel",
                    "Do not apply, and stop the turn",
                    PermissionOptionKind::RejectOnce,
                ),
            ],
            withdrawn: json!({ "decision": "cancel" }),
        }
    }

    /// A request for more permissions: granted as asked for the turn or
    /// the session, or not at all.
    pub(super) fn permissions(asked: &PermissionsApproval) -> Self {
        let grant = |scope: &str| json!({ "permissions": asked.permissions, "scope": scope });
        let none = json!({ "permissions": {} });
        Self {
            options: vec![
                (
                    option(
                        "turn",
                        "Allow for this turn",
                        PermissionOptionKind::AllowOnce,
                    ),
                    grant("turn"),
                ),
                (
                    option(
                        "session",
                        "Allow for this session",
                        PermissionOptionKind::AllowAlways,
                    ),
                    grant("session"),
                ),
                (
                    option("decline", "Do not allow", PermissionOptionKind::RejectOnce),
                    none.clone(),
                ),
            ],
            withdrawn: none,
        }
    }
}

/// A command decision as an option: its id, its words and its kind;
/// `None` for one this build does not know.
impl Approvals {
    /// Hold `asking` for Codex's request `request` on `tool_call`: the
    /// permission request to publish, and an older approval withdrawn to
    /// stay within [`MAX_WAITING`].
    pub(super) fn open(
        &mut self,
        request: Value,
        tool_call: ToolCallId,
        asking: Asking,
    ) -> (PermissionRequest, Option<(PermissionId, Settled)>) {
        let displaced = if self.pending.len() >= MAX_WAITING {
            let oldest = self.pending.remove(0);
            let _new = self.settled.insert(oldest.id);
            let settled = Settled {
                request: oldest.request,
                response: oldest.withdrawn,
            };
            Some((oldest.id, settled))
        } else {
            None
        };
        let id = PermissionId::random();
        let mut options = Vec::with_capacity(asking.options.len());
        let mut choices = Vec::with_capacity(asking.options.len());
        for (offered, response) in asking.options {
            choices.push((offered.id.clone(), response));
            options.push(offered);
        }
        self.pending.push(Approval {
            id,
            request,
            choices,
            withdrawn: asking.withdrawn,
        });
        let asked = PermissionRequest {
            plan_proposal: None,
            delivery_state: vessel_proto::timeline::PermissionDeliveryState::Pending,
            tool_scope: None,
            id,
            tool_call: Some(tool_call),
            options,

            presentation: None,
        };
        (asked, displaced)
    }

    /// Answer approval `id` as `outcome` says.
    ///
    /// # Errors
    /// [`InputError::PermissionSettled`], [`InputError::PermissionNotFound`]
    /// or [`InputError::UnknownOption`]; nothing changes then.
    pub(super) fn answer(
        &mut self,
        id: PermissionId,
        outcome: &PermissionOutcome,
    ) -> Result<Settled, InputError> {
        let Some(index) = self.pending.iter().position(|pending| pending.id == id) else {
            return Err(if self.settled.contains(&id) {
                InputError::PermissionSettled(id)
            } else {
                InputError::PermissionNotFound(id)
            });
        };
        let chosen = match outcome {
            PermissionOutcome::Selected { option } => {
                let response = self.pending.get(index).and_then(|pending| {
                    pending
                        .choices
                        .iter()
                        .find(|(offered, _)| offered == option)
                        .map(|(_, response)| response.clone())
                });
                let Some(response) = response else {
                    return Err(InputError::UnknownOption {
                        id,
                        option: option.clone(),
                    });
                };
                Some(response)
            }
            PermissionOutcome::Cancelled => None,
        };
        let pending = self.pending.remove(index);
        let _new = self.settled.insert(id);
        Ok(Settled {
            request: pending.request,
            response: chosen.unwrap_or(pending.withdrawn),
        })
    }

    /// Codex no longer waits on `request` (`serverRequest/resolved`): its
    /// approval's id, if one was open.
    pub(super) fn resolved(&mut self, request: &Value) -> Option<PermissionId> {
        let index = self
            .pending
            .iter()
            .position(|pending| pending.request == *request)?;
        let pending = self.pending.remove(index);
        let _new = self.settled.insert(pending.id);
        Some(pending.id)
    }

    /// Withdraw every approval: each id, and its refusal for Codex.
    pub(super) fn withdraw_all(&mut self) -> Vec<(PermissionId, Settled)> {
        let withdrawn: Vec<_> = self
            .pending
            .drain(..)
            .map(|pending| {
                let settled = Settled {
                    request: pending.request,
                    response: pending.withdrawn,
                };
                (pending.id, settled)
            })
            .collect();
        for (id, _) in &withdrawn {
            let _new = self.settled.insert(*id);
        }
        withdrawn
    }
}

/// The tool call an approval belongs to, announced from the request when
/// Codex has not announced its item yet.
pub(super) fn tool_call(
    id: &str,
    title: String,
    kind: ToolKind,
    input: Option<&Value>,
) -> ToolCall {
    ToolCall {
        scope: None,
        id: ToolCallId::new(id),
        title,
        kind,
        status: ToolCallStatus::Pending,
        content: Vec::new(),
        locations: Vec::new(),
        raw_input: input.and_then(|input| RawJson::from_text(input.to_string()).ok()),

        detail: None,
        failure: None,
        agent_message: None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::codex::protocol::{UserInputOption, UserInputQuestion, UserInputRequest};
    use vessel_proto::conversation::{AnswerValue, FieldKind, FieldName};
    use vessel_proto::{QuestionAnswer, QuestionOutcome};

    fn command(decisions: Option<&Value>) -> CommandApproval {
        serde_json::from_value(json!({
            "threadId": "t", "turnId": "u", "itemId": "c", "command": "ls",
            "availableDecisions": decisions,
        }))
        .unwrap()
    }

    /// A decision carrying a rule goes back whole; one this build does not
    /// know is left out.
    #[test]
    fn a_command_offers_codexs_own_decisions() {
        let rule = json!({"acceptWithExecpolicyAmendment": {"execpolicy_amendment": ["ls"]}});
        let asked = command(Some(&json!(["accept", rule, "somethingNew", "cancel"])));
        let mut approvals = Approvals::default();
        let (request, displaced) =
            approvals.open(json!(7), ToolCallId::new("c"), Asking::command(&asked));
        assert!(displaced.is_none());
        let ids: Vec<&str> = request.options.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(ids, ["accept", "acceptWithExecpolicyAmendment", "cancel"]);
        let option = PermissionOptionId::new("acceptWithExecpolicyAmendment");
        let settled = approvals
            .answer(request.id, &PermissionOutcome::Selected { option })
            .unwrap();
        assert_eq!(settled.request, json!(7));
        assert_eq!(settled.response, json!({ "decision": rule }));
        assert!(matches!(
            approvals.answer(request.id, &PermissionOutcome::Cancelled),
            Err(InputError::PermissionSettled(_))
        ));
    }

    /// Without `availableDecisions`, the four plain ones; a cancel answers
    /// `cancel`, and an option not offered changes nothing.
    #[test]
    fn a_plain_command_and_its_cancel() {
        let mut approvals = Approvals::default();
        let (request, _) = approvals.open(
            json!("r"),
            ToolCallId::new("c"),
            Asking::command(&command(None)),
        );
        assert_eq!(request.options.len(), 4);
        let option = PermissionOptionId::new("approve");
        assert!(matches!(
            approvals.answer(request.id, &PermissionOutcome::Selected { option }),
            Err(InputError::UnknownOption { .. })
        ));
        let settled = approvals
            .answer(request.id, &PermissionOutcome::Cancelled)
            .unwrap();
        assert_eq!(settled.response, json!({ "decision": "cancel" }));
    }

    /// More permissions are granted as asked, for the scope chosen; the
    /// withdrawn answer grants none.
    #[test]
    fn permissions_are_granted_as_asked_or_not_at_all() {
        let asked: PermissionsApproval = serde_json::from_value(json!({
            "threadId": "t", "turnId": "u", "itemId": "p", "cwd": "/w", "startedAtMs": 1,
            "permissions": {"network": {"enabled": true}},
        }))
        .unwrap();
        let mut approvals = Approvals::default();
        let (request, _) =
            approvals.open(json!(1), ToolCallId::new("p"), Asking::permissions(&asked));
        let option = PermissionOptionId::new("session");
        let settled = approvals
            .answer(request.id, &PermissionOutcome::Selected { option })
            .unwrap();
        assert_eq!(
            settled.response,
            json!({"permissions": {"network": {"enabled": true}}, "scope": "session"})
        );
        let (again, _) =
            approvals.open(json!(2), ToolCallId::new("p"), Asking::permissions(&asked));
        let withdrawn = approvals.withdraw_all();
        assert_eq!(withdrawn.len(), 1);
        assert_eq!(withdrawn.first().map(|(id, _)| *id), Some(again.id));
        assert_eq!(
            withdrawn
                .first()
                .map(|(_, settled)| settled.response.clone()),
            Some(json!({ "permissions": {} }))
        );
    }

    /// The oldest approval is withdrawn past the bound.
    #[test]
    fn the_oldest_approval_gives_way() {
        let mut approvals = Approvals::default();
        let mut first = None;
        for n in 0..MAX_WAITING {
            let (request, displaced) =
                approvals.open(json!(n), ToolCallId::new("c"), Asking::file_change());
            assert!(displaced.is_none());
            let _first = first.get_or_insert(request.id);
        }
        let (_, displaced) =
            approvals.open(json!("last"), ToolCallId::new("c"), Asking::file_change());
        let (id, settled) = displaced.unwrap();
        assert_eq!(Some(id), first);
        assert_eq!(settled.request, json!(0));
        assert_eq!(settled.response, json!({ "decision": "cancel" }));
    }

    fn user_input(questions: Vec<UserInputQuestion>) -> UserInputRequest {
        UserInputRequest {
            thread_id: None,
            turn_id: None,
            item_id: "q".to_owned(),
            questions,
        }
    }

    fn asked(id: &str, options: Option<Vec<&str>>, is_other: bool) -> UserInputQuestion {
        UserInputQuestion {
            id: id.to_owned(),
            header: "Header".to_owned(),
            question: format!("{id}?"),
            is_other,
            allow_other: false,
            multi_select: crate::codex::protocol::NativeSelection::Single,
            allow_empty: vessel_proto::timeline::EmptyAnswerPolicy::RequireText,
            placeholder: None,
            dismiss_label: None,
            is_secret: false,
            options: options.map(|labels| {
                labels
                    .into_iter()
                    .map(|label| UserInputOption {
                        label: label.to_owned(),
                        description: None,
                        preview: None,
                    })
                    .collect()
            }),
        }
    }

    /// Choices remain choices when custom text is allowed; only the provider's
    /// original parent id receives the normalized answer.
    #[test]
    fn codexs_questions_become_one_form() {
        let mut questions = Questions::default();
        let request = user_input(vec![
            asked("pick", Some(vec!["a", "b"]), false),
            asked("free", None, false),
            asked("either", Some(vec!["a"]), true),
        ]);
        let (question, _) = questions.user_input(json!(3), &request).unwrap();
        let kinds: Vec<bool> = question
            .fields
            .iter()
            .map(|field| matches!(field.kind, FieldKind::Choice { .. }))
            .collect();
        assert_eq!(kinds, [true, false, true, false]);
        let companion = question.fields[2]
            .presentation
            .as_ref()
            .unwrap()
            .custom_answer
            .clone()
            .unwrap();
        let mut values: BTreeMap<FieldName, AnswerValue> = [
            ("pick", AnswerValue::Choice("b".to_owned())),
            ("free", AnswerValue::Text("words".to_owned())),
        ]
        .into_iter()
        .map(|(name, value)| (name.parse().unwrap(), value))
        .collect();
        let _previous = values.insert(companion, AnswerValue::Text("c".to_owned()));
        let (outcome, settled) = questions
            .answer(question.id, QuestionAnswer::Accept { values })
            .unwrap();
        assert!(matches!(outcome, QuestionOutcome::Answered { .. }));
        assert_eq!(settled.request, json!(3));
        assert_eq!(
            settled.response,
            json!({"answers": {
                "pick": {"answers": ["b"]},
                "free": {"answers": ["words"]},
                "either": {"answers": ["c"]},
            }})
        );
    }

    /// An answer outside the choices is refused, and the question waits.
    #[test]
    fn an_answer_that_does_not_fit_changes_nothing() {
        let mut questions = Questions::default();
        let request = user_input(vec![asked("pick", Some(vec!["a"]), false)]);
        let (question, _) = questions.user_input(json!(4), &request).unwrap();
        let values = [("pick".parse().unwrap(), AnswerValue::Choice("z".to_owned()))].into();
        assert!(matches!(
            questions.answer(question.id, QuestionAnswer::Accept { values }),
            Err(InputError::InvalidAnswer { .. })
        ));
        let (_, settled) = questions
            .answer(question.id, QuestionAnswer::Decline)
            .unwrap();
        assert_eq!(settled.response, json!({ "answers": {} }));
    }

    /// A secret question is never shown.
    #[test]
    fn a_secret_is_not_asked() {
        let mut secret = asked("token", None, false);
        secret.is_secret = true;
        let mut questions = Questions::default();
        assert!(
            questions
                .user_input(json!(5), &user_input(vec![secret]))
                .is_err()
        );
        assert!(questions.withdraw_all().is_empty());
    }

    /// A tool server's form is read as ACP's; another mode is not a form.
    #[test]
    fn a_tool_servers_form_is_asked_and_answered_as_mcps() {
        let mut questions = Questions::default();
        let form = json!({
            "threadId": "t", "turnId": "u", "serverName": "s", "mode": "form",
            "message": "Which?",
            "requestedSchema": {"type": "object", "properties": {
                "name": {"type": "string"}}, "required": ["name"]},
        });
        let (question, _) = questions.elicitation(json!(6), "t", &form).unwrap();
        assert_eq!(question.fields.len(), 1);
        let values = [("name".parse().unwrap(), AnswerValue::Text("x".to_owned()))].into();
        let (_, settled) = questions
            .answer(question.id, QuestionAnswer::Accept { values })
            .unwrap();
        assert_eq!(
            settled.response,
            json!({"action": "accept", "content": {"name": "x"}})
        );
        let link = json!({
            "threadId": "t", "serverName": "s", "mode": "url", "message": "Open",
            "url": "https://example.invalid", "elicitationId": "e",
        });
        assert!(questions.elicitation(json!(7), "t", &link).is_err());
        assert_eq!(
            elicitation_response(&QuestionOutcome::Declined),
            json!({ "action": "decline" })
        );
    }

    /// Codex withdrawing a request settles it without an answer.
    #[test]
    fn what_codex_resolves_is_settled_here() {
        let mut approvals = Approvals::default();
        let (request, _) = approvals.open(json!(9), ToolCallId::new("c"), Asking::file_change());
        assert_eq!(approvals.resolved(&json!(8)), None);
        assert_eq!(approvals.resolved(&json!(9)), Some(request.id));
        assert!(approvals.withdraw_all().is_empty());
        let mut questions = Questions::default();
        let request = user_input(vec![asked("free", None, false)]);
        let (question, _) = questions.user_input(json!("q"), &request).unwrap();
        assert_eq!(questions.resolved(&json!("q")), Some(question.id));
        assert!(matches!(
            questions.answer(question.id, QuestionAnswer::Decline),
            Err(InputError::QuestionSettled(_))
        ));
    }
}
