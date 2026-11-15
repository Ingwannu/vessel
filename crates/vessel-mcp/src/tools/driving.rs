//! A session's record, a cancel of its turn, its title, and one of its
//! options. Not starting or stopping a session, and not its queue.
//!
//! The node decides who may operate, including that an agent does not
//! own its own session. These calls do not refuse that case themselves.
//!
//! `tools` builds MCP's JSON Schema, a tree. Each call returns a protocol
//! type (or [`Cancelled`], which names the session an ack does not), and
//! the server encodes it.

use rmcp::model::{JsonObject, Tool};
use serde_json::{Value, json};
use vessel_proto::{
    ChoiceValue, EventSeq, GetSession, OptionChoice, OptionId, OptionValueId, RenameSession,
    SendInput, SessionId, SessionInfo, SessionInput, SessionOption, SessionTitle, SetSessionOption,
};

use super::{ToolError, VesselTools, required, session, text, tool};

impl VesselTools {
    pub(super) async fn get_session(&self, args: &JsonObject) -> Result<SessionDetails, ToolError> {
        let node = self.target(args).await?;
        let info = self
            .on(
                node,
                GetSession {
                    session: session(args)?,
                },
            )
            .await?;
        let permissions = match info.spec.mode {
            vessel_proto::SessionMode::Structured { .. } => Some(
                self.on(
                    node,
                    vessel_proto::conversation::GetConversation { session: info.id },
                )
                .await?
                .permissions,
            ),
            vessel_proto::SessionMode::Terminal { .. } => None,
        };
        if permissions
            .as_ref()
            .is_some_and(|items| items.len() > vessel_proto::mcp::MAX_PENDING_PERMISSIONS)
        {
            return Err(ToolError::PermissionInventory);
        }
        Ok(SessionDetails { info, permissions })
    }

    pub(super) async fn cancel_turn(&self, args: &JsonObject) -> Result<Cancelled, ToolError> {
        let node = self.target(args).await?;
        let session = session(args)?;
        let turn = super::number(args, "turn")?.map(EventSeq::new);
        let _ack = self
            .on(
                node,
                SendInput {
                    session,
                    input: SessionInput::CancelTurn { turn },
                },
            )
            .await?;
        Ok(Cancelled {
            cancelled: session,
            turn,
        })
    }

    pub(super) async fn rename_session(&self, args: &JsonObject) -> Result<SessionInfo, ToolError> {
        let node = self.target(args).await?;
        let session = session(args)?;
        let title: SessionTitle = required(args, "title")?
            .parse()
            .map_err(|error| ToolError::Arguments(format!("title: {error}")))?;
        self.on(node, RenameSession { session, title }).await
    }

    pub(super) async fn set_session_option(
        &self,
        args: &JsonObject,
    ) -> Result<Vec<SessionOption>, ToolError> {
        let node = self.target(args).await?;
        let session = session(args)?;
        self.on(
            node,
            SetSessionOption {
                session,
                choice: choice(args)?,
            },
        )
        .await
    }
}

/// A sequential current-permission read beside ordinary session metadata.
#[derive(Debug, serde::Serialize)]
pub(super) struct SessionDetails {
    #[serde(flatten)]
    info: SessionInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    permissions: Option<Vec<vessel_proto::PermissionRequest>>,
}

/// `session.send` of a cancel answers an ack. The tool names the session.
/// The server encodes this; the fields stay here so the call does not
/// return a JSON tree.
#[derive(Debug)]
pub(super) struct Cancelled {
    /// The session whose turn was cancelled.
    pub(super) cancelled: SessionId,
    /// The turn, when the caller named one.
    pub(super) turn: Option<EventSeq>,
}

/// One option change: a select value or a toggle, not a mix.
fn choice(args: &JsonObject) -> Result<OptionChoice, ToolError> {
    let id = required(args, "option")?;
    if id.is_empty() {
        return Err(ToolError::Arguments("`option` is required".into()));
    }
    let value = match (text(args, "value")?, toggle(args)?) {
        (None, None) => {
            return Err(ToolError::Arguments(
                "give `value` (a select option) or `on` (a toggle), not neither".into(),
            ));
        }
        (Some(_), Some(_)) => {
            return Err(ToolError::Arguments(
                "give `value` or `on`, not both".into(),
            ));
        }
        (Some(""), None) => {
            return Err(ToolError::Arguments("`value` is empty".into()));
        }
        (Some(value), None) => ChoiceValue::Select {
            value: OptionValueId::new(value),
        },
        (None, Some(on)) => ChoiceValue::Toggle { on },
    };
    Ok(OptionChoice {
        id: OptionId::new(id),
        value,
    })
}

fn toggle(args: &JsonObject) -> Result<Option<bool>, ToolError> {
    match args.get("on") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(on)) => Ok(Some(*on)),
        Some(_) => Err(ToolError::Arguments("`on` must be true or false".into())),
    }
}

pub(super) fn tools(node: &Value, session: &Value) -> Vec<Tool> {
    vec![
        tool(
            "get_session",
            "One session: its id, harness, mode, status, profile, creator and current structured permissions. \
             Reading is not operating; this does not change it.",
            json!({ "node": node, "session": session }),
            &["session"],
        ),
        tool(
            "cancel_turn",
            "Cancel the turn a structured session you own is in. Not your own session. \
             With `turn`, that turn's `turn_started` record; without it, whatever turn \
             is running.",
            json!({
                "node": node,
                "session": session,
                "turn": { "type": "integer", "minimum": 0, "description": "A record seq." }
            }),
            &["session"],
        ),
        tool(
            "rename_session",
            "Set the title of a session you own. Not your own session. The title must satisfy the protocol limits.",
            json!({
                "node": node,
                "session": session,
                "title": { "type": "string" }
            }),
            &["session", "title"],
        ),
        tool(
            "set_session_option",
            "Change one option of a structured session you own (a mode is one option). \
             Not your own session. Give `value` for a select option or `on` for a toggle, \
             not both: the agent refuses a choice it does not offer.",
            json!({
                "node": node,
                "session": session,
                "option": { "type": "string", "description": "The option's id." },
                "value": { "type": "string", "description": "A select option's value." },
                "on": { "type": "boolean", "description": "A toggle: on or off." }
            }),
            &["session", "option"],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn args(value: &Value) -> JsonObject {
        value.as_object().cloned().unwrap()
    }

    #[test]
    fn an_option_is_a_select_or_a_toggle_not_both() {
        let select = choice(&args(&json!({ "option": "mode", "value": "plan" }))).unwrap();
        assert!(matches!(select.value, ChoiceValue::Select { .. }));
        let off = choice(&args(&json!({ "option": "fast", "on": false }))).unwrap();
        assert_eq!(off.value, ChoiceValue::Toggle { on: false });
        for wrong in [
            json!({ "option": "mode" }),
            json!({ "option": "mode", "value": "plan", "on": true }),
            json!({ "option": "", "value": "plan" }),
            json!({ "option": "mode", "value": "" }),
            json!({ "option": "mode", "on": "yes" }),
        ] {
            assert!(choice(&args(&wrong)).is_err(), "{wrong}");
        }
    }
}
