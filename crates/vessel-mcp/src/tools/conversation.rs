//! The conversation's tools (protocol 10, milestone 11's Track A): a
//! prompt delivered as asked (`send_input`'s `delivery`), what a session
//! holds now, its queue, answering a question, starting an ended session
//! again, archiving. Each is one call through the agent's own principal,
//! like the rest; the node's ownership rule (ADR 0013) decides which
//! sessions an agent may change.
//!
//! [Decision Log]
//! - Purpose: an agent that drives another session meets what a person
//!   does: a turn still running when it has the next instruction, a
//!   question the other agent asks, a session that ended and should go on
//!   or be put away.
//! - Not offered, and why: marking read or unread (the node refuses an
//!   agent: an agent reading a session is not its owner seeing it);
//!   pinning (how a person orders their own lists); deleting (it removes
//!   a log for good, and an agent puts a session away by archiving it);
//!   attachments (an agent hands another agent a file by naming its path
//!   in the prompt, which that agent reads with its own tools).
//! - Values as an agent writes them: `answer_question` takes a JSON
//!   object, each value read by the question's own field kinds
//!   (`vessel_client::compose`, which the CLI uses too), so an agent never
//!   spells out `AnswerValue`'s tagged form.
//! - Today an agent operates only on sessions it started, and the node
//!   offers questions only to sessions a person or a device started (the
//!   contract's review, item 4): `answer_question` is then refused for an
//!   agent's own sessions as for others'. It is built on the wire as it
//!   is, for the profile grants ADR 0013 leaves for later.

use rmcp::model::{JsonObject, Tool};
use serde_json::{Value, json};
use vessel_client::compose;
use vessel_proto::conversation::{
    ChangeQueue, GetConversation, MarkSession, PromptSession, ResumeSession,
};
use vessel_proto::{
    ContentBlock, Delivery, GetSession, Mark, QuestionAnswer, QuestionId, QueueChange,
    QueueEntryId, SendInput, SessionId, SessionInput,
};

use super::{Target, ToolError, VesselTools, flag, number, required, session, text, tool};

impl VesselTools {
    /// One of this module's tools.
    pub(super) async fn conversation(
        &self,
        name: &str,
        args: &JsonObject,
    ) -> Result<Value, ToolError> {
        let node = self.target(args).await?;
        let session = session(args)?;
        match name {
            "read_conversation" => Ok(json!(self.on(node, GetConversation { session }).await?)),
            "change_queue" => self.change_queue(node, session, args).await,
            "answer_question" => self.answer_question(node, session, args).await,
            "resume_session" => {
                let info = self.on(node, ResumeSession { session }).await?;
                Ok(json!({ "session": info }))
            }
            "archive_session" => {
                let mark = if flag(args, "unarchive")? {
                    Mark::Unarchive
                } else {
                    Mark::Archive
                };
                let info = self.on(node, MarkSession { session, mark }).await?;
                Ok(json!({ "session": info }))
            }
            other => Err(ToolError::Arguments(format!("no tool `{other}`"))),
        }
    }

    /// `send_input` with a `delivery` or `notify`: `session.prompt`; where
    /// the log stood just before, and what became of the prompt. With
    /// `notify`, the caller's own session is told about the turn it
    /// starts (protocol 11).
    pub(super) async fn deliver(
        &self,
        node: Target,
        session: SessionId,
        text: &str,
        delivery: Delivery,
        notify: bool,
    ) -> Result<Value, ToolError> {
        let from = self.on(node, GetSession { session }).await?.next_seq;
        let content = vec![ContentBlock::Text {
            text: text.to_owned(),
        }];
        let delivered = self
            .on(
                node,
                PromptSession {
                    session,
                    content,
                    delivery,
                    notify,
                },
            )
            .await?;
        Ok(json!({ "from": from, "delivered": delivered }))
    }

    async fn change_queue(
        &self,
        node: Target,
        session: SessionId,
        args: &JsonObject,
    ) -> Result<Value, ToolError> {
        let change = match required(args, "change")? {
            "hold" => QueueChange::Hold,
            "resume" => QueueChange::Resume,
            "clear" => QueueChange::Clear,
            "remove" => QueueChange::Remove {
                entry: entry(args, "entry")?,
            },
            "move" => QueueChange::Move {
                entry: entry(args, "entry")?,
                before: text(args, "before")?.map(parse_entry).transpose()?,
            },
            "send_now" => QueueChange::SendNow {
                entry: entry(args, "entry")?,
                delivery: delivery(args)?.unwrap_or(Delivery::Start),
            },
            "edit" => self.edit(node, session, args).await?,
            other => {
                return Err(ToolError::Arguments(format!(
                    "`change` is edit, move, remove, send_now, hold, resume or clear, not `{other}`"
                )));
            }
        };
        let changed = self.on(node, ChangeQueue { session, change }).await?;
        Ok(json!(changed))
    }

    /// An edit of an entry's text: its attachments stay, and the revision
    /// is the one given, else the entry's current one. An entry no longer
    /// waiting is sent as given, for the node to say so (`not_found`).
    async fn edit(
        &self,
        node: Target,
        session: SessionId,
        args: &JsonObject,
    ) -> Result<QueueChange, ToolError> {
        let entry = entry(args, "entry")?;
        let words = required(args, "text")?.to_owned();
        let revision = number(args, "revision")?
            .map(|revision| {
                u32::try_from(revision)
                    .map_err(|_too_big| ToolError::Arguments("`revision` is too large".to_owned()))
            })
            .transpose()?;
        let waiting = self.on(node, GetConversation { session }).await?.queue;
        let found = waiting.entries.iter().find(|queued| queued.id == entry);
        Ok(QueueChange::Edit {
            entry,
            revision: revision
                .or_else(|| found.map(|queued| queued.revision))
                .unwrap_or_default(),
            content: compose::with_text(
                found.map_or(&[], |queued| queued.content.as_slice()),
                words,
            ),
        })
    }

    async fn answer_question(
        &self,
        node: Target,
        session: SessionId,
        args: &JsonObject,
    ) -> Result<Value, ToolError> {
        let id: QuestionId = required(args, "question")?
            .parse()
            .map_err(|error| ToolError::Arguments(format!("question: {error}")))?;
        let answer = if flag(args, "decline")? {
            QuestionAnswer::Decline
        } else {
            let given = match args.get("values") {
                None | Some(Value::Null) => Vec::new(),
                Some(Value::Object(values)) => texts(values)?,
                Some(_) => {
                    return Err(ToolError::Arguments(
                        "`values` must be an object of field names and values".to_owned(),
                    ));
                }
            };
            let waiting = self.on(node, GetConversation { session }).await?.questions;
            match waiting.iter().find(|question| question.id == id) {
                Some(question) => compose::fill(
                    question,
                    given.iter().map(|(name, value)| (*name, value.as_str())),
                )
                .map_err(|error| ToolError::Failed(error.to_string()))?,
                // Not waiting: the node says why (`not_found`, or
                // `conflict` once it was answered).
                None => QuestionAnswer::Accept {
                    values: std::collections::BTreeMap::new(),
                },
            }
        };
        let input = SessionInput::AnswerQuestion { id, answer };
        let _ack = self.on(node, SendInput { session, input }).await?;
        Ok(json!({ "answered": true }))
    }
}

/// How a prompt goes, from `delivery`, if given.
pub(super) fn delivery(args: &JsonObject) -> Result<Option<Delivery>, ToolError> {
    text(args, "delivery")?
        .map(|delivery| {
            serde_json::from_value(Value::String(delivery.to_owned()))
                .map_err(|error| ToolError::Arguments(format!("delivery: {error}")))
        })
        .transpose()
}

fn entry(args: &JsonObject, key: &str) -> Result<QueueEntryId, ToolError> {
    parse_entry(required(args, key)?)
}

fn parse_entry(text: &str) -> Result<QueueEntryId, ToolError> {
    text.parse()
        .map_err(|error| ToolError::Arguments(format!("entry: {error}")))
}

/// A question's values as `(field, text)` pairs: a string as it is, a
/// number or a boolean as JSON writes it, a list as one pair per item.
fn texts(values: &JsonObject) -> Result<Vec<(&str, String)>, ToolError> {
    let mut pairs = Vec::with_capacity(values.len());
    for (name, value) in values {
        match value {
            Value::Array(items) => {
                for item in items {
                    pairs.push((name.as_str(), scalar(name, item)?));
                }
            }
            Value::Null
            | Value::Bool(_)
            | Value::Number(_)
            | Value::String(_)
            | Value::Object(_) => {
                pairs.push((name.as_str(), scalar(name, value)?));
            }
        }
    }
    Ok(pairs)
}

fn scalar(name: &str, value: &Value) -> Result<String, ToolError> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Number(number) => Ok(number.to_string()),
        Value::Bool(yes) => Ok(yes.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => Err(ToolError::Arguments(format!(
            "`values.{name}` must be a string, a number, true or false, or a list of them"
        ))),
    }
}

/// `send_input`'s `delivery`.
pub(super) fn delivery_property() -> Value {
    json!({
        "type": "string",
        "enum": ["start", "queue", "steer", "interrupt"],
        "description": "How a structured session takes it while a turn runs: `start` is refused \
                        then (as when left out), `queue` waits its turn, `steer` joins the running \
                        turn (agents that take that), `interrupt` cancels the running turn first."
    })
}

/// This module's tools with their input schemas.
pub(super) fn tools(node: &Value, session: &Value) -> Vec<Tool> {
    let entry = json!({ "type": "string", "description": "A queued prompt's id." });
    vec![
        tool(
            "read_conversation",
            "A structured session's conversation as it stands: the prompts waiting in its queue \
             (with each entry's id and revision), the agent's commands and context use, the \
             permission requests and questions waiting, and what its agent can do.",
            json!({ "node": node, "session": session }),
            &["session"],
        ),
        tool(
            "change_queue",
            "Change a structured session's queue of waiting prompts: `edit` an entry's text, \
             `move` it before another (or last), `remove` it, `send_now`, `hold` the queue, \
             `resume` it, or `clear` it. Returns the queue after.",
            json!({
                "node": node,
                "session": session,
                "change": {
                    "type": "string",
                    "enum": ["edit", "move", "remove", "send_now", "hold", "resume", "clear"]
                },
                "entry": entry,
                "before": {
                    "type": "string",
                    "description": "For `move`: the entry it goes before; left out, last."
                },
                "text": { "type": "string", "description": "For `edit`: the new text." },
                "revision": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "For `edit`: the revision read; left out, the current one."
                },
                "delivery": {
                    "type": "string",
                    "enum": ["start", "steer", "interrupt"],
                    "description": "For `send_now`; left out, `start`."
                }
            }),
            &["session", "change"],
        ),
        tool(
            "answer_question",
            "Answer a question a session's agent asks (`question_asked` from `wait_turn`): \
             `values` maps each field's name to its value (a list for a field of several \
             choices), or `decline` it. The first answer wins.",
            json!({
                "node": node,
                "session": session,
                "question": { "type": "string", "description": "The question's id." },
                "values": { "type": "object" },
                "decline": { "type": "boolean" }
            }),
            &["session", "question"],
        ),
        tool(
            "resume_session",
            "Start an ended structured session you started again: its agent resumes the \
             conversation in the same log; prompts that were waiting come back held \
             (`change_queue` `resume` lets them go).",
            json!({ "node": node, "session": session }),
            &["session"],
        ),
        tool(
            "archive_session",
            "Put a session you started away, stopping it first if it runs; with `unarchive`, \
             bring it back.",
            json!({ "node": node, "session": session, "unarchive": { "type": "boolean" } }),
            &["session"],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_become_one_text_per_value() {
        let Value::Object(values) =
            json!({ "way": "left", "count": 3, "sure": true, "ways": ["a", "b"] })
        else {
            panic!("an object");
        };
        let mut pairs = texts(&values).unwrap();
        pairs.sort();
        assert_eq!(
            pairs,
            [
                ("count", "3".to_owned()),
                ("sure", "true".to_owned()),
                ("way", "left".to_owned()),
                ("ways", "a".to_owned()),
                ("ways", "b".to_owned()),
            ]
        );
        for bad in [
            json!({ "way": null }),
            json!({ "way": { "a": 1 } }),
            json!({ "way": [[1]] }),
        ] {
            let Value::Object(values) = bad else {
                panic!("an object");
            };
            assert!(matches!(texts(&values), Err(ToolError::Arguments(_))));
        }
    }
}
