//! `fork_session` and `rewind_session` (protocol 14, ADR 0052): an agent
//! forks a session's conversation from a turn's end, or takes a session it
//! may operate back to one, as a person does with `vessel session fork`
//! and `rewind`.

use rmcp::model::{JsonObject, Tool};
use serde_json::{Value, json};
use vessel_proto::EventSeq;
use vessel_proto::forking::{ForkSession, RewindSession};

use super::{ToolError, VesselTools, number, required, tool};

impl VesselTools {
    pub(super) async fn forking(&self, name: &str, args: &JsonObject) -> Result<Value, ToolError> {
        let node = self.target(args).await?;
        let session = required(args, "session")?
            .parse()
            .map_err(|error| ToolError::Arguments(format!("session: {error}")))?;
        let after = number(args, "after")?.map(EventSeq::new);
        if name == "rewind_session" {
            let after =
                after.ok_or_else(|| ToolError::Arguments("`after` is required".to_owned()))?;
            Ok(json!(
                self.on(node, RewindSession { session, after }).await?
            ))
        } else {
            Ok(json!(self.on(node, ForkSession { session, after }).await?))
        }
    }
}

pub(super) fn tools(node: &Value, session: &Value) -> Vec<Tool> {
    let after = |what: &str| {
        json!({
            "type": "integer",
            "description": format!(
                "The `seq` of a `turn_ended` record of the session (`read_events`): {what}."
            )
        })
    };
    vec![
        tool(
            "fork_session",
            "Start a new session that continues a session's conversation from the end of a \
             turn (`after`), or as it is now (no `after`, not while a turn runs). Only with an \
             agent that can fork; the new session runs in the same folder. Returns it.",
            json!({
                "session": session,
                "after": after("the turn to fork after"),
                "node": node
            }),
            &["session"],
        ),
        tool(
            "rewind_session",
            "Take a session you may operate back to the end of a turn, between turns: its \
             agent continues from there; what came after stays in its log; files are not put \
             back. Returns the session.",
            json!({
                "session": session,
                "after": after("the turn to go back to"),
                "node": node
            }),
            &["session", "after"],
        ),
    ]
}
