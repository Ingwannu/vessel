//! Discovery selectors use the landed lifecycle request, never client filtering.
//!
//! Omission preserves old full-list behavior while lifecycle is unassigned.
//! Explicit selectors stay present through shared admission on every hop; an
//! unsupported peer must not turn a requested predicate into an unfiltered read.
use rmcp::model::JsonObject;
use serde_json::{Value, json};
use vessel_proto::{
    LabelKey, LabelValue, ListSessions, ListingScope, SessionLabels, SessionSelection,
};

use super::{ToolError, VesselTools};

fn request(args: &JsonObject, supports_discovery: bool) -> Result<ListSessions, ToolError> {
    let explicit = args.contains_key("scope") || args.contains_key("labels");
    if !explicit && !supports_discovery {
        return Ok(ListSessions::default());
    }
    let scope = match args.get("scope") {
        None => ListingScope::Shown,
        Some(Value::String(scope)) if scope == "shown" => ListingScope::Shown,
        Some(Value::String(scope)) if scope == "all" => ListingScope::All,
        Some(_) => return Err(ToolError::Arguments("invalid discovery scope".into())),
    };
    let labels = match args.get("labels") {
        None => SessionLabels::default(),
        Some(value) => labels(value)?,
    };
    Ok(ListSessions::selected(SessionSelection { scope, labels }))
}

fn labels(value: &Value) -> Result<SessionLabels, ToolError> {
    // Reject aggregate/type excess before copying a JSON tree. The domain
    // constructors remain the sole key/value rules and canonicalize keys.
    let fields = value
        .as_object()
        .filter(|fields| fields.len() <= SessionLabels::MAX)
        .ok_or_else(|| ToolError::Arguments("invalid discovery label map".into()))?;
    let mut labels = SessionLabels::new();
    for (key, value) in fields {
        let value = value
            .as_str()
            .ok_or_else(|| ToolError::Arguments("discovery labels must be strings".into()))?;
        let key: LabelKey = key
            .parse()
            .map_err(|_invalid| ToolError::Arguments("invalid discovery label key".into()))?;
        let value: LabelValue = value
            .parse()
            .map_err(|_invalid| ToolError::Arguments("invalid discovery label value".into()))?;
        labels.insert(key, value).map_err(|_invalid| {
            ToolError::Arguments("discovery label map exceeds its limit".into())
        })?;
    }
    Ok(labels)
}

impl VesselTools {
    pub(super) async fn list_sessions(&self, args: &JsonObject) -> Result<Value, ToolError> {
        let supported = vessel_proto::lifecycle_version::SINCE
            .is_some_and(|since| self.client.protocol() >= since);
        // Decode before even destination-name resolution performs node work.
        let request = request(args, supported)?;
        let node = self.target(args).await?;
        Ok(json!(self.on(node, request).await?))
    }
}

pub(super) fn labels_schema() -> Value {
    json!({"type":"object","maxProperties":SessionLabels::MAX,
        "additionalProperties":{"type":"string","maxLength":LabelValue::MAX_CHARS},
        "description":format!("Every validated label pair must match. Keys are trimmed and limited to {} Unicode scalars. Requires lifecycle support on every hop; an explicit predicate never falls back to a full list.", LabelKey::MAX_CHARS)})
}

#[cfg(test)]
mod tests;
