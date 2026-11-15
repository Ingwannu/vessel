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
use vessel_proto::conversation::{AnswerValue, ChoiceOption, FieldKind, FieldName, QuestionField};
use vessel_proto::{
    PermissionId, PermissionOption, PermissionOptionId, PermissionOptionKind, PermissionOutcome,
    PermissionRequest, Question, QuestionAnswer, QuestionId, QuestionOutcome, RawJson, ToolCall,
    ToolCallId, ToolCallStatus, ToolKind,
};

use super::protocol::{CommandApproval, PermissionsApproval, UserInputRequest};
use crate::MAX_WAITING;
use crate::error::InputError;
use crate::mapper;
use crate::recent::Recent;

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

fn option(id: &str, name: &str, kind: PermissionOptionKind) -> PermissionOption {
    PermissionOption {
        id: PermissionOptionId::new(id),
        name: name.to_owned(),
        kind,
    }
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
fn command_choice(decision: &Value) -> Option<(&str, &'static str, PermissionOptionKind)> {
    let id = match decision {
        Value::String(id) => id.as_str(),
        // A decision carrying what it applies: its one key names it.
        Value::Object(fields) if fields.len() == 1 => fields.keys().next()?.as_str(),
        Value::Object(_) | Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) => {
            return None;
        }
    };
    let (name, kind) = match id {
        "accept" => ("Run", PermissionOptionKind::AllowOnce),
        "acceptForSession" => (
            "Run, and do not ask again this session",
            PermissionOptionKind::AllowAlways,
        ),
        "acceptWithExecpolicyAmendment" => (
            "Run, and allow commands like it from now on",
            PermissionOptionKind::AllowAlways,
        ),
        "applyNetworkPolicyAmendment" => (
            "Apply the proposed network rule from now on",
            PermissionOptionKind::AllowAlways,
        ),
        "decline" => ("Do not run", PermissionOptionKind::RejectOnce),
        "cancel" => (
            "Do not run, and stop the turn",
            PermissionOptionKind::RejectOnce,
        ),
        _ => return None,
    };
    Some((id, name, kind))
}

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
            id,
            tool_call,
            options,
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
        id: ToolCallId::new(id),
        title,
        kind,
        status: ToolCallStatus::Pending,
        content: Vec::new(),
        locations: Vec::new(),
        raw_input: input.and_then(|input| RawJson::from_text(input.to_string()).ok()),
    }
}

/// How a question's answer goes back to Codex.
#[derive(Debug, Clone, Copy)]
enum Form {
    /// Codex's own questions: each field is the question of that id.
    UserInput,
    /// A tool server's form, answered as an MCP elicitation.
    Elicitation,
}

/// One question waiting for a person.
#[derive(Debug)]
struct Asked {
    question: Question,
    request: Value,
    form: Form,
}

/// The questions waiting for a person.
#[derive(Debug, Default)]
pub(super) struct Questions {
    pending: Vec<Asked>,
    settled: Recent<QuestionId>,
}

/// A question held: what to publish, and an older one withdrawn to stay
/// within [`MAX_WAITING`].
pub(super) type Opened = (Question, Option<(QuestionId, Settled)>);

impl Questions {
    fn open(&mut self, request: Value, question: Question, form: Form) -> Opened {
        let displaced = if self.pending.len() >= MAX_WAITING {
            let oldest = self.pending.remove(0);
            let _new = self.settled.insert(oldest.question.id);
            let settled = Settled {
                request: oldest.request,
                response: response(oldest.form, &QuestionOutcome::Cancelled),
            };
            Some((oldest.question.id, settled))
        } else {
            None
        };
        self.pending.push(Asked {
            question: question.clone(),
            request,
            form,
        });
        (question, displaced)
    }

    /// Codex's own questions (`item/tool/requestUserInput`) as one
    /// question of a field each; `Err` with why they cannot be shown.
    pub(super) fn user_input(
        &mut self,
        request: Value,
        asked: &UserInputRequest,
    ) -> Result<Opened, &'static str> {
        let question = user_input_question(asked)?;
        Ok(self.open(request, question, Form::UserInput))
    }

    /// A tool server's form (`mcpServer/elicitation/request`), read as
    /// ACP's; `Err` with why it cannot be shown.
    pub(super) fn elicitation(
        &mut self,
        request: Value,
        thread: &str,
        params: &Value,
    ) -> Result<Opened, String> {
        let question = elicitation_question(thread, params)?;
        Ok(self.open(request, question, Form::Elicitation))
    }

    /// Answer question `id`: checked against its form, then turned into
    /// Codex's response.
    ///
    /// # Errors
    /// [`InputError::QuestionSettled`], [`InputError::QuestionNotFound`]
    /// or [`InputError::InvalidAnswer`]; nothing changes then.
    pub(super) fn answer(
        &mut self,
        id: QuestionId,
        answer: QuestionAnswer,
    ) -> Result<(QuestionOutcome, Settled), InputError> {
        let Some(index) = self
            .pending
            .iter()
            .position(|asked| asked.question.id == id)
        else {
            return Err(if self.settled.contains(&id) {
                InputError::QuestionSettled(id)
            } else {
                InputError::QuestionNotFound(id)
            });
        };
        if let Some(asked) = self.pending.get(index)
            && let Err(reason) = asked.question.check(&answer)
        {
            return Err(InputError::InvalidAnswer { id, reason });
        }
        let outcome = match answer {
            QuestionAnswer::Accept { values } => QuestionOutcome::Answered { values },
            QuestionAnswer::Decline => QuestionOutcome::Declined,
        };
        let asked = self.pending.remove(index);
        let _new = self.settled.insert(id);
        let settled = Settled {
            request: asked.request,
            response: response(asked.form, &outcome),
        };
        Ok((outcome, settled))
    }

    /// Codex no longer waits on `request` (`serverRequest/resolved`): its
    /// question's id, if one was open.
    pub(super) fn resolved(&mut self, request: &Value) -> Option<QuestionId> {
        let index = self
            .pending
            .iter()
            .position(|asked| asked.request == *request)?;
        let asked = self.pending.remove(index);
        let _new = self.settled.insert(asked.question.id);
        Some(asked.question.id)
    }

    /// Withdraw every question: each id, and its cancel for Codex.
    pub(super) fn withdraw_all(&mut self) -> Vec<(QuestionId, Settled)> {
        let withdrawn: Vec<_> = self
            .pending
            .drain(..)
            .map(|asked| {
                let settled = Settled {
                    request: asked.request,
                    response: response(asked.form, &QuestionOutcome::Cancelled),
                };
                (asked.question.id, settled)
            })
            .collect();
        for (id, _) in &withdrawn {
            let _new = self.settled.insert(*id);
        }
        withdrawn
    }
}

/// Codex's response to `item/tool/requestUserInput` when nobody is asked:
/// no answers.
pub(super) fn no_answers() -> Value {
    response(Form::UserInput, &QuestionOutcome::Declined)
}

/// Codex's response to an elicitation settled as `outcome`.
pub(super) fn elicitation_response(outcome: &QuestionOutcome) -> Value {
    response(Form::Elicitation, outcome)
}

/// Codex's response to a question settled as `outcome`.
fn response(form: Form, outcome: &QuestionOutcome) -> Value {
    match form {
        Form::UserInput => {
            let answers: serde_json::Map<String, Value> = match outcome {
                QuestionOutcome::Answered { values } => values
                    .iter()
                    .map(|(name, value)| {
                        let given = match value {
                            AnswerValue::Text(text) | AnswerValue::Choice(text) => {
                                vec![text.clone()]
                            }
                            AnswerValue::Choices(texts) => texts.clone(),
                            AnswerValue::Number(number) => vec![number.get().to_string()],
                            AnswerValue::Integer(number) => vec![number.to_string()],
                            AnswerValue::Boolean(yes) => vec![yes.to_string()],
                        };
                        (name.as_str().to_owned(), json!({ "answers": given }))
                    })
                    .collect(),
                QuestionOutcome::Declined | QuestionOutcome::Cancelled => serde_json::Map::new(),
            };
            json!({ "answers": answers })
        }
        Form::Elicitation => serde_json::to_value(mapper::elicitation_response(outcome))
            .unwrap_or_else(|_unwritable| json!({ "action": "cancel" })),
    }
}

/// Codex's questions as one form; `Err` with why it cannot be shown.
fn user_input_question(asked: &UserInputRequest) -> Result<Question, &'static str> {
    if asked.questions.is_empty() {
        return Err("it asks nothing");
    }
    if asked.questions.len() > Question::MAX_FIELDS {
        return Err("it asks more than a question keeps");
    }
    if asked.questions.iter().any(|question| question.is_secret) {
        return Err("it asks for a secret, which the session's timeline must not keep");
    }
    let mut fields = Vec::with_capacity(asked.questions.len());
    for question in &asked.questions {
        let name: FieldName = question
            .id
            .parse()
            .map_err(|_unfit| "a question's id cannot name a field")?;
        let options = question.options.as_deref().unwrap_or_default();
        let choice = !options.is_empty() && !question.is_other;
        if options.len() > Question::MAX_OPTIONS {
            return Err("a question offers more options than a question keeps");
        }
        let kind = if choice {
            FieldKind::Choice {
                options: options
                    .iter()
                    .map(|option| ChoiceOption {
                        value: option.label.clone(),
                        title: cut(option.label.clone(), Question::MAX_LABEL),
                        description: Some(cut(option.description.clone(), Question::MAX_LABEL)),
                    })
                    .collect(),
                multiple: false,
                min_items: None,
                max_items: None,
                default: Vec::new(),
            }
        } else {
            FieldKind::Text {
                min_length: None,
                max_length: None,
                format: None,
                pattern: None,
                default: None,
            }
        };
        fields.push(QuestionField {
            name,
            title: Some(cut(question.header.clone(), Question::MAX_LABEL)),
            description: Some(cut(question.question.clone(), Question::MAX_LABEL)),
            required: true,
            kind,
        });
    }
    let message = asked
        .questions
        .iter()
        .map(|question| question.question.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Question {
        id: QuestionId::random(),
        message: cut(message, Question::MAX_MESSAGE),
        tool_call: Some(ToolCallId::new(asked.item_id.as_str())),
        fields,
    })
}

/// A tool server's form read as ACP's `elicitation/create` on `thread`.
fn elicitation_question(thread: &str, params: &Value) -> Result<Question, String> {
    let mut form = params.clone();
    let Some(fields) = form.as_object_mut() else {
        return Err("a form that is not an object".to_owned());
    };
    // ACP's request names the session it belongs to.
    let _old = fields.insert("sessionId".to_owned(), json!(thread));
    let request = serde_json::from_value(form).map_err(|error| error.to_string())?;
    mapper::question(QuestionId::random(), request).map_err(|refusal| match refusal {
        mapper::Refusal::NotAForm { mode } => format!("a `{mode}` request, not a form"),
        mapper::Refusal::Unfit(why) => why.to_string(),
    })
}

/// `text` cut to at most `bytes`, at a character's edge.
fn cut(mut text: String, bytes: usize) -> String {
    if text.len() > bytes {
        let mut at = bytes;
        while !text.is_char_boundary(at) {
            at = at.saturating_sub(1);
        }
        text.truncate(at);
    }
    text
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::codex::protocol::{UserInputOption, UserInputQuestion};

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
            is_secret: false,
            options: options.map(|labels| {
                labels
                    .into_iter()
                    .map(|label| UserInputOption {
                        label: label.to_owned(),
                        description: String::new(),
                    })
                    .collect()
            }),
        }
    }

    /// Options make a choice, none (or free text too) a text; the answer
    /// goes back under each question's id.
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
        assert_eq!(kinds, [true, false, false]);
        let values: BTreeMap<FieldName, AnswerValue> = [
            ("pick", AnswerValue::Choice("b".to_owned())),
            ("free", AnswerValue::Text("words".to_owned())),
            ("either", AnswerValue::Text("c".to_owned())),
        ]
        .into_iter()
        .map(|(name, value)| (name.parse().unwrap(), value))
        .collect();
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
