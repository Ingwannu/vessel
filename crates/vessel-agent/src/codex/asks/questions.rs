//! Codex blocking question responders and their typed form conversion.
//! Durable nonblocking records belong to the session persistence path instead.

use super::Settled;
use crate::codex::protocol::UserInputRequest;
use crate::{MAX_WAITING, error::InputError, mapper, recent::Recent};
use serde_json::{Value, json};
use vessel_proto::conversation::{AnswerValue, ChoiceOption, FieldKind, FieldName, QuestionField};
use vessel_proto::{Question, QuestionAnswer, QuestionId, QuestionOutcome, ToolCallId};

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
pub(in crate::codex) struct Questions {
    pending: Vec<Asked>,
    settled: Recent<QuestionId>,
}

/// A question held: what to publish, and an older one withdrawn to stay
/// within [`MAX_WAITING`].
pub(in crate::codex) type Opened = (Question, Option<(QuestionId, Settled)>);

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
    pub(in crate::codex) fn user_input(
        &mut self,
        request: Value,
        asked: &UserInputRequest,
    ) -> Result<Opened, &'static str> {
        let question = user_input_question(asked)?;
        Ok(self.open(request, question, Form::UserInput))
    }

    /// A tool server's form (`mcpServer/elicitation/request`), read as
    /// ACP's; `Err` with why it cannot be shown.
    pub(in crate::codex) fn elicitation(
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
    pub(in crate::codex) fn answer(
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
    pub(in crate::codex) fn resolved(&mut self, request: &Value) -> Option<QuestionId> {
        let index = self
            .pending
            .iter()
            .position(|asked| asked.request == *request)?;
        let asked = self.pending.remove(index);
        let _new = self.settled.insert(asked.question.id);
        Some(asked.question.id)
    }

    /// Withdraw every question: each id, and its cancel for Codex.
    pub(in crate::codex) fn withdraw_all(&mut self) -> Vec<(QuestionId, Settled)> {
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
pub(in crate::codex) fn no_answers() -> Value {
    response(Form::UserInput, &QuestionOutcome::Declined)
}

/// Codex's response to an elicitation settled as `outcome`.
pub(in crate::codex) fn elicitation_response(outcome: &QuestionOutcome) -> Value {
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
