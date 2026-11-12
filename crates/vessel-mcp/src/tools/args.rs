//! Reading a tool's JSON arguments: each a typed value or an
//! `arguments` error naming the key.

use rmcp::model::JsonObject;
use serde_json::Value;
use vessel_proto::SessionId;

use super::ToolError;

pub(super) fn text<'a>(args: &'a JsonObject, key: &str) -> Result<Option<&'a str>, ToolError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text)),
        Some(_) => Err(ToolError::Arguments(format!("`{key}` must be a string"))),
    }
}

pub(super) fn required<'a>(args: &'a JsonObject, key: &str) -> Result<&'a str, ToolError> {
    text(args, key)?.ok_or_else(|| ToolError::Arguments(format!("`{key}` is required")))
}

pub(super) fn number(args: &JsonObject, key: &str) -> Result<Option<u64>, ToolError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| ToolError::Arguments(format!("`{key}` must be a non-negative integer"))),
    }
}

pub(super) fn flag(args: &JsonObject, key: &str) -> Result<bool, ToolError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(flag)) => Ok(*flag),
        Some(_) => Err(ToolError::Arguments(format!(
            "`{key}` must be true or false"
        ))),
    }
}

pub(super) fn session(args: &JsonObject) -> Result<SessionId, ToolError> {
    required(args, "session")?
        .parse()
        .map_err(|error| ToolError::Arguments(format!("session: {error}")))
}
