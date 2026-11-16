//! The session tools: start one (in a folder, a project or a worktree,
//! with a first prompt), send it input, answer its permission requests,
//! read its log; and their schemas.

use std::path::PathBuf;

use bytes::Bytes;
use rmcp::model::{JsonObject, Tool};
use serde_json::{Value, json};
use vessel_proto::{
    ContentBlock, CreateSession, Delivery, EventSeq, GetSession, HarnessRef, ModelId,
    PermissionOutcome, ReadEvents, SendInput, SessionId, SessionInput, SessionMode, SessionSpec,
    TerminalBytes, TerminalInput, TerminalSize,
};

use super::{
    READ_DEFAULT, READ_MAX, Target, ToolError, VesselTools, WAIT_MAX_SECS, conversation, flag,
    number, required, session, text, tool, workspace,
};

impl VesselTools {
    async fn check_notify_route(&self, node: Target) -> Result<(), ToolError> {
        if let Some(node) = node {
            let _ready = self
                .on(Some(node), vessel_proto::mcp::CheckPromptTracking {})
                .await?;
        }
        Ok(())
    }

    pub(super) async fn create_session(&self, args: &JsonObject) -> Result<Value, ToolError> {
        let node = self.target(args).await?;
        let terminal = flag(args, "terminal")?;
        let spec = Self::spec(args)?;
        // Decode and validate every option before creation can have a side effect.
        let prompt = text(args, "prompt")?;
        let notify = notification(
            args,
            self.own_session().is_some() && !terminal && prompt.is_some(),
        )?;
        if notify && (terminal || prompt.is_none() || self.own_session().is_none()) {
            return Err(ToolError::Arguments(
                "`notify` needs a structured first prompt and an explicit own-session context"
                    .to_owned(),
            ));
        }
        if notify {
            self.check_notify_route(node).await?;
        }
        let info = self.on(node, CreateSession { spec }).await?;
        let Some(prompt) = prompt else {
            return Ok(json!({ "session": info }));
        };
        if !terminal {
            self.until_running(node, info.id).await?;
        }
        if notify {
            let sent = self
                .deliver(node, info.id, prompt, Delivery::Queue, true)
                .await?;
            let session = self.on(node, GetSession { session: info.id }).await?;
            return Ok(json!({
                "session": session,
                "from": sent.get("from"),
                "delivered": sent.get("delivered")
            }));
        }
        let from = self.send_text(node, info.id, terminal, prompt).await?;
        let session = self.on(node, GetSession { session: info.id }).await?;
        Ok(json!({ "session": session, "from": from }))
    }

    /// The spec `create_session`'s arguments (and `create_schedule`'s new
    /// session) describe.
    pub(super) fn spec(args: &JsonObject) -> Result<SessionSpec, ToolError> {
        let harness: HarnessRef = required(args, "harness")?
            .parse()
            .map_err(|error| ToolError::Arguments(format!("{error}")))?;
        let mode = if flag(args, "terminal")? {
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            }
        } else {
            SessionMode::Structured {
                model: text(args, "model")?.map(ModelId::new),
                resume: None,
                options: Vec::new(),
            }
        };
        let mut spec = SessionSpec::new(harness, mode);
        match (workspace::place(args)?, text(args, "cwd")?) {
            (Some(_), Some(_)) => {
                return Err(ToolError::Arguments(
                    "give `cwd`, or a `project` or `worktree`, not both".to_owned(),
                ));
            }
            (Some(place), None) => spec = spec.with_place(place),
            (None, Some(cwd)) => spec = spec.with_cwd(PathBuf::from(cwd)),
            (None, None) => {}
        }
        if let Some(profile) = text(args, "profile")? {
            let profile = profile
                .parse()
                .map_err(|error| ToolError::Arguments(format!("{error}")))?;
            spec = spec.with_profile(profile);
        }
        super::launch::configure(spec, args)
    }

    pub(super) async fn send_input(&self, args: &JsonObject) -> Result<Value, ToolError> {
        let node = self.target(args).await?;
        let session = session(args)?;
        let text = required(args, "text")?;
        let terminal = matches!(
            self.on(node, GetSession { session }).await?.spec.mode,
            SessionMode::Terminal { .. }
        );
        let notify = notification(args, self.own_session().is_some() && !terminal)?;
        let delivery = conversation::delivery(args)?;
        let delivery = if delivery.is_none() && !terminal && self.own_session().is_some() {
            let current = self
                .on(
                    node,
                    vessel_proto::conversation::GetConversation { session },
                )
                .await?;
            Some(
                if current.abilities.is_some_and(|abilities| abilities.steer) {
                    Delivery::Steer
                } else {
                    Delivery::Interrupt
                },
            )
        } else {
            delivery
        };
        if delivery.is_some() || notify {
            if notify {
                self.check_notify_route(node).await?;
            }
            let delivery = delivery.unwrap_or(Delivery::Queue);
            return self.deliver(node, session, text, delivery, notify).await;
        }
        let from = self.send_text(node, session, terminal, text).await?;
        Ok(json!({ "from": from }))
    }

    /// Send `text` as a prompt, or typed into a terminal followed by Enter;
    /// returns where the session's log stood just before.
    pub(super) async fn send_text(
        &self,
        node: Target,
        session: SessionId,
        terminal: bool,
        text: &str,
    ) -> Result<EventSeq, ToolError> {
        let from = self.on(node, GetSession { session }).await?.next_seq;
        let input = if terminal {
            SessionInput::Terminal {
                input: TerminalInput::Data {
                    data: TerminalBytes::new(Bytes::from(format!("{text}\r"))),
                },
            }
        } else {
            SessionInput::Prompt {
                content: vec![ContentBlock::Text {
                    text: text.to_owned(),
                }],
            }
        };
        let _ack = self.on(node, SendInput { session, input }).await?;
        Ok(from)
    }

    pub(super) async fn answer_permission(&self, args: &JsonObject) -> Result<Value, ToolError> {
        let node = self.target(args).await?;
        let session = session(args)?;
        let id = serde_json::from_value(Value::String(required(args, "request")?.to_owned()))
            .map_err(|error| ToolError::Arguments(format!("request: {error}")))?;
        let outcome = match text(args, "option")? {
            Some(option) => PermissionOutcome::Selected {
                option: serde_json::from_value(Value::String(option.to_owned()))
                    .map_err(|error| ToolError::Arguments(format!("option: {error}")))?,
            },
            None => PermissionOutcome::Cancelled,
        };
        let input = SessionInput::AnswerPermission { id, outcome };
        let _ack = self.on(node, SendInput { session, input }).await?;
        Ok(json!({ "answered": true }))
    }

    pub(super) async fn read_events(&self, args: &JsonObject) -> Result<Value, ToolError> {
        let node = self.target(args).await?;
        let session = session(args)?;
        let from = EventSeq::new(number(args, "from")?.unwrap_or(0));
        let limit = number(args, "limit")?
            .and_then(|limit| u32::try_from(limit).ok())
            .unwrap_or(READ_DEFAULT)
            .clamp(1, READ_MAX);
        let page = self
            .on(
                node,
                ReadEvents {
                    session,
                    from,
                    limit,
                    wait: None,
                },
            )
            .await?;
        Ok(json!(page))
    }
}

pub(super) fn session_tools(node: &Value, session: &Value) -> Vec<Tool> {
    vec![
        tool(
            "list_nodes",
            "The nodes you can reach: this node and, in a fleet, the others (through the \
             fleet's home), with whether each is reachable now.",
            json!({}),
            &[],
        ),
        tool(
            "list_sessions",
            "Sessions on the node, newest first. Explicit scope/labels require lifecycle \
             support on every hop. Omission preserves the legacy full list until supported.",
            json!({ "node": node,
                "scope":{"type":"string","enum":["shown","all"],
                    "description":"Shown discovery or inclusive background discovery; independent of archival and access permissions."},
                "labels":super::discovery::labels_schema() }),
            &[],
        ),
        create_session_tool(node),
        tool(
            "send_input",
            "Send a prompt to a structured session, or type a line into a terminal one. \
             Completion notifications default on for a session agent; finish your turn to receive them. \
             Returns `from` and, with `delivery` or notifications, the prompt receipt.",
            json!({
                "node": node,
                "session": session,
                "text": { "type": "string" },
                "delivery": conversation::delivery_property(),
                "notify": notify_property()
            }),
            &["session", "text"],
        ),
        tool(
            "answer_permission",
            "Answer a session's current permission request: the chosen option's id, or none to refuse.",
            json!({
                "node": node,
                "session": session,
                "request": { "type": "string", "description": "The request's id." },
                "option": { "type": "string", "description": "The chosen option's id." }
            }),
            &["session", "request"],
        ),
        tool(
            "read_events",
            "A page of a session's log from record `from` (0 is the start).",
            json!({
                "node": node,
                "session": session,
                "from": { "type": "integer", "minimum": 0 },
                "limit": { "type": "integer", "minimum": 1, "maximum": READ_MAX }
            }),
            &["session"],
        ),
        tool(
            "wait_turn",
            "Wait, from record `from` (what `send_input` or `create_session` returned, or a \
             previous `next`), until the session's turn ends, it asks for a permission or asks a \
             question, or it ends. Returns the outcome, the reply text, and `next` to go on \
             from.",
            json!({
                "node": node,
                "session": session,
                "from": { "type": "integer", "minimum": 0 },
                "timeout_secs": { "type": "integer", "minimum": 1, "maximum": WAIT_MAX_SECS }
            }),
            &["session", "from"],
        ),
        tool(
            "kill_session",
            "Stop a session you started (or one those started).",
            json!({ "node": node, "session": session }),
            &["session"],
        ),
    ]
}

/// `notify` on `send_input` and `create_session`'s first prompt.
fn notify_property() -> Value {
    json!({
        "type": "boolean",
        "description": "Defaults true for a structured session agent. Your own session receives \
                        a queued notice when this prompt's exact turn needs an answer, finishes, \
                        or its session ends, including across supported fleet routes. Finish \
                        your turn to await that notice. Set false for explicit reporting."
    })
}

pub(super) fn create_session_tool(node: &Value) -> Tool {
    tool(
        "create_session",
        "Start a session; it is yours to drive and stop. Structured (a conversation) unless \
         `terminal` is true. With `prompt`, it is sent once the session is ready. Structured \
         session agents receive completion notifications by default; finish your turn to await them.",
        json!({
            "node": node,
            "harness": {
                "type": "string",
                "description": "claude, codex, gemini, opencode, or an agent added on the node."
            },
            "profile": {
                "type": "string",
                "description": "One of the node's profiles for that harness (an account)."
            },
            "cwd": {
                "type": "string",
                "description": "Absolute working directory on the node (or give `project` \
                                or `worktree`)."
            },
            "project": {
                "type": "string",
                "description": "Run in this project's folder (an id from `list_projects`)."
            },
            "dir": {
                "type": "string",
                "description": "With `project`: a folder inside it, relative."
            },
            "worktree": {
                "type": "string",
                "description": "Run in this worktree (from `create_worktree`): one you made, \
                                or one made for a session of yours."
            },
            "title": { "type": "string", "description": "Initial session title." },
            "labels": { "type": "object", "additionalProperties": {"type":"string"}, "description": "Validated session labels; requires lifecycle support on every hop." },
            "options": super::launch::options_schema(),
            "model": { "type": "string", "description": "A model the agent offers." },
            "prompt": { "type": "string", "description": "The first prompt." },
            "notify": notify_property(),
            "terminal": {
                "type": "boolean",
                "description": "Run the harness's own terminal UI instead."
            }
        }),
        &["harness"],
    )
}

fn notification(args: &JsonObject, default: bool) -> Result<bool, ToolError> {
    match args.get("notify") {
        None | Some(Value::Null) => Ok(default),
        Some(_) => flag(args, "notify"),
    }
}
