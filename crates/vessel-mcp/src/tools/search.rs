//! `search_sessions` (protocol 12, ADR 0037): an agent finds what was said
//! in any session of a node, as a person does with `vessel search`. A read:
//! an agent reads the fleet's sessions (ADR 0013), so it may search them.

use rmcp::model::{JsonObject, Tool};
use serde_json::{Value, json};
use vessel_proto::search::{SearchCursor, SearchLimit, SearchMessages};

use super::{ToolError, VesselTools, number, required, text, tool};

impl VesselTools {
    pub(super) async fn search(&self, args: &JsonObject) -> Result<Value, ToolError> {
        let node = self.target(args).await?;
        let request = request(args)?;
        Ok(json!(self.on(node, request).await?))
    }
}

/// The request from the tool's arguments.
fn request(args: &JsonObject) -> Result<SearchMessages, ToolError> {
    let query = required(args, "query")?
        .parse()
        .map_err(|error| ToolError::Arguments(format!("query: {error}")))?;
    let session = text(args, "session")?
        .map(|session| {
            session
                .parse()
                .map_err(|error| ToolError::Arguments(format!("session: {error}")))
        })
        .transpose()?;
    let before = text(args, "before")?
        .map(|cursor| {
            cursor
                .parse::<SearchCursor>()
                .map_err(|error| ToolError::Arguments(format!("before: {error}")))
        })
        .transpose()?;
    let limit = number(args, "limit")?
        .map(|limit| {
            u8::try_from(limit)
                .ok()
                .and_then(SearchLimit::new)
                .ok_or_else(|| ToolError::Arguments("`limit` is 1 to 100".to_owned()))
        })
        .transpose()?;
    Ok(SearchMessages {
        query,
        session,
        before,
        limit,
    })
}

pub(super) fn tools(node: &Value, session: &Value) -> Vec<Tool> {
    vec![tool(
        "search_sessions",
        "Search what was said in a node's sessions (prompts and the agents' replies, not tool \
         output), newest first. Every word must be in a message, in any order, inside other \
         words, without case. Each hit names its session and the record `seq` where the \
         message starts (`read_events` from there). A page may hold fewer hits than asked and \
         still give `next`: pass it as `before` to go on.",
        json!({
            "query": { "type": "string", "description": "The words, 1 to 256 characters." },
            "session": session,
            "limit": { "type": "integer", "description": "Hits a page, 1 to 100 (20)." },
            "before": { "type": "string", "description": "An earlier page's `next`." },
            "node": node
        }),
        &["query"],
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(value: &Value) -> JsonObject {
        value.as_object().cloned().unwrap()
    }

    #[test]
    fn arguments_become_a_bounded_request() {
        let made = request(&args(&json!({
            "query": " parser 오류 ",
            "limit": 5,
            "before": "1700000000000000000"
        })))
        .unwrap();
        assert_eq!(made.query.as_str(), "parser 오류");
        assert_eq!(made.limit.map(SearchLimit::get), Some(5));
        assert_eq!(
            made.before,
            Some(SearchCursor::new(1_700_000_000_000_000_000))
        );
        assert!(request(&args(&json!({ "query": "x", "limit": 0 }))).is_err());
        assert!(request(&args(&json!({ "query": "   " }))).is_err());
        assert!(request(&args(&json!({}))).is_err());
    }
}
